use chrono::Local;
use cownter_core::model::*;
use cownter_core::secrets::{MemorySecrets, SecretStore};
use cownter_core::sources::{BaseUrls, FetchError, Fetcher};
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fetcher(secrets: &[(&str, &str)], server: &MockServer) -> Fetcher {
    let s: Arc<dyn SecretStore> = Arc::new(MemorySecrets::with(secrets));
    Fetcher::new(s).with_base_urls(BaseUrls {
        youtube: Some(server.uri()),
        github: Some(server.uri()),
        stripe: Some(server.uri()),
        shopify: Some(server.uri()),
    })
}

fn custom(url: String, headers: Vec<Header>, pick: Pick) -> SourceCfg {
    SourceCfg::CustomUrl { url, method: "GET".into(), headers, body: None, pick }
}

async fn get(f: &Fetcher, cfg: &SourceCfg) -> Result<f64, FetchError> {
    f.fetch(cfg, Local::now()).await
}

#[tokio::test]
async fn custom_url_json_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/stats"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{"subs":"48,213"}})))
        .mount(&server)
        .await;
    let f = fetcher(&[], &server);
    let cfg = custom(format!("{}/stats", server.uri()), vec![], Pick::JsonPath("data.subs".into()));
    assert_eq!(get(&f, &cfg).await.unwrap(), 48213.0);
}

#[tokio::test]
async fn custom_url_html_login_page_is_parse_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("<html>Sign in 2024</html>", "text/html"))
        .mount(&server)
        .await;
    let f = fetcher(&[], &server);
    let cfg = custom(server.uri(), vec![], Pick::JsonPath("data.subs".into()));
    assert_eq!(get(&f, &cfg).await.unwrap_err(), FetchError::Parse("response is not JSON".into()));
}

#[tokio::test]
async fn secret_header_sent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("Authorization", "Bearer x"))
        .and(header("X-Plain", "p"))
        .respond_with(ResponseTemplate::new(200).set_body_string("37"))
        .mount(&server)
        .await;
    let f = fetcher(&[("h1", "Bearer x")], &server);
    let cfg = custom(
        server.uri(),
        vec![
            Header { name: "Authorization".into(), value: SecretOrText::Secret("h1".into()) },
            Header { name: "X-Plain".into(), value: SecretOrText::Text("p".into()) },
        ],
        Pick::FirstNumber,
    );
    assert_eq!(get(&f, &cfg).await.unwrap(), 37.0);
}

#[tokio::test]
async fn post_body_sent() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::body_string("{\"q\":1}"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"n": 9})))
        .mount(&server)
        .await;
    let f = fetcher(&[], &server);
    let cfg = SourceCfg::CustomUrl {
        url: server.uri(),
        method: "post".into(),
        headers: vec![],
        body: Some("{\"q\":1}".into()),
        pick: Pick::JsonPath("n".into()),
    };
    assert_eq!(get(&f, &cfg).await.unwrap(), 9.0);
}

#[tokio::test]
async fn missing_secret() {
    let server = MockServer::start().await;
    let f = fetcher(&[], &server);
    let cfg = custom(
        server.uri(),
        vec![Header { name: "Authorization".into(), value: SecretOrText::Secret("nope".into()) }],
        Pick::FirstNumber,
    );
    assert!(matches!(get(&f, &cfg).await.unwrap_err(), FetchError::Secret(_)));
}

#[tokio::test]
async fn bad_config_is_reported() {
    let server = MockServer::start().await;
    let f = fetcher(&[], &server);
    for url in ["not a url", "ftp://example.com/x", "file:///etc/passwd"] {
        let cfg = custom(url.into(), vec![], Pick::FirstNumber);
        assert!(matches!(get(&f, &cfg).await.unwrap_err(), FetchError::Config(_)), "{url}");
    }
    let cfg = SourceCfg::CustomUrl { url: server.uri(), method: "DELETE".into(), headers: vec![], body: None, pick: Pick::FirstNumber };
    assert!(matches!(get(&f, &cfg).await.unwrap_err(), FetchError::Config(_)));
    let cfg = custom(server.uri(), vec![Header { name: "bad header".into(), value: SecretOrText::Text("x".into()) }], Pick::FirstNumber);
    assert!(matches!(get(&f, &cfg).await.unwrap_err(), FetchError::Config(_)));
}

#[tokio::test]
async fn maps_statuses() {
    let server = MockServer::start().await;
    for (p, code) in [("/401", 401), ("/403", 403), ("/500", 500), ("/404", 404)] {
        Mock::given(path(p)).respond_with(ResponseTemplate::new(code)).mount(&server).await;
    }
    Mock::given(path("/429"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "120"))
        .mount(&server)
        .await;
    let f = fetcher(&[], &server);
    let at = |p: &str| custom(format!("{}{p}", server.uri()), vec![], Pick::FirstNumber);
    assert_eq!(get(&f, &at("/401")).await.unwrap_err(), FetchError::Auth(401));
    assert_eq!(get(&f, &at("/403")).await.unwrap_err(), FetchError::Auth(403));
    assert_eq!(get(&f, &at("/429")).await.unwrap_err(), FetchError::RateLimited { retry_after_s: Some(120) });
    assert_eq!(get(&f, &at("/500")).await.unwrap_err(), FetchError::Http(500));
    assert_eq!(get(&f, &at("/404")).await.unwrap_err(), FetchError::Http(404));
}

#[tokio::test]
async fn timeout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("1").set_delay(Duration::from_millis(800)))
        .mount(&server)
        .await;
    let s: Arc<dyn SecretStore> = Arc::new(MemorySecrets::new());
    let f = Fetcher::with_timeout(s, Duration::from_millis(200));
    let cfg = custom(server.uri(), vec![], Pick::FirstNumber);
    assert_eq!(get(&f, &cfg).await.unwrap_err(), FetchError::Timeout);
}

#[tokio::test]
async fn unreachable_host_is_network_error() {
    let s: Arc<dyn SecretStore> = Arc::new(MemorySecrets::new());
    let f = Fetcher::with_timeout(s, Duration::from_secs(2));
    let cfg = custom("http://127.0.0.1:9/".into(), vec![], Pick::FirstNumber);
    assert!(matches!(get(&f, &cfg).await.unwrap_err(), FetchError::Network(_)));
}

#[tokio::test]
async fn test_url_returns_preview_and_value() {
    let server = MockServer::start().await;
    Mock::given(path("/ok"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"count": 37})))
        .mount(&server)
        .await;
    Mock::given(path("/denied")).respond_with(ResponseTemplate::new(401).set_body_string("nope")).mount(&server).await;
    let f = fetcher(&[], &server);
    let r = f.test_url(&custom(format!("{}/ok", server.uri()), vec![], Pick::JsonPath("count".into()))).await.unwrap();
    assert_eq!((r.status, r.value, r.truncated), (200, Some(37.0), false));
    assert!(r.body.contains("count"));
    let r = f.test_url(&custom(format!("{}/denied", server.uri()), vec![], Pick::FirstNumber)).await.unwrap();
    assert_eq!(r.status, 401);
    assert_eq!(r.body, "nope");
    assert_eq!(r.value, None);
}

#[tokio::test]
async fn youtube_subs() {
    let server = MockServer::start().await;
    Mock::given(path("/youtube/v3/channels"))
        .and(query_param("id", "UC123"))
        .and(query_param("key", "k"))
        .and(query_param("part", "statistics"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [{"statistics": {"subscriberCount": "48213", "viewCount": "9000000", "videoCount": "120"}}]
        })))
        .mount(&server)
        .await;
    let f = fetcher(&[("yt", "k")], &server);
    let cfg = SourceCfg::YouTube { channel_id: "UC123".into(), metric: YouTubeMetric::Subscribers, api_key: "yt".into() };
    assert_eq!(get(&f, &cfg).await.unwrap(), 48213.0);
    let cfg = SourceCfg::YouTube { channel_id: "UC123".into(), metric: YouTubeMetric::Views, api_key: "yt".into() };
    assert_eq!(get(&f, &cfg).await.unwrap(), 9_000_000.0);
}

#[tokio::test]
async fn youtube_unknown_channel() {
    let server = MockServer::start().await;
    Mock::given(path("/youtube/v3/channels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"items": []})))
        .mount(&server)
        .await;
    let f = fetcher(&[("yt", "k")], &server);
    let cfg = SourceCfg::YouTube { channel_id: "UCx".into(), metric: YouTubeMetric::Subscribers, api_key: "yt".into() };
    assert!(matches!(get(&f, &cfg).await.unwrap_err(), FetchError::Parse(m) if m.contains("channel not found")));
}

#[tokio::test]
async fn github_stars_and_followers() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/tauri-apps/tauri"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"stargazers_count": 1284, "forks_count": 7})))
        .mount(&server)
        .await;
    Mock::given(path("/users/octocat"))
        .and(header("Authorization", "Bearer gh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"followers": 42})))
        .mount(&server)
        .await;
    Mock::given(path("/repos/a/missing")).respond_with(ResponseTemplate::new(404)).mount(&server).await;
    let f = fetcher(&[("tok", "gh")], &server);
    let stars = SourceCfg::GitHub { target: "tauri-apps/tauri".into(), metric: GitHubMetric::Stars, token: None };
    assert_eq!(get(&f, &stars).await.unwrap(), 1284.0);
    let url_form = SourceCfg::GitHub { target: "https://github.com/tauri-apps/tauri/".into(), metric: GitHubMetric::Forks, token: None };
    assert_eq!(get(&f, &url_form).await.unwrap(), 7.0);
    let fol = SourceCfg::GitHub { target: "octocat".into(), metric: GitHubMetric::Followers, token: Some("tok".into()) };
    assert_eq!(get(&f, &fol).await.unwrap(), 42.0);
    let wrong = SourceCfg::GitHub { target: "octocat".into(), metric: GitHubMetric::Stars, token: None };
    assert!(matches!(get(&f, &wrong).await.unwrap_err(), FetchError::Config(_)));
    let missing = SourceCfg::GitHub { target: "a/missing".into(), metric: GitHubMetric::Stars, token: None };
    assert!(matches!(get(&f, &missing).await.unwrap_err(), FetchError::Parse(m) if m.contains("not found")));
    let bad = SourceCfg::GitHub { target: "a/b/c".into(), metric: GitHubMetric::Stars, token: None };
    assert!(matches!(get(&f, &bad).await.unwrap_err(), FetchError::Config(_)));
}

#[tokio::test]
async fn stripe_sums_paginated_paid_charges() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/charges"))
        .and(query_param("starting_after", "ch_2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "has_more": false,
            "data": [
                {"id": "ch_3", "status": "succeeded", "currency": "eur", "amount": 1000, "amount_refunded": 250},
                {"id": "ch_4", "status": "failed", "currency": "eur", "amount": 9999, "amount_refunded": 0}
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(path("/v1/charges"))
        .and(header("Authorization", "Bearer sk"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "has_more": true,
            "data": [
                {"id": "ch_1", "status": "succeeded", "currency": "eur", "amount": 2000, "amount_refunded": 0},
                {"id": "ch_2", "status": "succeeded", "currency": "usd", "amount": 5000, "amount_refunded": 0}
            ]
        })))
        .mount(&server)
        .await;
    let f = fetcher(&[("st", "sk")], &server);
    let cfg = SourceCfg::Stripe { api_key: "st".into(), currency: "EUR".into() };
    assert_eq!(get(&f, &cfg).await.unwrap(), 27.5);
    let bad = SourceCfg::Stripe { api_key: "st".into(), currency: "euro".into() };
    assert!(matches!(get(&f, &bad).await.unwrap_err(), FetchError::Config(_)));
}

#[tokio::test]
async fn shopify_count() {
    let server = MockServer::start().await;
    Mock::given(path("/admin/api/2025-10/orders/count.json"))
        .and(header("X-Shopify-Access-Token", "shp"))
        .and(query_param("status", "any"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"count": 37})))
        .mount(&server)
        .await;
    let f = fetcher(&[("sh", "shp")], &server);
    let cfg = SourceCfg::Shopify { shop: "my-store.myshopify.com".into(), token: "sh".into() };
    assert_eq!(get(&f, &cfg).await.unwrap(), 37.0);
    let bad = SourceCfg::Shopify { shop: "not a shop".into(), token: "sh".into() };
    assert!(matches!(get(&f, &bad).await.unwrap_err(), FetchError::Config(_)));
}
