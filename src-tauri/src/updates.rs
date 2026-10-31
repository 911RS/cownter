use crate::state::AppState;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::{Update, UpdaterExt};

const FIRST_CHECK: Duration = Duration::from_secs(60);
const EVERY: Duration = Duration::from_secs(6 * 3600);

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub available: Option<String>,
    pub notes: Option<String>,
    pub error: Option<String>,
}

/// Update found by the last check, kept so installing doesn't download the manifest again.
#[derive(Default)]
pub struct Pending(pub Mutex<Option<Update>>);

fn pending(app: &AppHandle) -> std::sync::MutexGuard<'_, Option<Update>> {
    app.state::<Pending>().inner().0.lock().unwrap_or_else(|p| p.into_inner())
}

pub async fn check(app: &AppHandle) -> UpdateInfo {
    let current = app.package_info().version.to_string();
    let found = match app.updater() {
        Ok(u) => u.check().await,
        Err(e) => Err(e),
    };
    match found {
        Ok(Some(u)) => {
            let info = UpdateInfo { current, available: Some(u.version.clone()), notes: u.body.clone(), error: None };
            *pending(app) = Some(u);
            let _ = app.emit("update-available", &info);
            info
        }
        Ok(None) => {
            *pending(app) = None;
            UpdateInfo { current, ..Default::default() }
        }
        Err(e) => UpdateInfo { current, error: Some(e.to_string()), ..Default::default() },
    }
}

pub async fn install(app: &AppHandle) -> Result<(), String> {
    let found = pending(app).take();
    let update = match found {
        Some(u) => u,
        None => app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?.ok_or("Cownter is up to date.")?,
    };
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| format!("could not install the update: {e}"))?;
    app.restart();
}

/// Checks GitHub shortly after start and then every few hours; release builds only.
pub fn start(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        let mut told: Option<String> = None;
        loop {
            let on = app.try_state::<AppState>().map(|s| s.display().auto_update).unwrap_or(true);
            if on {
                if let Some(v) = check(&app).await.available {
                    if told.as_deref() != Some(v.as_str()) {
                        let _ = app.notification().builder().title("Cownter").body(format!("Version {v} is ready. Open Settings → General to install it.")).show();
                        told = Some(v);
                    }
                }
            }
            tokio::time::sleep(EVERY).await;
        }
    });
}
