use crate::display::DisplayConfig;
use crate::push::PushInfo;
use crate::state::AppState;
use crate::{hotkeys, tray, windows};
use base64::Engine;
use cownter_core::formula;
use cownter_core::push;
use cownter_core::model::*;
use cownter_core::recap::{self, Recap};
use cownter_core::rules;
use cownter_core::scheduler::{day_start_ms, now_ms};
use cownter_core::secrets::{LayeredSecrets, MemorySecrets, SecretStore};
use cownter_core::sources::TestResult;
use cownter_core::store::DAY_MS;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

type R<T> = Result<T, String>;

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

fn valid_secret_name(n: &str) -> bool {
    !n.is_empty() && n.len() <= 80 && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[tauri::command]
pub fn list_entries(state: State<AppState>) -> R<Vec<EntryView>> {
    let (now, day) = (now_ms(), day_start_ms());
    state
        .store
        .entries()
        .map_err(e)?
        .into_iter()
        .map(|en| rules::build_view(&state.store, en, now, day).map_err(e))
        .collect()
}

fn formulas_using(state: &AppState, slug: &str, except: i64) -> R<Vec<String>> {
    Ok(state
        .store
        .entries()
        .map_err(e)?
        .into_iter()
        .filter(|x| x.id != except)
        .filter(|x| match &x.source {
            SourceCfg::Formula { expr } => {
                formula::parse(expr).map(|a| formula::refs(&a).iter().any(|r| r == slug)).unwrap_or(false)
            }
            _ => false,
        })
        .map(|x| x.name)
        .collect())
}

#[tauri::command]
pub fn save_entry(app: AppHandle, state: State<AppState>, mut entry: Entry, secrets: Vec<(String, String)>) -> R<Entry> {
    entry.slug = entry.slug.trim().to_string();
    if entry.sounds.pack.as_deref().is_some_and(|p| !crate::display::SOUND_PACKS.contains(&p)) {
        entry.sounds.pack = None;
    }
    if let SourceCfg::Formula { expr } = &entry.source {
        rules::validate_formula(&state.store, &entry.slug, expr)?;
        entry.interval_s = entry.interval_s.max(10);
    }
    let old = if entry.id > 0 { state.store.entry(entry.id).map_err(e)? } else { None };
    if entry.id > 0 && old.is_none() {
        return Err("this entry no longer exists".into());
    }
    if let Some(o) = &old {
        if o.slug != entry.slug {
            let users = formulas_using(&state, &o.slug, o.id)?;
            if !users.is_empty() {
                return Err(format!("`{}` is used by {}; update those formulas first", o.slug, users.join(", ")));
            }
        }
    }
    let wanted = entry.source.secret_names();
    for n in &wanted {
        if !valid_secret_name(n) {
            return Err("invalid secret name".into());
        }
    }
    let given: Vec<&(String, String)> = secrets.iter().filter(|(n, v)| wanted.contains(n) && !v.trim().is_empty()).collect();
    if !given.is_empty() && !state.secrets.available() {
        return Err("the system keyring is not available, so keys cannot be saved safely".into());
    }
    for n in &wanted {
        let provided = given.iter().any(|(gn, _)| gn == n);
        if !provided && state.secrets.get(n).ok().flatten().is_none() {
            return Err("enter the key or token for this source".into());
        }
    }
    for (n, v) in &given {
        state.secrets.set(n, v.trim()).map_err(e)?;
    }
    let id = if let Some(o) = &old {
        state.store.update_entry(&entry).map_err(e)?;
        for n in o.source.secret_names() {
            if !wanted.contains(&n) {
                let _ = state.secrets.delete(&n);
            }
        }
        o.id
    } else {
        entry.sort = state.store.entries().map_err(e)?.len() as i64;
        state.store.insert_entry(&entry, now_ms()).map_err(e)?
    };
    state.scheduler.reload(id);
    let saved = state.store.entry(id).map_err(e)?.ok_or("entry not found")?;
    if let Ok(view) = rules::build_view(&state.store, saved.clone(), now_ms(), day_start_ms()) {
        let _ = app.emit("core-event", CoreEvent::EntryUpdated { view });
    }
    crate::sync::changed(&app);
    Ok(saved)
}

#[tauri::command]
pub fn delete_entry(app: AppHandle, state: State<AppState>, id: i64) -> R<()> {
    let Some(en) = state.store.entry(id).map_err(e)? else { return Ok(()) };
    let users = formulas_using(&state, &en.slug, id)?;
    if !users.is_empty() {
        return Err(format!("used by {}; remove it from those formulas first", users.join(", ")));
    }
    state.scheduler.remove(id);
    state.store.delete_entry(id).map_err(e)?;
    for n in en.source.secret_names() {
        let _ = state.secrets.delete(&n);
    }
    crate::sync::changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_entries(app: AppHandle, state: State<AppState>, ids: Vec<i64>) -> R<()> {
    state.store.reorder_entries(&ids).map_err(e)?;
    let _ = app.emit("entries-reordered", ids);
    crate::sync::changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_groups(state: State<AppState>) -> R<Vec<Group>> {
    state.store.groups().map_err(e)
}

#[tauri::command]
pub fn save_group(app: AppHandle, state: State<AppState>, group: Group) -> R<Group> {
    let id = state.store.upsert_group(&group).map_err(e)?;
    crate::sync::changed(&app);
    Ok(Group { id, name: group.name.trim().to_string(), sort: group.sort })
}

#[tauri::command]
pub fn delete_group(app: AppHandle, state: State<AppState>, id: i64) -> R<()> {
    state.store.delete_group(id).map_err(e)?;
    crate::sync::changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_alerts(state: State<AppState>, entry_id: i64) -> R<Vec<Alert>> {
    state.store.alerts_for(entry_id).map_err(e)
}

#[tauri::command]
pub fn save_alert(app: AppHandle, state: State<AppState>, alert: Alert) -> R<Alert> {
    let id = state.store.upsert_alert(&alert).map_err(e)?;
    crate::sync::changed(&app);
    Ok(Alert { id, ..alert })
}

#[tauri::command]
pub fn delete_alert(app: AppHandle, state: State<AppState>, id: i64) -> R<()> {
    state.store.delete_alert(id).map_err(e)?;
    crate::sync::changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn test_source(state: State<'_, AppState>, source: SourceCfg, secrets: Vec<(String, String)>) -> R<TestResult> {
    let top = MemorySecrets::new();
    for (n, v) in secrets.iter().filter(|(_, v)| !v.trim().is_empty()) {
        let _ = top.set(n, v.trim());
    }
    let layered: Arc<dyn SecretStore> = Arc::new(LayeredSecrets { top, base: state.secrets.clone() });
    let f = state.fetcher.with_secrets(layered);
    match &source {
        SourceCfg::CustomUrl { .. } => f.test_url(&source).await.map_err(e),
        SourceCfg::Formula { .. } => Err("formulas are checked when saved".into()),
        SourceCfg::Push => Err("push numbers get their value from outside; see General for the address".into()),
        _ => {
            let started = Instant::now();
            let v = f.fetch(&source, chrono::Local::now()).await.map_err(e)?;
            Ok(TestResult {
                status: 200,
                elapsed_ms: started.elapsed().as_millis() as u64,
                content_type: None,
                body: String::new(),
                truncated: false,
                value: Some(v),
                error: None,
                text: None,
            })
        }
    }
}

/// Downloads `/favicon.ico` (or the given image URL) and returns it as a data URL.
#[tauri::command]
pub async fn fetch_favicon(url: String) -> R<String> {
    let mut u = reqwest::Url::parse(url.trim()).map_err(|_| "the URL is not valid".to_string())?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err("the URL must start with http:// or https://".into());
    }
    let looks_like_image = [".png", ".ico", ".svg", ".jpg", ".jpeg", ".webp", ".gif"]
        .iter()
        .any(|ext| u.path().to_ascii_lowercase().ends_with(ext));
    if !looks_like_image {
        u.set_path("/favicon.ico");
        u.set_query(None);
    }
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(e)?;
    let resp = client.get(u).send().await.map_err(e)?;
    if !resp.status().is_success() {
        return Err(format!("no icon found (HTTP {})", resp.status().as_u16()));
    }
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/x-icon")
        .split(';')
        .next()
        .unwrap_or("image/x-icon")
        .trim()
        .to_string();
    if !ct.starts_with("image/") {
        return Err("the site did not return an image".into());
    }
    let bytes = resp.bytes().await.map_err(e)?;
    if bytes.len() > 512 * 1024 {
        return Err("the icon is larger than 512 KB".into());
    }
    Ok(format!("data:{ct};base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)))
}

#[tauri::command]
pub fn entry_history(state: State<AppState>, entry_id: i64, days: u32) -> R<Vec<(i64, f64)>> {
    let now = now_ms();
    state.store.spark(entry_id, now - days.clamp(1, 3650) as i64 * DAY_MS, now, 120).map_err(e)
}

#[tauri::command]
pub fn weekly_recap(state: State<AppState>) -> R<Recap> {
    recap::weekly_recap(&state.store, now_ms(), day_start_ms()).map_err(e)
}

#[tauri::command]
pub fn show_recap(app: AppHandle) -> R<()> {
    windows::show_recap(&app).map_err(e)
}

/// Saves a PNG snapshot to the Pictures folder and returns the file path.
#[tauri::command]
pub fn save_snapshot(app: AppHandle, name: String, png: String) -> R<String> {
    let bytes = crate::snapshot::decode(&png)?;
    let dir = app.path().picture_dir().or_else(|_| app.path().home_dir().map(|h| h.join("Pictures"))).map_err(e)?;
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let path = crate::snapshot::write(&dir, &name, &date, "png", &bytes)?;
    Ok(path.display().to_string())
}

/// Writes the history of one entry, or of all entries, as CSV to the Downloads folder and returns the file path.
#[tauri::command]
pub fn export_csv(app: AppHandle, state: State<AppState>, entry_id: Option<i64>) -> R<String> {
    let entries = match entry_id {
        Some(id) => vec![state.store.entry(id).map_err(e)?.ok_or("this number no longer exists")?],
        None => state.store.entries().map_err(e)?,
    };
    let csv = cownter_core::export::entries_csv(&state.store, &entries).map_err(e)?;
    let label = if entry_id.is_some() { entries[0].name.as_str() } else { "all" };
    let dir = app.path().download_dir().or_else(|_| app.path().home_dir()).map_err(e)?;
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let path = crate::snapshot::write(&dir, label, &date, "csv", csv.as_bytes())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn simulate_change(state: State<AppState>, entry_id: i64, value: f64) -> R<()> {
    if !value.is_finite() {
        return Err("value must be a number".into());
    }
    state.scheduler.simulate(entry_id, value)
}

#[tauri::command]
pub fn keyring_available(state: State<AppState>) -> bool {
    state.secrets.available()
}

#[tauri::command]
pub fn has_secrets(state: State<AppState>, names: Vec<String>) -> Vec<bool> {
    names
        .iter()
        .map(|n| valid_secret_name(n) && state.secrets.get(n).ok().flatten().is_some())
        .collect()
}

#[tauri::command]
pub fn push_info(state: State<AppState>) -> PushInfo {
    state.push.info()
}

#[tauri::command]
pub fn regenerate_push_token(app: AppHandle, state: State<AppState>) -> R<PushInfo> {
    let token = push::new_token();
    push::save_token(&*state.secrets, &state.store, &token)?;
    state.push.set_token(token);
    let info = state.push.info();
    let _ = app.emit("push-changed", &info);
    Ok(info)
}

#[tauri::command]
pub fn restart_push(app: AppHandle, state: State<AppState>) -> PushInfo {
    crate::push::start(&app, state.display().push_port, true).unwrap_or_else(|| state.push.info())
}

#[tauri::command]
pub fn get_display(state: State<AppState>) -> DisplayConfig {
    state.display()
}

#[tauri::command]
pub fn save_display(app: AppHandle, state: State<AppState>, cfg: DisplayConfig) -> R<DisplayConfig> {
    let cfg = apply_display(&app, &state, cfg)?;
    crate::sync::changed(&app);
    Ok(cfg)
}

/// Saves the display config and updates every window, the tray, hotkeys and the push endpoint.
pub(crate) fn apply_display(app: &AppHandle, state: &AppState, cfg: DisplayConfig) -> R<DisplayConfig> {
    let cfg = cfg.sanitized();
    let old = state.display();
    hotkeys::register(app, &cfg)?;
    if old.launch_at_login != cfg.launch_at_login {
        crate::set_autostart(app, cfg.launch_at_login)?;
    }
    cfg.save(&state.store)?;
    state.set_display(cfg.clone());
    windows::sync(app, &cfg);
    if old.hidden != cfg.hidden {
        windows::set_hidden(app, &cfg);
    }
    windows::replace_all(app, &cfg);
    crate::push::start(app, cfg.push_port, false);
    crate::refresh_tray(app);
    let _ = app.emit("display-changed", &cfg);
    Ok(cfg)
}

#[tauri::command]
pub fn sync_info(state: State<AppState>) -> crate::sync::SyncInfo {
    crate::sync::info(&state.store)
}

#[tauri::command]
pub fn set_sync_folder(app: AppHandle, state: State<AppState>, folder: Option<String>) -> R<crate::sync::SyncInfo> {
    crate::sync::set_folder(&state.store, folder.as_deref())?;
    crate::sync::set_error(&state.store, None);
    let info = crate::sync::info(&state.store);
    if info.pending.is_none() {
        crate::sync::export(&app)?;
    }
    Ok(crate::sync::info(&state.store))
}

#[tauri::command]
pub fn sync_export_now(app: AppHandle, state: State<AppState>) -> R<crate::sync::SyncInfo> {
    if crate::sync::folder(&state.store).is_none() {
        return Err("choose a sync folder first".into());
    }
    crate::sync::export(&app)?;
    crate::sync::mark_exported(&state.store, now_ms());
    Ok(crate::sync::info(&state.store))
}

#[tauri::command]
pub fn sync_import_now(app: AppHandle) -> R<crate::sync::ImportReport> {
    crate::sync::import(&app, false)?.ok_or_else(|| "nothing to import".into())
}

#[tauri::command]
pub fn sync_dismiss_notice(state: State<AppState>) {
    crate::sync::clear_notice(&state.store);
}

#[tauri::command]
pub fn widget_resize(app: AppHandle, window: WebviewWindow, state: State<AppState>, width: f64, height: f64, show: bool) -> R<()> {
    if !width.is_finite() || !height.is_finite() {
        return Err("invalid size".into());
    }
    windows::place(&app, &state.display(), window.label(), width, height, show)
}

#[tauri::command]
pub fn widget_hit(window: WebviewWindow, rect: crate::hit::Rect) -> R<()> {
    if window.label() == "settings" {
        return Ok(());
    }
    crate::hit::set(&window, rect)
}

#[tauri::command]
pub fn widget_done(window: WebviewWindow) -> R<()> {
    if window.label() == "settings" {
        return Ok(());
    }
    window.hide().map_err(e)
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> R<()> {
    windows::open_settings(&app).map_err(e)
}

#[tauri::command]
pub fn refresh_all(state: State<AppState>) {
    state.scheduler.refresh_all();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    os: &'static str,
    version: &'static str,
}

#[tauri::command]
pub fn platform_info() -> PlatformInfo {
    PlatformInfo { os: std::env::consts::OS, version: env!("CARGO_PKG_VERSION") }
}

#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Vec<String> {
    windows::monitors(&app)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

pub fn tray_update_for(app: &AppHandle, state: &AppState, view: &EntryView) {
    let cfg = state.display();
    if cfg.tray.entry_id == Some(view.entry.id) {
        tray::update(app, Some(view), cfg.tray.show_number);
    }
}
