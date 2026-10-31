use cownter_core::model::*;
use cownter_core::scheduler::{Scheduler, SchedulerOptions};
use cownter_core::secrets::{MemorySecrets, SecretStore};
use cownter_core::sources::Fetcher;
use cownter_core::store::Store;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn entry(slug: &str, url: String) -> Entry {
    Entry {
        id: 0,
        slug: slug.into(),
        name: slug.into(),
        icon: Icon::default(),
        group_id: None,
        source: SourceCfg::CustomUrl { url, method: "GET".into(), headers: vec![], body: None, pick: Pick::FirstNumber },
        interval_s: 10,
        format: NumberFormat::default(),
        goal_value: None,
        celebrate: true,
        sounds: Sounds::default(),
        sort: 0,
        text_mode: false,
        goal_text: None,
    }
}

fn setup() -> (Arc<Store>, Arc<Fetcher>, mpsc::UnboundedSender<CoreEvent>, mpsc::UnboundedReceiver<CoreEvent>) {
    let store = Arc::new(Store::open_memory().unwrap());
    let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecrets::new());
    let fetcher = Arc::new(Fetcher::new(secrets));
    let (tx, rx) = mpsc::unbounded_channel();
    (store, fetcher, tx, rx)
}

fn fast() -> SchedulerOptions {
    SchedulerOptions { delay_override: Some(Duration::from_millis(50)), start_spread: None }
}

struct Concurrency {
    now: Arc<AtomicUsize>,
    max: Arc<AtomicUsize>,
}

impl Respond for Concurrency {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        let n = self.now.fetch_add(1, Ordering::SeqCst) + 1;
        self.max.fetch_max(n, Ordering::SeqCst);
        let now = self.now.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(290));
            now.fetch_sub(1, Ordering::SeqCst);
        });
        ResponseTemplate::new(200).set_body_string("5").set_delay(Duration::from_millis(300))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn no_overlap_when_slow() {
    let server = MockServer::start().await;
    let max = Arc::new(AtomicUsize::new(0));
    Mock::given(wiremock::matchers::any())
        .respond_with(Concurrency { now: Arc::new(AtomicUsize::new(0)), max: max.clone() })
        .mount(&server)
        .await;
    let (store, fetcher, tx, _rx) = setup();
    store.insert_entry(&entry("slow", server.uri()), 0).unwrap();
    let _s = Scheduler::start_with(store.clone(), fetcher, tx, fast());
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert_eq!(max.load(Ordering::SeqCst), 1);
    assert!(server.received_requests().await.unwrap().len() >= 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn live_value_flows_to_events() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any()).respond_with(ResponseTemplate::new(200).set_body_string("42")).mount(&server).await;
    let (store, fetcher, tx, mut rx) = setup();
    let id = store.insert_entry(&entry("a", server.uri()), 0).unwrap();
    let _s = Scheduler::start_with(store.clone(), fetcher, tx, fast());
    let ev = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(CoreEvent::EntryUpdated { view }) = rx.recv().await {
                if view.entry.id == id && view.value == Some(42.0) {
                    return view;
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(ev.status, Status::Live);
}

#[tokio::test(flavor = "multi_thread")]
async fn auth_failure_pauses_until_reload() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any()).respond_with(ResponseTemplate::new(401)).mount(&server).await;
    let (store, fetcher, tx, mut rx) = setup();
    let id = store.insert_entry(&entry("a", server.uri()), 0).unwrap();
    let s = Scheduler::start_with(store.clone(), fetcher, tx, fast());
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let mut auth = 0;
    while let Ok(e) = rx.try_recv() {
        if matches!(e, CoreEvent::AuthFailed { .. }) {
            auth += 1;
        }
    }
    assert_eq!(auth, 1);
    assert_eq!(store.latest(id).unwrap().unwrap().status, Status::NoLink);
    s.reload(id);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_discards_inflight() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(200).set_body_string("1").set_delay(Duration::from_millis(400)))
        .mount(&server)
        .await;
    let (store, fetcher, tx, _rx) = setup();
    let id = store.insert_entry(&entry("a", server.uri()), 0).unwrap();
    let s = Scheduler::start_with(store.clone(), fetcher, tx, fast());
    tokio::time::sleep(Duration::from_millis(150)).await;
    s.remove(id);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(store.latest(id).unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn simulate_writes_and_emits_change() {
    let (store, fetcher, tx, mut rx) = setup();
    let mut e = entry("a", "http://127.0.0.1:9".into());
    e.source = SourceCfg::Formula { expr: "1".into() };
    let id = store.insert_entry(&e, 0).unwrap();
    let s = Scheduler::start_with(store.clone(), fetcher, tx, fast());
    s.simulate(id, 10.0).unwrap();
    s.simulate(id, 11.0).unwrap();
    let mut changed = false;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, CoreEvent::ValueChanged { value, prev, .. } if value == 11.0 && prev == 10.0) {
            changed = true;
        }
    }
    assert!(changed);
    assert!(s.simulate(9999, 1.0).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn reload_works_from_a_thread_without_runtime() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any()).respond_with(ResponseTemplate::new(200).set_body_string("7")).mount(&server).await;
    let (store, fetcher, tx, mut rx) = setup();
    let s = Scheduler::start_with(store.clone(), fetcher, tx, fast());
    let id = store.insert_entry(&entry("late", server.uri()), 0).unwrap();
    let s2 = s.clone();
    std::thread::spawn(move || {
        s2.reload(id);
        s2.refresh_all();
    })
    .join()
    .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(CoreEvent::EntryUpdated { view }) = rx.recv().await {
                if view.entry.id == id && view.value == Some(7.0) {
                    return;
                }
            }
        }
    })
    .await
    .unwrap();
}
