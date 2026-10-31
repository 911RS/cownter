use crate::state::AppState;
use cownter_core::push::{self, Ctx};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushInfo {
    pub port: u16,
    pub url: String,
    pub token: String,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Default)]
struct Run {
    port: u16,
    task: Option<JoinHandle<()>>,
    error: Option<String>,
}

/// The local push endpoint and its current state.
pub struct PushServer {
    ctx: Arc<Ctx>,
    token: Mutex<String>,
    run: Mutex<Run>,
}

impl PushServer {
    pub fn new(ctx: Ctx, token: String) -> Self {
        PushServer { ctx: Arc::new(ctx), token: Mutex::new(token), run: Mutex::new(Run::default()) }
    }

    pub fn info(&self) -> PushInfo {
        let run = self.run.lock().unwrap_or_else(|p| p.into_inner());
        PushInfo {
            port: run.port,
            url: format!("http://127.0.0.1:{}", run.port),
            token: self.token.lock().unwrap_or_else(|p| p.into_inner()).clone(),
            running: run.task.is_some(),
            error: run.error.clone(),
        }
    }

    pub fn set_token(&self, token: String) {
        self.ctx.set_token(token.clone());
        *self.token.lock().unwrap_or_else(|p| p.into_inner()) = token;
    }

    /// Listens on `port`; does nothing when already listening there unless `force` is set.
    fn start(&self, port: u16, force: bool) -> bool {
        let mut run = self.run.lock().unwrap_or_else(|p| p.into_inner());
        if !force && run.port == port && run.task.is_some() {
            return false;
        }
        let same_port = run.port == port;
        let stopped = match run.task.take() {
            Some(t) => {
                t.abort();
                true
            }
            None => false,
        };
        run.port = port;
        let tries = if stopped && same_port { 10 } else { 1 };
        let mut bound = Err(String::new());
        for i in 0..tries {
            if i > 0 {
                std::thread::sleep(Duration::from_millis(50));
            }
            bound = push::bind(port);
            if bound.is_ok() {
                break;
            }
        }
        match bound {
            Ok(listener) => {
                let ctx = self.ctx.clone();
                run.task = Some(tauri::async_runtime::spawn(async move {
                    if let Err(e) = push::serve(listener, ctx).await {
                        eprintln!("push endpoint: {e}");
                    }
                }));
                run.error = None;
            }
            Err(e) => {
                eprintln!("push endpoint: {e}");
                run.error = Some(e);
            }
        }
        true
    }
}

/// Starts or moves the endpoint and tells the settings window.
pub fn start(app: &AppHandle, port: u16, force: bool) -> Option<PushInfo> {
    let state = app.try_state::<AppState>()?;
    let changed = state.push.start(port, force);
    let info = state.push.info();
    if changed {
        let _ = app.emit("push-changed", &info);
    }
    Some(info)
}
