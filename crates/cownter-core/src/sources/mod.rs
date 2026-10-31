mod custom_url;
mod discord;
mod github;
mod gumroad;
mod lemonsqueezy;
mod npm;
mod plausible;
mod producthunt;
mod shopify;
mod stripe;
mod youtube;

use crate::model::SourceCfg;
use crate::secrets::{SecretError, SecretStore};
use chrono::{DateTime, Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

pub use custom_url::TestResult;

pub const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum FetchError {
    #[error("network error: {0}")]
    Network(String),
    #[error("request timed out")]
    Timeout,
    #[error("access denied (HTTP {0}); check the key or token")]
    Auth(u16),
    #[error("rate limited by the service")]
    RateLimited { retry_after_s: Option<u64> },
    #[error("server answered HTTP {0}")]
    Http(u16),
    #[error("{0}")]
    Parse(String),
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Secret(String),
}

#[derive(Debug, Clone, Default)]
pub struct BaseUrls {
    pub youtube: Option<String>,
    pub github: Option<String>,
    pub stripe: Option<String>,
    pub shopify: Option<String>,
    pub plausible: Option<String>,
    pub npm: Option<String>,
    pub gumroad: Option<String>,
    pub lemonsqueezy: Option<String>,
    pub producthunt: Option<String>,
    pub discord: Option<String>,
}

#[derive(Clone)]
pub struct Fetcher {
    client: reqwest::Client,
    secrets: Arc<dyn SecretStore>,
    base: BaseUrls,
}

pub(crate) struct Body {
    pub status: u16,
    pub content_type: Option<String>,
    pub text: String,
}

impl Fetcher {
    pub fn new(secrets: Arc<dyn SecretStore>) -> Self {
        Self::with_timeout(secrets, TIMEOUT)
    }

    pub fn with_timeout(secrets: Arc<dyn SecretStore>, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(timeout)
            .user_agent(concat!("Cownter/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("http client");
        Fetcher { client, secrets, base: BaseUrls::default() }
    }

    pub fn with_base_urls(mut self, base: BaseUrls) -> Self {
        self.base = base;
        self
    }

    pub fn with_secrets(&self, secrets: Arc<dyn SecretStore>) -> Self {
        Fetcher { client: self.client.clone(), secrets, base: self.base.clone() }
    }

    pub(crate) fn secret(&self, name: &str) -> Result<String, FetchError> {
        match self.secrets.get(name) {
            Ok(Some(v)) if !v.is_empty() => Ok(v),
            Ok(_) => Err(FetchError::Secret("a saved key is missing; open the entry and enter it again".into())),
            Err(SecretError::Unavailable) => Err(FetchError::Secret("the system keyring is not available".into())),
            Err(e) => Err(FetchError::Secret(e.to_string())),
        }
    }

    pub(crate) fn base(&self, which: Option<&String>, default: &str) -> String {
        which.cloned().unwrap_or_else(|| default.to_string()).trim_end_matches('/').to_string()
    }

    /// Sends a request and reads the body, without judging the status code.
    pub(crate) async fn send_raw(&self, req: reqwest::RequestBuilder) -> Result<(Body, Option<u64>), FetchError> {
        let mut resp = req.send().await.map_err(map_reqwest)?;
        let status = resp.status().as_u16();
        let content_type =
            resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(str::to_string);
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_retry_after);
        let mut buf: Vec<u8> = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(map_reqwest)? {
            if buf.len() + chunk.len() > MAX_BODY {
                return Err(FetchError::Parse("response is larger than 5 MB".into()));
            }
            buf.extend_from_slice(&chunk);
        }
        let text = String::from_utf8_lossy(&buf).into_owned();
        Ok((Body { status, content_type, text }, retry_after))
    }

    /// Sends a request and maps non-success statuses to errors.
    pub(crate) async fn send(&self, req: reqwest::RequestBuilder) -> Result<Body, FetchError> {
        let (body, retry_after) = self.send_raw(req).await?;
        match body.status {
            200..=299 => Ok(body),
            401 | 403 => Err(FetchError::Auth(body.status)),
            429 => Err(FetchError::RateLimited { retry_after_s: retry_after }),
            s => Err(FetchError::Http(s)),
        }
    }

    pub(crate) async fn json(&self, req: reqwest::RequestBuilder) -> Result<serde_json::Value, FetchError> {
        let body = self.send(req).await?;
        serde_json::from_str(&body.text).map_err(|_| FetchError::Parse("response is not JSON".into()))
    }

    pub async fn fetch(&self, cfg: &SourceCfg, now: DateTime<Local>) -> Result<f64, FetchError> {
        match cfg {
            SourceCfg::CustomUrl { .. } => custom_url::fetch(self, cfg).await,
            SourceCfg::YouTube { channel_id, metric, api_key } => {
                youtube::fetch(self, channel_id, *metric, api_key).await
            }
            SourceCfg::GitHub { target, metric, token } => github::fetch(self, target, *metric, token.as_deref()).await,
            SourceCfg::Shopify { shop, token } => shopify::fetch(self, shop, token, now).await,
            SourceCfg::Stripe { api_key, currency } => stripe::fetch(self, api_key, currency, now).await,
            SourceCfg::Plausible { site_id, metric, api_key } => plausible::fetch(self, site_id, *metric, api_key).await,
            SourceCfg::Npm { package } => npm::fetch(self, package).await,
            SourceCfg::Gumroad { metric, token } => gumroad::fetch(self, *metric, token).await,
            SourceCfg::LemonSqueezy { metric, api_key } => lemonsqueezy::fetch(self, *metric, api_key).await,
            SourceCfg::ProductHunt { post, token } => producthunt::fetch(self, post, token).await,
            SourceCfg::Discord { invite, metric } => discord::fetch(self, invite, *metric).await,
            SourceCfg::Formula { .. } => Err(FetchError::Config("formulas are computed, not fetched".into())),
            SourceCfg::Push => Err(FetchError::Config("push numbers are sent to Cownter, not fetched".into())),
        }
    }

    /// Fetches a text value; only the Any URL source can do this.
    pub async fn fetch_text(&self, cfg: &SourceCfg) -> Result<String, FetchError> {
        match cfg {
            SourceCfg::CustomUrl { .. } => custom_url::fetch_text(self, cfg).await,
            _ => Err(FetchError::Config("text values only work with the Any URL source".into())),
        }
    }

    pub async fn test_url(&self, cfg: &SourceCfg) -> Result<TestResult, FetchError> {
        custom_url::test(self, cfg).await
    }
}

fn map_reqwest(e: reqwest::Error) -> FetchError {
    if e.is_timeout() {
        FetchError::Timeout
    } else if e.is_builder() {
        FetchError::Config(format!("invalid request: {e}"))
    } else {
        let mut msg = e.to_string();
        let mut src = std::error::Error::source(&e);
        while let Some(s) = src {
            msg = s.to_string();
            src = s.source();
        }
        FetchError::Network(msg)
    }
}

pub fn parse_retry_after(v: &str) -> Option<u64> {
    let v = v.trim();
    if let Ok(n) = v.parse::<u64>() {
        return Some(n);
    }
    let when = httpdate::parse_http_date(v).ok()?;
    Some(when.duration_since(std::time::SystemTime::now()).map(|d| d.as_secs()).unwrap_or(0))
}

pub(crate) fn local_midnight(now: DateTime<Local>) -> DateTime<Local> {
    let naive = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    Local.from_local_datetime(&naive).earliest().unwrap_or(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_formats() {
        assert_eq!(parse_retry_after("120"), Some(120));
        assert_eq!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"), Some(0));
        assert_eq!(parse_retry_after("soon"), None);
    }
}
