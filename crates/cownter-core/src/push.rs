use crate::model::{Entry, EntryView};
use crate::rules;
use crate::scheduler::{Scheduler, day_start_ms, now_ms};
use crate::secrets::SecretStore;
use crate::store::Store;
use serde_json::{Map, Value, json};
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const DEFAULT_PORT: u16 = 47615;
/// Keyring name of the push token.
pub const TOKEN_SECRET: &str = "push-token";
/// Settings key used for the token when no keyring is available.
const TOKEN_SETTING: &str = "push_token";
const TOKEN_LEN: usize = 40;
const MAX_HEAD: usize = 16 * 1024;
const MAX_BODY: usize = 64 * 1024;
const MAX_TEXT: usize = 1000;
const IO_TIMEOUT: Duration = Duration::from_secs(5);

pub fn new_token() -> String {
    use rand::Rng;
    rand::rng().sample_iter(&rand::distr::Alphanumeric).take(TOKEN_LEN).map(char::from).collect()
}

/// Reads the saved token from the keyring, or from the database when the keyring has none.
pub fn read_token(secrets: &dyn SecretStore, store: &Store) -> Option<String> {
    if secrets.available() {
        if let Ok(Some(t)) = secrets.get(TOKEN_SECRET) {
            if !t.is_empty() {
                return Some(t);
            }
        }
    }
    store.setting(TOKEN_SETTING).ok().flatten().filter(|t| !t.is_empty())
}

/// Saves the token in the keyring, or in the database when the keyring is not available.
pub fn save_token(secrets: &dyn SecretStore, store: &Store, token: &str) -> Result<(), String> {
    if secrets.available() && secrets.set(TOKEN_SECRET, token).is_ok() {
        let _ = store.delete_setting(TOKEN_SETTING);
        return Ok(());
    }
    store.set_setting(TOKEN_SETTING, token).map_err(|e| e.to_string())
}

/// Returns the saved token, creating one on first run.
pub fn ensure_token(secrets: &dyn SecretStore, store: &Store) -> Result<String, String> {
    if let Some(t) = read_token(secrets, store) {
        return Ok(t);
    }
    let t = new_token();
    save_token(secrets, store, &t)?;
    Ok(t)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    pub fn query(&self, name: &str) -> Option<&str> {
        self.query.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub status: u16,
    pub body: Value,
}

impl Response {
    pub fn ok(body: Value) -> Response {
        Response { status: 200, body }
    }

    pub fn err(status: u16, msg: impl Into<String>) -> Response {
        Response { status, body: json!({ "error": msg.into() }) }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let body = self.body.to_string();
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{}",
            self.status,
            reason(self.status),
            body.len(),
            body
        )
        .into_bytes()
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        _ => "Internal Server Error",
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() && b[i + 1].is_ascii_hexdigit() && b[i + 2].is_ascii_hexdigit() => {
                let hex = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
                out.push(hex(b[i + 1]) * 16 + hex(b[i + 2]));
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Percent-encodes a query value.
pub fn encode_query(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn parse_query(q: &str) -> Vec<(String, String)> {
    q.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| match p.split_once('=') {
            Some((k, v)) => (percent_decode(k), percent_decode(v)),
            None => (percent_decode(p), String::new()),
        })
        .collect()
}

/// Parses one HTTP/1.1 request from the start of `buf`; `Ok(None)` means more bytes are needed.
pub fn parse_request(buf: &[u8]) -> Result<Option<Request>, Response> {
    let bad = |m: &str| Response::err(400, m);
    let Some(end) = find(buf, b"\r\n\r\n") else {
        return if buf.len() > MAX_HEAD { Err(Response::err(431, "request headers are too large")) } else { Ok(None) };
    };
    if end > MAX_HEAD {
        return Err(Response::err(431, "request headers are too large"));
    }
    let head = std::str::from_utf8(&buf[..end]).map_err(|_| bad("request headers are not valid text"))?;
    let mut lines = head.split("\r\n");
    let mut start = lines.next().unwrap_or("").split(' ');
    let (Some(method), Some(target), Some(version), None) = (start.next(), start.next(), start.next(), start.next()) else {
        return Err(bad("malformed request line"));
    };
    if method.is_empty() || !method.bytes().all(|c| c.is_ascii_uppercase()) || !target.starts_with('/') || !version.starts_with("HTTP/1.") {
        return Err(bad("malformed request line"));
    }
    let mut headers = Vec::new();
    for line in lines {
        let Some((k, v)) = line.split_once(':') else { return Err(bad("malformed header")) };
        if k.is_empty() || k.bytes().any(|c| c.is_ascii_whitespace()) {
            return Err(bad("malformed header"));
        }
        headers.push((k.to_string(), v.trim().to_string()));
    }
    let get = |n: &str| headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(n)).map(|(_, v)| v.as_str());
    if get("transfer-encoding").is_some() {
        return Err(Response::err(501, "chunked bodies are not supported; send Content-Length"));
    }
    let len = match get("content-length") {
        Some(v) => v.parse::<usize>().map_err(|_| bad("invalid Content-Length"))?,
        None => 0,
    };
    if len > MAX_BODY {
        return Err(Response::err(413, "the body is larger than 64 KB"));
    }
    let body_start = end + 4;
    if buf.len() < body_start + len {
        return Ok(None);
    }
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, parse_query(q)),
        None => (target, vec![]),
    };
    Ok(Some(Request {
        method: method.to_string(),
        path: path.to_string(),
        query,
        headers,
        body: buf[body_start..body_start + len].to_vec(),
    }))
}

/// What the request handler needs; the token can be replaced while the server runs.
pub struct Ctx {
    pub store: Arc<Store>,
    pub scheduler: Scheduler,
    token: RwLock<String>,
    writes: Mutex<()>,
}

impl Ctx {
    pub fn new(store: Arc<Store>, scheduler: Scheduler, token: String) -> Ctx {
        Ctx { store, scheduler, token: RwLock::new(token), writes: Mutex::new(()) }
    }

    pub fn set_token(&self, token: String) {
        *self.token.write().unwrap_or_else(|p| p.into_inner()) = token;
    }

    fn authorized(&self, req: &Request) -> bool {
        let Some((scheme, given)) = req.header("authorization").and_then(|h| h.split_once(' ')) else { return false };
        let want = self.token.read().unwrap_or_else(|p| p.into_inner());
        scheme.eq_ignore_ascii_case("bearer") && !want.is_empty() && same(given.trim().as_bytes(), want.as_bytes())
    }
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

type Res = Result<Response, Response>;

fn internal(e: impl std::fmt::Display) -> Response {
    Response::err(500, e.to_string())
}

pub fn handle(ctx: &Ctx, req: &Request) -> Response {
    if !matches!(req.path.as_str(), "/v1/set" | "/v1/add" | "/v1/get" | "/v1/list") {
        return Response::err(404, "unknown endpoint; use /v1/set, /v1/add, /v1/get or /v1/list");
    }
    if !ctx.authorized(req) {
        return Response::err(401, "missing or wrong token; send Authorization: Bearer <token>");
    }
    let res = match (req.method.as_str(), req.path.as_str()) {
        ("POST", "/v1/set") => set(ctx, req),
        ("POST", "/v1/add") => add(ctx, req),
        ("GET", "/v1/get") => get(ctx, req),
        ("GET", "/v1/list") => list(ctx),
        _ => Err(Response::err(405, "method not allowed")),
    };
    res.unwrap_or_else(|e| e)
}

fn body_json(req: &Request) -> Result<Map<String, Value>, Response> {
    match serde_json::from_slice::<Value>(&req.body) {
        Ok(Value::Object(m)) => Ok(m),
        _ => Err(Response::err(400, "the body must be a JSON object")),
    }
}

fn entry_key(v: Option<&Value>) -> Result<String, Response> {
    match v {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        _ => Err(Response::err(400, "`entry` must be the name, short name or id of a number")),
    }
}

fn number(v: Option<&Value>, field: &str) -> Result<f64, Response> {
    let n = match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    n.filter(|x| x.is_finite()).ok_or_else(|| Response::err(400, format!("`{field}` must be a number")))
}

/// Finds an entry by id, short name or (case-insensitive) name.
pub fn find_entry(store: &Store, key: &str) -> Result<Entry, Response> {
    let entries = store.entries().map_err(internal)?;
    if let Ok(id) = key.parse::<i64>() {
        if let Some(e) = entries.iter().find(|e| e.id == id) {
            return Ok(e.clone());
        }
    }
    if let Some(e) = entries.iter().find(|e| e.slug == key) {
        return Ok(e.clone());
    }
    let want = key.to_lowercase();
    let named: Vec<&Entry> = entries.iter().filter(|e| e.name.trim().to_lowercase() == want).collect();
    match named.as_slice() {
        [e] => Ok((*e).clone()),
        [] => Err(Response::err(404, format!("no number called `{key}`"))),
        _ => Err(Response::err(409, format!("more than one number is called `{key}`; use its short name or id"))),
    }
}

fn push_entry(store: &Store, key: &str) -> Result<Entry, Response> {
    let e = find_entry(store, key)?;
    if !e.source.is_push() {
        return Err(Response::err(409, format!("`{}` gets its value from its source; only Push numbers can be set", e.name)));
    }
    Ok(e)
}

fn view_json(v: &EntryView) -> Value {
    json!({
        "id": v.entry.id,
        "name": v.entry.name,
        "slug": v.entry.slug,
        "push": v.entry.source.is_push(),
        "value": v.value,
        "text": v.text,
        "status": v.status,
        "updatedAt": v.updated_at,
        "deltaToday": v.delta_today,
    })
}

fn summary(ctx: &Ctx, e: Entry) -> Res {
    let v = rules::build_view(&ctx.store, e, now_ms(), day_start_ms()).map_err(internal)?;
    Ok(Response::ok(view_json(&v)))
}

fn set(ctx: &Ctx, req: &Request) -> Res {
    let body = body_json(req)?;
    let key = entry_key(body.get("entry"))?;
    let _w = ctx.writes.lock().unwrap_or_else(|p| p.into_inner());
    let e = push_entry(&ctx.store, &key)?;
    if e.text_mode {
        let text = match body.get("value") {
            Some(Value::String(s)) => s.trim().to_string(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        if text.is_empty() {
            return Err(Response::err(400, "`value` must be a text"));
        }
        if text.chars().count() > MAX_TEXT {
            return Err(Response::err(400, format!("the text is longer than {MAX_TEXT} characters")));
        }
        ctx.scheduler.record(e.id, None, Some(text)).map_err(internal)?;
    } else {
        let v = number(body.get("value"), "value")?;
        ctx.scheduler.record(e.id, Some(v), None).map_err(internal)?;
    }
    summary(ctx, e)
}

fn add(ctx: &Ctx, req: &Request) -> Res {
    let body = body_json(req)?;
    let key = entry_key(body.get("entry"))?;
    let by = match body.get("by") {
        None => 1.0,
        v => number(v, "by")?,
    };
    let _w = ctx.writes.lock().unwrap_or_else(|p| p.into_inner());
    let e = push_entry(&ctx.store, &key)?;
    if e.text_mode {
        return Err(Response::err(409, format!("`{}` shows text, so it cannot be added to", e.name)));
    }
    let base = ctx.store.latest_value(e.id).map_err(internal)?.map(|(_, v)| v).unwrap_or(0.0);
    let v = base + by;
    if !v.is_finite() {
        return Err(Response::err(400, "the result is not a finite number"));
    }
    ctx.scheduler.record(e.id, Some(v), None).map_err(internal)?;
    summary(ctx, e)
}

fn get(ctx: &Ctx, req: &Request) -> Res {
    let key = req
        .query("entry")
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .ok_or_else(|| Response::err(400, "add ?entry=<name, short name or id>"))?;
    let e = find_entry(&ctx.store, key)?;
    summary(ctx, e)
}

fn list(ctx: &Ctx) -> Res {
    let (now, day) = (now_ms(), day_start_ms());
    let mut items = Vec::new();
    for e in ctx.store.entries().map_err(internal)? {
        items.push(view_json(&rules::build_view(&ctx.store, e, now, day).map_err(internal)?));
    }
    Ok(Response::ok(json!({ "entries": items })))
}

/// Binds the push endpoint on 127.0.0.1 only.
pub fn bind(port: u16) -> Result<std::net::TcpListener, String> {
    let l = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).map_err(|e| match e.kind() {
        ErrorKind::AddrInUse => format!("port {port} is already in use; pick another port"),
        ErrorKind::PermissionDenied => format!("not allowed to use port {port}; pick another port"),
        _ => format!("could not open port {port}: {e}"),
    })?;
    l.set_nonblocking(true).map_err(|e| e.to_string())?;
    Ok(l)
}

/// Answers requests until the task is dropped.
pub async fn serve(listener: std::net::TcpListener, ctx: Arc<Ctx>) -> Result<(), String> {
    let listener = tokio::net::TcpListener::from_std(listener).map_err(|e| e.to_string())?;
    loop {
        match listener.accept().await {
            Ok((sock, peer)) if peer.ip().is_loopback() => {
                let ctx = ctx.clone();
                tokio::spawn(connection(sock, ctx));
            }
            Ok(_) => {}
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}

async fn read_request(sock: &mut tokio::net::TcpStream) -> Result<Request, Response> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(r) = parse_request(&buf)? {
            return Ok(r);
        }
        let n = sock.read(&mut chunk).await.map_err(|_| Response::err(400, "could not read the request"))?;
        if n == 0 {
            return Err(Response::err(400, "the request ended early"));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

async fn connection(mut sock: tokio::net::TcpStream, ctx: Arc<Ctx>) {
    let resp = match tokio::time::timeout(IO_TIMEOUT, read_request(&mut sock)).await {
        Ok(Ok(req)) => tokio::task::spawn_blocking(move || handle(&ctx, &req))
            .await
            .unwrap_or_else(|_| Response::err(500, "internal error")),
        Ok(Err(r)) => r,
        Err(_) => Response::err(408, "request timed out"),
    };
    let _ = tokio::time::timeout(IO_TIMEOUT, async {
        let _ = sock.write_all(&resp.to_bytes()).await;
        let _ = sock.shutdown().await;
    })
    .await;
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ClientError {
    #[error("Cownter isn't running")]
    NotRunning,
    #[error("{0}")]
    Failed(String),
}

/// Sends one request to the local endpoint and returns the status and JSON body.
pub fn call(port: u16, token: &str, method: &str, path: &str, body: Option<&Value>) -> Result<(u16, Value), ClientError> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| match e.kind() {
        ErrorKind::ConnectionRefused => ClientError::NotRunning,
        _ => ClientError::Failed(format!("could not reach Cownter: {e}")),
    })?;
    let io = |e: std::io::Error| ClientError::Failed(format!("could not talk to Cownter: {e}"));
    s.set_read_timeout(Some(Duration::from_secs(10))).map_err(io)?;
    s.set_write_timeout(Some(Duration::from_secs(10))).map_err(io)?;
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\nContent-Length: {}\r\n",
        payload.len()
    );
    if body.is_some() {
        req.push_str("Content-Type: application/json\r\n");
    }
    req.push_str("\r\n");
    req.push_str(&payload);
    s.write_all(req.as_bytes()).map_err(io)?;
    let mut out = Vec::new();
    s.read_to_end(&mut out).map_err(io)?;
    parse_response(&out)
}

pub fn parse_response(raw: &[u8]) -> Result<(u16, Value), ClientError> {
    let invalid = || ClientError::Failed("got an answer that is not from Cownter; check the port".into());
    let end = find(raw, b"\r\n\r\n").ok_or_else(invalid)?;
    let head = String::from_utf8_lossy(&raw[..end]);
    if !head.starts_with("HTTP/1.") {
        return Err(invalid());
    }
    let status = head.split(' ').nth(1).and_then(|s| s.parse::<u16>().ok()).ok_or_else(invalid)?;
    let body = serde_json::from_slice(&raw[end + 4..]).unwrap_or(Value::Null);
    Ok((status, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::scheduler::SchedulerOptions;
    use crate::secrets::{MemorySecrets, SecretError};
    use crate::sources::Fetcher;
    use crate::store::tests::{entry, url_source};
    use tokio::sync::mpsc::UnboundedReceiver;

    const TOKEN: &str = "t0k3n";

    fn req(method: &str, path: &str, auth: Option<&str>, body: &str) -> Request {
        let mut raw = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n");
        if let Some(t) = auth {
            raw.push_str(&format!("Authorization: Bearer {t}\r\n"));
        }
        raw.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
        parse_request(raw.as_bytes()).unwrap().unwrap()
    }

    fn post(path: &str, body: Value) -> Request {
        req("POST", path, Some(TOKEN), &body.to_string())
    }

    fn setup() -> (Arc<Ctx>, UnboundedReceiver<CoreEvent>) {
        let store = Arc::new(Store::open_memory().unwrap());
        let fetcher = Arc::new(Fetcher::new(Arc::new(MemorySecrets::new())));
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let scheduler = Scheduler::start_with(store.clone(), fetcher, tx, SchedulerOptions::default());
        let mut orders = entry("orders", SourceCfg::Push);
        orders.name = "Orders".into();
        store.insert_entry(&orders, 0).unwrap();
        (Arc::new(Ctx::new(store, scheduler, TOKEN.into())), rx)
    }

    fn drain(rx: &mut UnboundedReceiver<CoreEvent>) -> Vec<CoreEvent> {
        let mut out = vec![];
        while let Ok(e) = rx.try_recv() {
            out.push(e);
        }
        out
    }

    #[test]
    fn parses_requests() {
        let r = parse_request(b"POST /v1/set HTTP/1.1\r\nContent-Length: 5\r\nauthorization: Bearer x\r\n\r\nhello").unwrap().unwrap();
        assert_eq!((r.method.as_str(), r.path.as_str(), r.body.as_slice()), ("POST", "/v1/set", &b"hello"[..]));
        assert_eq!(r.header("Authorization"), Some("Bearer x"));
        let g = parse_request(b"GET /v1/get?entry=Orders%20today&x=a+b HTTP/1.1\r\n\r\n").unwrap().unwrap();
        assert_eq!((g.path.as_str(), g.query("entry"), g.query("x")), ("/v1/get", Some("Orders today"), Some("a b")));
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode(&encode_query("Café & co")), "Café & co");
    }

    #[test]
    fn waits_for_the_whole_request() {
        assert_eq!(parse_request(b"POST /v1/set HTTP/1.1\r\nContent-Le").unwrap(), None);
        assert_eq!(parse_request(b"POST /v1/set HTTP/1.1\r\nContent-Length: 10\r\n\r\nabc").unwrap(), None);
    }

    #[test]
    fn rejects_bad_requests() {
        let status = |raw: &[u8]| parse_request(raw).unwrap_err().status;
        assert_eq!(status(b"GARBAGE\r\n\r\n"), 400);
        assert_eq!(status(b"get / HTTP/1.1\r\n\r\n"), 400);
        assert_eq!(status(b"GET / HTTP/1.1\r\nno colon\r\n\r\n"), 400);
        assert_eq!(status(b"POST / HTTP/1.1\r\nContent-Length: x\r\n\r\n"), 400);
        assert_eq!(status(b"POST / HTTP/1.1\r\nContent-Length: 999999\r\n\r\n"), 413);
        assert_eq!(status(b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n"), 501);
        assert_eq!(status(&vec![b'a'; MAX_HEAD + 1]), 431);
    }

    #[test]
    fn reads_responses() {
        let r = Response::ok(json!({"value": 3})).to_bytes();
        assert_eq!(parse_response(&r).unwrap(), (200, json!({"value": 3})));
        assert!(parse_response(b"SSH-2.0-OpenSSH\r\n\r\n").is_err());
    }

    #[tokio::test]
    async fn needs_the_token() {
        let (ctx, _rx) = setup();
        let body = r#"{"entry":"Orders","value":1}"#;
        assert_eq!(handle(&ctx, &req("POST", "/v1/set", None, body)).status, 401);
        assert_eq!(handle(&ctx, &req("POST", "/v1/set", Some("nope"), body)).status, 401);
        assert_eq!(handle(&ctx, &req("GET", "/v1/list", Some("nope"), "")).status, 401);
        assert_eq!(handle(&ctx, &req("POST", "/v1/set", Some(TOKEN), body)).status, 200);
        ctx.set_token("new".into());
        assert_eq!(handle(&ctx, &req("POST", "/v1/set", Some(TOKEN), body)).status, 401);
        assert_eq!(handle(&ctx, &req("POST", "/nope", Some("new"), body)).status, 404);
        assert_eq!(handle(&ctx, &req("GET", "/v1/set", Some("new"), body)).status, 405);
    }

    #[tokio::test]
    async fn push_records_like_a_poll() {
        let (ctx, mut rx) = setup();
        let id = ctx.store.entry_by_slug("orders").unwrap().unwrap().id;
        ctx.store
            .upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::Above, threshold: 100.0, window_s: 3600, enabled: true, last_fired_at: None, notify: true, effect: String::new() })
            .unwrap();
        let r = handle(&ctx, &post("/v1/set", json!({"entry": "orders", "value": 42})));
        assert_eq!((r.status, r.body["value"].clone(), r.body["status"].clone()), (200, json!(42.0), json!("live")));
        assert_eq!(ctx.store.latest_value(id).unwrap().unwrap().1, 42.0);
        assert!(drain(&mut rx).iter().any(|e| matches!(e, CoreEvent::EntryUpdated { view } if view.value == Some(42.0))));

        let r = handle(&ctx, &post("/v1/set", json!({"entry": "ORDERS", "value": "150"})));
        assert_eq!(r.body["value"], json!(150.0));
        let evs = drain(&mut rx);
        assert!(evs.iter().any(|e| matches!(e, CoreEvent::ValueChanged { value, prev, .. } if *value == 150.0 && *prev == 42.0)));
        assert!(evs.iter().any(|e| matches!(e, CoreEvent::AlertFired { .. })));

        assert_eq!(handle(&ctx, &post("/v1/add", json!({"entry": "Orders", "by": 2.5}))).body["value"], json!(152.5));
        assert_eq!(handle(&ctx, &post("/v1/add", json!({"entry": id}))).body["value"], json!(153.5));
        let g = handle(&ctx, &req("GET", "/v1/get?entry=Orders", Some(TOKEN), ""));
        assert_eq!((g.body["value"].clone(), g.body["deltaToday"].clone()), (json!(153.5), json!(111.5)));
    }

    #[tokio::test]
    async fn validates_pushes() {
        let (ctx, _rx) = setup();
        ctx.store.insert_entry(&entry("polled", url_source()), 0).unwrap();
        let st = |path: &str, body: Value| handle(&ctx, &post(path, body)).status;
        assert_eq!(st("/v1/set", json!({"entry": "Orders", "value": "abc"})), 400);
        assert_eq!(st("/v1/set", json!({"entry": "Orders"})), 400);
        assert_eq!(st("/v1/set", json!({"value": 1})), 400);
        assert_eq!(st("/v1/set", json!({"entry": "missing", "value": 1})), 404);
        assert_eq!(st("/v1/set", json!({"entry": "polled", "value": 1})), 409);
        assert_eq!(st("/v1/add", json!({"entry": "Orders", "by": "x"})), 400);
        assert_eq!(handle(&ctx, &req("POST", "/v1/set", Some(TOKEN), "not json")).status, 400);
        assert_eq!(handle(&ctx, &req("GET", "/v1/get", Some(TOKEN), "")).status, 400);
        assert_eq!(handle(&ctx, &req("GET", "/v1/get?entry=polled", Some(TOKEN), "")).status, 200);
    }

    #[tokio::test]
    async fn text_pushes_and_list() {
        let (ctx, mut rx) = setup();
        let mut e = entry("deploy", SourceCfg::Push);
        e.name = "Deploy".into();
        e.text_mode = true;
        ctx.store.insert_entry(&e, 0).unwrap();
        assert_eq!(handle(&ctx, &post("/v1/set", json!({"entry": "Deploy", "value": "green"}))).body["text"], json!("green"));
        drain(&mut rx);
        handle(&ctx, &post("/v1/set", json!({"entry": "Deploy", "value": "red"})));
        assert!(drain(&mut rx).iter().any(|e| matches!(e, CoreEvent::TextChanged { text, prev, .. } if text == "red" && prev == "green")));
        assert_eq!(handle(&ctx, &post("/v1/add", json!({"entry": "Deploy"}))).status, 409);
        assert_eq!(handle(&ctx, &post("/v1/set", json!({"entry": "Deploy", "value": "x".repeat(MAX_TEXT + 1)}))).status, 400);
        let l = handle(&ctx, &req("GET", "/v1/list", Some(TOKEN), ""));
        let names: Vec<&str> = l.body["entries"].as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["Orders", "Deploy"]);
    }

    #[tokio::test]
    async fn duplicate_names_need_a_slug() {
        let (ctx, _rx) = setup();
        let mut e = entry("orders2", SourceCfg::Push);
        e.name = "orders ".into();
        ctx.store.insert_entry(&e, 0).unwrap();
        assert_eq!(find_entry(&ctx.store, "Orders").unwrap_err().status, 409);
        assert_eq!(find_entry(&ctx.store, "orders2").unwrap().slug, "orders2");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn serves_over_tcp() {
        let (ctx, _rx) = setup();
        let listener = bind(0).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(bind(port).unwrap_err().contains("already in use"));
        let server = tokio::spawn(serve(listener, ctx.clone()));
        let (set, get, bad) = tokio::task::spawn_blocking(move || {
            (
                call(port, TOKEN, "POST", "/v1/set", Some(&json!({"entry": "Orders", "value": 7}))).unwrap(),
                call(port, TOKEN, "GET", &format!("/v1/get?entry={}", encode_query("Orders")), None).unwrap(),
                call(port, "wrong", "GET", "/v1/list", None).unwrap(),
            )
        })
        .await
        .unwrap();
        assert_eq!((set.0, set.1["value"].clone()), (200, json!(7.0)));
        assert_eq!(get.1["value"], json!(7.0));
        assert_eq!(bad.0, 401);
        server.abort();
        let _ = server.await;
        let closed = tokio::task::spawn_blocking(move || call(port, TOKEN, "GET", "/v1/list", None)).await.unwrap();
        assert_eq!(closed.unwrap_err(), ClientError::NotRunning);
    }

    struct NoKeyring;

    impl SecretStore for NoKeyring {
        fn get(&self, _: &str) -> Result<Option<String>, SecretError> {
            Err(SecretError::Unavailable)
        }
        fn set(&self, _: &str, _: &str) -> Result<(), SecretError> {
            Err(SecretError::Unavailable)
        }
        fn delete(&self, _: &str) -> Result<(), SecretError> {
            Err(SecretError::Unavailable)
        }
        fn available(&self) -> bool {
            false
        }
    }

    #[test]
    fn token_lives_in_the_keyring_or_the_store() {
        let store = Store::open_memory().unwrap();
        let keyring = MemorySecrets::new();
        let t = ensure_token(&keyring, &store).unwrap();
        assert_eq!(t.len(), TOKEN_LEN);
        assert_eq!(keyring.get(TOKEN_SECRET).unwrap().as_deref(), Some(t.as_str()));
        assert_eq!(store.setting(TOKEN_SETTING).unwrap(), None);
        assert_eq!(ensure_token(&keyring, &store).unwrap(), t);

        let plain = Store::open_memory().unwrap();
        let t2 = ensure_token(&NoKeyring, &plain).unwrap();
        assert_ne!(t2, t);
        assert_eq!(read_token(&NoKeyring, &plain).as_deref(), Some(t2.as_str()));
        assert_eq!(plain.setting(TOKEN_SETTING).unwrap().as_deref(), Some(t2.as_str()));
    }
}
