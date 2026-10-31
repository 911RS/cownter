use crate::backoff::{Outcome, PollState, jitter, next_delay};
use crate::model::*;
use crate::rules;
use crate::sources::{FetchError, Fetcher};
use crate::store::{HOUR_MS, Store};
use chrono::Local;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{Semaphore, mpsc::UnboundedSender};
use tokio::task::JoinHandle;

const MAX_PARALLEL_FETCHES: usize = 6;

#[derive(Debug, Clone, Default)]
pub struct SchedulerOptions {
    /// Replaces every computed delay; used by tests.
    pub delay_override: Option<Duration>,
    /// Upper bound of the random delay before an entry's first poll.
    pub start_spread: Option<Duration>,
}

struct Inner {
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    tx: UnboundedSender<CoreEvent>,
    tasks: Mutex<HashMap<i64, JoinHandle<()>>>,
    rt: tokio::runtime::Handle,
    permits: Arc<Semaphore>,
    opts: SchedulerOptions,
}

#[derive(Clone)]
pub struct Scheduler {
    inner: Arc<Inner>,
    _maintenance: Arc<MaintenanceGuard>,
}

struct MaintenanceGuard(JoinHandle<()>);

impl Drop for MaintenanceGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn day_start_ms() -> i64 {
    let now = Local::now();
    let naive = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    use chrono::TimeZone;
    Local.from_local_datetime(&naive).earliest().map(|d| d.timestamp_millis()).unwrap_or(now.timestamp_millis())
}

enum Fetched {
    Number(f64),
    Text(String),
}

fn reading_for(entry_id: i64, ts: i64, res: &Result<Fetched, FetchError>) -> Reading {
    match res {
        Ok(Fetched::Number(v)) => Reading { entry_id, ts, value: Some(*v), status: Status::Live, error: None, text: None },
        Ok(Fetched::Text(t)) => Reading { entry_id, ts, value: None, status: Status::Live, error: None, text: Some(t.clone()) },
        Err(e) => {
            let status = match e {
                FetchError::Auth(_) => Status::NoLink,
                FetchError::Network(_) | FetchError::Timeout | FetchError::RateLimited { .. } => Status::Stale,
                FetchError::Http(c) if *c >= 500 || *c == 408 => Status::Stale,
                _ => Status::Error,
            };
            Reading { entry_id, ts, value: None, status, error: Some(e.to_string()), text: None }
        }
    }
}

impl Scheduler {
    pub fn start(store: Arc<Store>, fetcher: Arc<Fetcher>, tx: UnboundedSender<CoreEvent>) -> Scheduler {
        Self::start_with(store, fetcher, tx, SchedulerOptions { start_spread: Some(Duration::from_secs(3)), ..Default::default() })
    }

    pub fn start_with(
        store: Arc<Store>,
        fetcher: Arc<Fetcher>,
        tx: UnboundedSender<CoreEvent>,
        opts: SchedulerOptions,
    ) -> Scheduler {
        let rt = tokio::runtime::Handle::current();
        let maint_store = store.clone();
        let maintenance = rt.spawn(async move {
            loop {
                let s = maint_store.clone();
                let _ = tokio::task::spawn_blocking(move || s.compact(now_ms())).await;
                tokio::time::sleep(Duration::from_millis(6 * HOUR_MS as u64)).await;
            }
        });
        let inner = Arc::new(Inner {
            store,
            fetcher,
            tx,
            tasks: Mutex::new(HashMap::new()),
            rt,
            permits: Arc::new(Semaphore::new(MAX_PARALLEL_FETCHES)),
            opts,
        });
        let s = Scheduler { inner, _maintenance: Arc::new(MaintenanceGuard(maintenance)) };
        s.refresh_all();
        s
    }

    fn send(&self, evs: Vec<CoreEvent>) {
        for e in evs {
            let _ = self.inner.tx.send(e);
        }
    }

    /// Restarts polling for one entry (after it was created or edited); clears a paused state.
    pub fn reload(&self, entry_id: i64) {
        self.stop(entry_id);
        let Ok(Some(entry)) = self.inner.store.entry(entry_id) else { return };
        if entry.source.is_formula() {
            if let Ok(evs) = rules::recompute_formula(&self.inner.store, &entry, now_ms(), day_start_ms()) {
                self.send(evs);
            }
            return;
        }
        let me = self.clone();
        let handle = self.inner.rt.spawn(async move { me.poll_loop(entry_id, false).await });
        self.inner.tasks.lock().unwrap().insert(entry_id, handle);
    }

    fn stop(&self, entry_id: i64) {
        if let Some(h) = self.inner.tasks.lock().unwrap().remove(&entry_id) {
            h.abort();
        }
    }

    pub fn remove(&self, entry_id: i64) {
        self.stop(entry_id);
        self.send(vec![CoreEvent::EntryRemoved { entry_id }]);
    }

    /// Polls every entry now and recomputes formulas.
    pub fn refresh_all(&self) {
        let ids: Vec<i64> = self.inner.tasks.lock().unwrap().keys().copied().collect();
        for id in ids {
            self.stop(id);
        }
        let Ok(entries) = self.inner.store.entries() else { return };
        let first = self.inner.tasks.lock().unwrap().is_empty();
        for e in &entries {
            if e.source.is_formula() {
                continue;
            }
            let me = self.clone();
            let id = e.id;
            let handle = self.inner.rt.spawn(async move { me.poll_loop(id, first).await });
            self.inner.tasks.lock().unwrap().insert(id, handle);
        }
        for e in entries.iter().filter(|e| e.source.is_formula()) {
            if let Ok(evs) = rules::recompute_formula(&self.inner.store, e, now_ms(), day_start_ms()) {
                self.send(evs);
            }
        }
    }

    /// Records a value without any network call, as if the source had returned it.
    pub fn simulate(&self, entry_id: i64, value: f64) -> Result<(), String> {
        let store = &self.inner.store;
        if store.entry(entry_id).map_err(|e| e.to_string())?.is_none() {
            return Err("entry not found".into());
        }
        let ts = now_ms();
        store.insert_reading(&Reading { entry_id, ts, value: Some(value), status: Status::Live, error: None, text: None })
            .map_err(|e| e.to_string())?;
        let evs = rules::after_reading(store, entry_id, ts, day_start_ms()).map_err(|e| e.to_string())?;
        self.send(evs);
        Ok(())
    }

    async fn poll_loop(self, entry_id: i64, spread: bool) {
        if spread {
            if let Some(max) = self.inner.opts.start_spread {
                let d = max.mul_f64(rand::random::<f64>());
                tokio::time::sleep(d).await;
            }
        }
        let mut state = PollState::default();
        loop {
            let entry = match self.inner.store.entry(entry_id) {
                Ok(Some(e)) => e,
                _ => return,
            };
            let res = {
                let _permit = self.inner.permits.acquire().await;
                if entry.text_mode {
                    self.inner.fetcher.fetch_text(&entry.source).await.map(Fetched::Text)
                } else {
                    self.inner.fetcher.fetch(&entry.source, Local::now()).await.map(Fetched::Number)
                }
            };
            let ts = now_ms();
            let reading = reading_for(entry_id, ts, &res);
            let store = self.inner.store.clone();
            let evs = tokio::task::spawn_blocking(move || -> Result<Vec<CoreEvent>, String> {
                if store.entry(entry_id).map_err(|e| e.to_string())?.is_none() {
                    return Ok(vec![]);
                }
                store.insert_reading(&reading).map_err(|e| e.to_string())?;
                rules::after_reading(&store, entry_id, ts, day_start_ms()).map_err(|e| e.to_string())
            })
            .await;
            if let Ok(Ok(evs)) = evs {
                self.send(evs);
            }
            if let Err(FetchError::Auth(_)) = &res {
                self.send(vec![CoreEvent::AuthFailed {
                    entry_id,
                    message: format!("{}: the service rejected the key or token", entry.name),
                }]);
            }
            let outcome = match &res {
                Ok(_) => Outcome::Ok,
                Err(e) => Outcome::Err(e),
            };
            let Some(delay) = next_delay(&mut state, entry.interval_s, outcome) else { return };
            let delay = match self.inner.opts.delay_override {
                Some(d) => d,
                None => jitter(delay, &mut rand::rng()),
            };
            tokio::time::sleep(delay).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping() {
        assert_eq!(reading_for(1, 0, &Ok(Fetched::Number(3.0))).status, Status::Live);
        let t = reading_for(1, 0, &Ok(Fetched::Text("open".into())));
        assert_eq!((t.status, t.value, t.text.as_deref()), (Status::Live, None, Some("open")));
        assert_eq!(reading_for(1, 0, &Err(FetchError::Timeout)).status, Status::Stale);
        assert_eq!(reading_for(1, 0, &Err(FetchError::Http(503))).status, Status::Stale);
        assert_eq!(reading_for(1, 0, &Err(FetchError::Http(404))).status, Status::Error);
        assert_eq!(reading_for(1, 0, &Err(FetchError::Auth(401))).status, Status::NoLink);
        assert_eq!(reading_for(1, 0, &Err(FetchError::Parse("x".into()))).status, Status::Error);
        assert_eq!(reading_for(1, 0, &Err(FetchError::RateLimited { retry_after_s: None })).status, Status::Stale);
    }
}
