pub mod cli;
mod commands;
mod display;
mod hit;
mod hotkeys;
mod pixfont;
mod push;
mod snapshot;
mod state;
mod sync;
mod tray;
mod updates;
mod windows;

use cownter_core::model::CoreEvent;
use cownter_core::{push as push_api, recap, rules};
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

/// Rewrites the login entry so it points at the running binary. Debug builds are skipped
/// because they need the dev server and must never replace the installed app's entry.
fn sync_autostart(app: &AppHandle, on: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    let enabled = app.autolaunch().is_enabled().unwrap_or(false);
    if on || enabled {
        if let Err(e) = set_autostart(app, on) {
            eprintln!("{e}");
        }
    }
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

pub(crate) fn notify(app: &AppHandle, body: &str) {
    let _ = app.notification().builder().title("Cownter").body(body).show();
}

/// Shows the weekly recap once per week when it is due and something was tracked.
fn check_recap(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let cfg = state.display();
    if !cfg.weekly_recap {
        return;
    }
    let due = recap::take_due(&state.store, chrono::Local::now().naive_local(), cfg.recap_weekday).unwrap_or(false);
    let any = due && recap::weekly_recap(&state.store, now_ms(), day_start_ms()).is_ok_and(|r| !r.items.is_empty());
    if any {
        if let Err(e) = windows::show_recap(app) {
            eprintln!("recap window: {e}");
        }
    }
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
    let token = push_api::ensure_token(&*secrets, &store).unwrap_or_else(|e| {
        eprintln!("push token: {e}");
        push_api::new_token()
    });
    let push = push::PushServer::new(push_api::Ctx::new(store.clone(), scheduler.clone(), token.clone()), token);
    app.manage(AppState { store: store.clone(), scheduler, secrets, fetcher, display: Mutex::new(display.clone()), push });
    forward_events(handle.clone(), rx);
    push::start(&handle, display.push_port, false);
    tray::build(&handle)?;
    hit::start(&handle);
    updates::start(&handle);
    if let Err(e) = hotkeys::register(&handle, &display) {
        eprintln!("{e}");
    }
    sync_autostart(&handle, display.launch_at_login);
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
    let sync_app = handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        sync::check_on_start(&sync_app);
    });
    let recap_app = handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        loop {
            check_recap(&recap_app);
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::Pending::default())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::get_display,
            commands::widget_resize,
            commands::widget_done,
            commands::widget_hit,
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
            commands::weekly_recap,
            commands::show_recap,
            commands::save_snapshot,
            commands::export_csv,
            commands::keyring_available,
            commands::has_secrets,
            commands::push_info,
            commands::regenerate_push_token,
            commands::restart_push,
            commands::save_display,
            commands::sync_info,
            commands::set_sync_folder,
            commands::sync_export_now,
            commands::sync_import_now,
            commands::sync_dismiss_notice,
            commands::platform_info,
            commands::list_monitors,
            commands::quit_app,
            commands::check_update,
            commands::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Cownter");
    app.run(|_, ev| {
        if let RunEvent::ExitRequested { code: None, api, .. } = ev {
            api.prevent_exit();
        }
    });
}
