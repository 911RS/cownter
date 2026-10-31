mod commands;
mod display;
mod hotkeys;
mod pixfont;
mod state;
mod tray;
mod windows;

use cownter_core::model::CoreEvent;
use cownter_core::rules;
use cownter_core::scheduler::{Scheduler, day_start_ms, now_ms};
use cownter_core::secrets::{KeyringSecrets, SecretStore};
use cownter_core::sources::Fetcher;
use cownter_core::store::Store;
use display::DisplayConfig;
use state::AppState;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_notification::NotificationExt;

pub(crate) fn set_autostart(app: &AppHandle, on: bool) -> Result<(), String> {
    let al = app.autolaunch();
    let res = if on { al.enable() } else { al.disable() };
    res.map_err(|e| format!("could not change launch at login: {e}"))
}

pub(crate) fn toggle_hidden(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let mut cfg = state.display();
    cfg.hidden = !cfg.hidden;
    let _ = cfg.save(&state.store);
    state.set_display(cfg.clone());
    windows::set_hidden(app, &cfg);
    let _ = app.emit("display-changed", &cfg);
}

pub(crate) fn refresh_tray(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let cfg = state.display();
    let view = cfg.tray.entry_id.and_then(|id| {
        let e = state.store.entry(id).ok().flatten()?;
        rules::build_view(&state.store, e, now_ms(), day_start_ms()).ok()
    });
    tray::update(app, view.as_ref(), cfg.tray.show_number);
}

fn notify(app: &AppHandle, body: &str) {
    let _ = app.notification().builder().title("Cownter").body(body).show();
}

fn forward_events(app: AppHandle, mut rx: tokio::sync::mpsc::UnboundedReceiver<CoreEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match &ev {
                CoreEvent::EntryUpdated { view } => {
                    if let Some(state) = app.try_state::<AppState>() {
                        commands::tray_update_for(&app, &state, view);
                    }
                }
                CoreEvent::AlertFired { message, notify: true, .. } | CoreEvent::AuthFailed { message, .. } => notify(&app, message),
                _ => {}
            }
            let _ = app.emit("core-event", &ev);
        }
    });
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let dir = app.path().app_data_dir()?;
    let store = Arc::new(Store::open(&dir.join("cownter.db"))?);
    let secrets: Arc<dyn SecretStore> = Arc::new(KeyringSecrets::new());
    let fetcher = Arc::new(Fetcher::new(secrets.clone()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let scheduler = tauri::async_runtime::block_on({
        let (store, fetcher) = (store.clone(), fetcher.clone());
        async move { Scheduler::start(store, fetcher, tx) }
    });
    let display = DisplayConfig::load(&store).sanitized();
    app.manage(AppState { store: store.clone(), scheduler, secrets, fetcher, display: Mutex::new(display.clone()) });
    forward_events(handle.clone(), rx);
    tray::build(&handle)?;
    if let Err(e) = hotkeys::register(&handle, &display) {
        eprintln!("{e}");
    }
    windows::sync(&handle, &display);
    refresh_tray(&handle);
    let later = handle.clone();
    tauri::async_runtime::spawn(async move {
        for secs in [2, 5, 15] {
            tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
            tray::clear_title(&later);
            refresh_tray(&later);
        }
    });
    if !display.onboarded || store.entries()?.is_empty() {
        windows::open_settings(&handle)?;
    }
    Ok(())
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = windows::open_settings(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::get_display,
            commands::widget_resize,
            commands::widget_done,
            commands::open_settings,
            commands::refresh_all,
            commands::save_entry,
            commands::delete_entry,
            commands::reorder_entries,
            commands::list_groups,
            commands::save_group,
            commands::delete_group,
            commands::list_alerts,
            commands::save_alert,
            commands::delete_alert,
            commands::test_source,
            commands::fetch_favicon,
            commands::entry_history,
            commands::simulate_change,
            commands::keyring_available,
            commands::has_secrets,
            commands::save_display,
            commands::platform_info,
            commands::list_monitors,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Cownter");
    app.run(|_, ev| {
        if let RunEvent::ExitRequested { code: None, api, .. } = ev {
            api.prevent_exit();
        }
    });
}
