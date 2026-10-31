use super::{FetchError, Fetcher};
use crate::model::{Pick, SecretOrText, SourceCfg};
use crate::parse::extract;
use serde::{Deserialize, Serialize};
use std::time::Instant;

const PREVIEW_LIMIT: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub status: u16,
    pub elapsed_ms: u64,
    pub content_type: Option<String>,
    pub body: String,
    pub truncated: bool,
    pub value: Option<f64>,
    pub error: Option<String>,
}

fn build(f: &Fetcher, cfg: &SourceCfg) -> Result<(reqwest::RequestBuilder, Pick), FetchError> {
    let SourceCfg::CustomUrl { url, method, headers, body, pick } = cfg else {
        return Err(FetchError::Config("not a custom URL source".into()));
    };
    let parsed = reqwest::Url::parse(url.trim()).map_err(|_| FetchError::Config("the URL is not valid".into()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(FetchError::Config("the URL must start with http:// or https://".into()));
    }
    let m = match method.to_ascii_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        other => return Err(FetchError::Config(format!("method {other} is not supported"))),
    };
    let mut req = f.client.request(m.clone(), parsed);
    for h in headers {
        let name = h.name.trim();
        if name.is_empty() {
            continue;
        }
        let value = match &h.value {
            SecretOrText::Text(t) => t.clone(),
            SecretOrText::Secret(n) => f.secret(n)?,
        };
        let hn = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| FetchError::Config(format!("header name `{name}` is not valid")))?;
        let hv = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|_| FetchError::Config(format!("header `{name}` has an invalid value")))?;
        req = req.header(hn, hv);
    }
    if m == reqwest::Method::POST {
        if let Some(b) = body {
            req = req.body(b.clone());
        }
    }
    Ok((req, pick.clone()))
}

pub(super) async fn fetch(f: &Fetcher, cfg: &SourceCfg) -> Result<f64, FetchError> {
    let (req, pick) = build(f, cfg)?;
    let body = f.send(req).await?;
    extract(&body.text, &pick).map_err(FetchError::Parse)
}

pub(super) async fn test(f: &Fetcher, cfg: &SourceCfg) -> Result<TestResult, FetchError> {
    let (req, pick) = build(f, cfg)?;
    let started = Instant::now();
    let (body, _) = f.send_raw(req).await?;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let (value, error) = if (200..300).contains(&body.status) {
        match extract(&body.text, &pick) {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        }
    } else {
        (None, Some(format!("server answered HTTP {}", body.status)))
    };
    let truncated = body.text.len() > PREVIEW_LIMIT;
    let mut preview = body.text;
    if truncated {
        let mut cut = PREVIEW_LIMIT;
        while !preview.is_char_boundary(cut) {
            cut -= 1;
        }
        preview.truncate(cut);
    }
    Ok(TestResult {
        status: body.status,
        elapsed_ms,
        content_type: body.content_type,
        body: preview,
        truncated,
        value,
        error,
    })
}
