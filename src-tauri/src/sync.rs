use crate::display::DisplayConfig;
use cownter_core::model::{Alert, Entry, Group};
use cownter_core::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

pub const FILE_NAME: &str = "cownter-settings.json";
const FORMAT: u32 = 1;
const FOLDER_KEY: &str = "sync_folder";
const MACHINE_KEY: &str = "machine_id";
const APPLIED_KEY: &str = "sync_applied_at";
const EXPORTED_KEY: &str = "sync_exported_at";
const NOTICE_KEY: &str = "sync_notice";
const ERROR_KEY: &str = "sync_error";

/// Everything that is shared between machines. Keys and tokens stay in the local keyring;
/// sources only carry the keyring names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsFile {
    pub format: u32,
    pub machine_id: String,
    pub hostname: String,
    pub written_at: i64,
    pub entries: Vec<Entry>,
    pub groups: Vec<Group>,
    pub alerts: Vec<Alert>,
    pub display: DisplayConfig,
}

impl SettingsFile {
    fn same_content(&self, other: &SettingsFile) -> bool {
        self.entries == other.entries && self.groups == other.groups && self.alerts == other.alerts && self.display == other.display
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub from: String,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncInfo {
    pub folder: Option<String>,
    pub suggested: String,
    pub file: Option<String>,
    pub machine_id: String,
    pub hostname: String,
    pub exported_at: Option<i64>,
    pub applied_at: Option<i64>,
    pub notice: Option<Notice>,
    /// A newer file from another machine that has not been imported yet.
    pub pending: Option<Notice>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub from: String,
    pub entries: usize,
    pub groups: usize,
    pub alerts: usize,
    pub skipped: Vec<String>,
}

/// Fields that describe this machine only and are never shared.
fn strip_local(mut d: DisplayConfig) -> DisplayConfig {
    let base = DisplayConfig::default();
    d.launch_at_login = base.launch_at_login;
    d.hidden = base.hidden;
    d.onboarded = true;
    d
}

fn keep_local(mut incoming: DisplayConfig, local: &DisplayConfig) -> DisplayConfig {
    incoming.launch_at_login = local.launch_at_login;
    incoming.hidden = local.hidden;
    incoming.onboarded = local.onboarded || incoming.onboarded;
    incoming.sanitized()
}

pub fn machine_id(store: &Store) -> String {
    if let Some(id) = store.setting(MACHINE_KEY).ok().flatten().filter(|s| !s.is_empty()) {
        return id;
    }
    let id = format!("{:016x}", rand::random::<u64>());
    let _ = store.set_setting(MACHINE_KEY, &id);
    id
}

pub fn hostname() -> String {
    let from_env = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).ok();
    let from_file = || std::fs::read_to_string("/etc/hostname").ok();
    let from_cmd = || {
        std::process::Command::new("hostname")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    from_env
        .or_else(from_file)
        .or_else(from_cmd)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "another computer".into())
}

pub fn suggested_folder() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join("Sync").join("Cownter-settings")
}

pub fn folder(store: &Store) -> Option<PathBuf> {
    store.setting(FOLDER_KEY).ok().flatten().filter(|s| !s.is_empty()).map(PathBuf::from)
}

/// Sets or clears the sync folder; creates it when missing.
pub fn set_folder(store: &Store, path: Option<&str>) -> Result<(), String> {
    let Some(p) = path.map(str::trim).filter(|p| !p.is_empty()) else {
        return store.delete_setting(FOLDER_KEY).map_err(|e| e.to_string());
    };
    let p = match p.strip_prefix("~") {
        Some(rest) => dirs::home_dir().ok_or("no home folder")?.join(rest.trim_start_matches(['/', '\\'])),
        None => PathBuf::from(p),
    };
    if !p.is_absolute() {
        return Err("use a full folder path".into());
    }
    std::fs::create_dir_all(&p).map_err(|e| format!("could not create the folder: {e}"))?;
    store.set_setting(FOLDER_KEY, &p.display().to_string()).map_err(|e| e.to_string())
}

pub fn snapshot(store: &Store, display: &DisplayConfig, machine_id: &str, hostname: &str, now: i64) -> Result<SettingsFile, String> {
    let entries = store.entries().map_err(|e| e.to_string())?;
    let mut alerts = vec![];
    for en in &entries {
        for a in store.alerts_for(en.id).map_err(|e| e.to_string())? {
            alerts.push(Alert { last_fired_at: None, ..a });
        }
    }
    Ok(SettingsFile {
        format: FORMAT,
        machine_id: machine_id.into(),
        hostname: hostname.into(),
        written_at: now,
        entries,
        groups: store.groups().map_err(|e| e.to_string())?,
        alerts,
        display: strip_local(display.clone()),
    })
}

pub fn read(dir: &Path) -> Result<Option<SettingsFile>, String> {
    let path = dir.join(FILE_NAME);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    let f: SettingsFile = serde_json::from_str(&text).map_err(|e| format!("{FILE_NAME} is not a Cownter settings file: {e}"))?;
    if f.format > FORMAT {
        return Err("the settings file was written by a newer Cownter; update this one first".into());
    }
    Ok(Some(f))
}

/// Writes the file unless it already holds the same settings. Returns true when written.
pub fn write(dir: &Path, f: &SettingsFile) -> Result<bool, String> {
    if let Ok(Some(old)) = read(dir) {
        if old.same_content(f) {
            return Ok(false);
        }
    }
    let json = serde_json::to_string_pretty(f).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!(".{FILE_NAME}.tmp"));
    std::fs::write(&tmp, json).map_err(|e| format!("could not write the settings file: {e}"))?;
    std::fs::rename(&tmp, dir.join(FILE_NAME)).map_err(|e| format!("could not write the settings file: {e}"))?;
    Ok(true)
}

/// Whether a file found on start should be applied automatically.
pub fn is_new(f: &SettingsFile, my_id: &str, applied_at: Option<i64>) -> bool {
    f.machine_id != my_id && applied_at.is_none_or(|t| f.written_at > t)
}

fn unique_slug(slug: &str, taken: &HashSet<String>) -> String {
    (2..)
        .map(|n| {
            let suffix = format!("_{n}");
            let keep = 40usize.saturating_sub(suffix.len()).min(slug.len());
            format!("{}{suffix}", &slug[..keep])
        })
        .find(|s| !taken.contains(s))
        .unwrap_or_else(|| slug.into())
}

/// Merges the file into the store by id and returns the display config to use and the entry ids that changed.
pub fn apply(store: &Store, f: &SettingsFile, local: &DisplayConfig, now: i64) -> Result<(DisplayConfig, Vec<i64>, ImportReport), String> {
    let mut report = ImportReport { from: f.hostname.clone(), entries: 0, groups: 0, alerts: 0, skipped: vec![] };
    for g in &f.groups {
        match store.put_group(g) {
            Ok(()) => report.groups += 1,
            Err(e) => report.skipped.push(format!("group {}: {e}", g.name)),
        }
    }
    let groups: HashSet<i64> = store.groups().map_err(|e| e.to_string())?.into_iter().map(|g| g.id).collect();
    let mut changed = vec![];
    for en in &f.entries {
        let others = store.entries().map_err(|e| e.to_string())?;
        let taken: HashSet<String> = others.iter().filter(|o| o.id != en.id).map(|o| o.slug.clone()).collect();
        let mut e = en.clone();
        if taken.contains(&e.slug) {
            e.slug = unique_slug(&e.slug, &taken);
        }
        if e.group_id.is_some_and(|g| !groups.contains(&g)) {
            e.group_id = None;
        }
        if store.entry(e.id).map_err(|x| x.to_string())?.as_ref() == Some(&e) {
            continue;
        }
        match store.put_entry(&e, now) {
            Ok(()) => {
                report.entries += 1;
                changed.push(e.id);
            }
            Err(x) => report.skipped.push(format!("{}: {x}", e.name)),
        }
    }
    for a in &f.alerts {
        if store.entry(a.entry_id).map_err(|e| e.to_string())?.is_none() {
            continue;
        }
        let same = store.alerts_for(a.entry_id).map_err(|e| e.to_string())?.into_iter().any(|x| Alert { last_fired_at: None, ..x } == *a);
        if same {
            continue;
        }
        match store.put_alert(a) {
            Ok(()) => report.alerts += 1,
            Err(e) => report.skipped.push(format!("alert {}: {e}", a.id)),
        }
    }
    Ok((keep_local(f.display.clone(), local), changed, report))
}

pub fn applied_at(store: &Store) -> Option<i64> {
    store.setting(APPLIED_KEY).ok().flatten().and_then(|s| s.parse().ok())
}

pub fn mark_applied(store: &Store, f: &SettingsFile, now: i64, auto: bool) {
    let _ = store.set_setting(APPLIED_KEY, &f.written_at.to_string());
    if auto {
        let n = Notice { from: f.hostname.clone(), at: now };
        let _ = store.set_setting(NOTICE_KEY, &serde_json::to_string(&n).unwrap_or_default());
    }
}

pub fn mark_exported(store: &Store, now: i64) {
    let _ = store.set_setting(EXPORTED_KEY, &now.to_string());
}

pub fn set_error(store: &Store, err: Option<&str>) {
    let _ = match err {
        Some(e) => store.set_setting(ERROR_KEY, e),
        None => store.delete_setting(ERROR_KEY),
    };
}

pub fn clear_notice(store: &Store) {
    let _ = store.delete_setting(NOTICE_KEY);
}

pub fn info(store: &Store) -> SyncInfo {
    let dir = folder(store);
    let get = |k: &str| store.setting(k).ok().flatten();
    let me = machine_id(store);
    let pending = dir
        .as_deref()
        .and_then(|d| read(d).ok().flatten())
        .filter(|f| is_new(f, &me, applied_at(store)))
        .map(|f| Notice { from: f.hostname, at: f.written_at });
    SyncInfo {
        folder: dir.as_ref().map(|d| d.display().to_string()),
        suggested: suggested_folder().display().to_string(),
        file: dir.map(|d| d.join(FILE_NAME).display().to_string()),
        machine_id: me,
        hostname: hostname(),
        exported_at: get(EXPORTED_KEY).and_then(|s| s.parse().ok()),
        applied_at: applied_at(store),
        notice: get(NOTICE_KEY).and_then(|s| serde_json::from_str(&s).ok()),
        pending,
        error: get(ERROR_KEY),
    }
}

/// Writes the settings file when a sync folder is set.
pub fn export(app: &AppHandle) -> Result<bool, String> {
    let Some(state) = app.try_state::<crate::state::AppState>() else { return Ok(false) };
    let Some(dir) = folder(&state.store) else { return Ok(false) };
    let now = cownter_core::scheduler::now_ms();
    let res = std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create the folder: {e}"))
        .and_then(|_| snapshot(&state.store, &state.display(), &machine_id(&state.store), &hostname(), now))
        .and_then(|f| write(&dir, &f));
    set_error(&state.store, res.as_ref().err().map(String::as_str));
    if res == Ok(true) {
        mark_exported(&state.store, now);
    }
    res
}

/// Call after any settings change.
pub fn changed(app: &AppHandle) {
    if let Err(e) = export(app) {
        eprintln!("settings sync: {e}");
    }
}

/// Reads the settings file and merges it in. With `auto`, only newer files from another machine are applied.
pub fn import(app: &AppHandle, auto: bool) -> Result<Option<ImportReport>, String> {
    let state = app.try_state::<crate::state::AppState>().ok_or("not ready")?;
    let dir = folder(&state.store).ok_or("choose a sync folder first")?;
    let Some(f) = read(&dir)? else {
        return if auto { Ok(None) } else { Err(format!("there is no {FILE_NAME} in that folder yet")) };
    };
    if auto && !is_new(&f, &machine_id(&state.store), applied_at(&state.store)) {
        return Ok(None);
    }
    let now = cownter_core::scheduler::now_ms();
    let (display, changed, report) = apply(&state.store, &f, &state.display(), now)?;
    mark_applied(&state.store, &f, now, auto);
    for id in &changed {
        state.scheduler.reload(*id);
        if let Ok(Some(en)) = state.store.entry(*id) {
            if let Ok(view) = cownter_core::rules::build_view(&state.store, en, now, cownter_core::scheduler::day_start_ms()) {
                let _ = app.emit("core-event", cownter_core::model::CoreEvent::EntryUpdated { view });
            }
        }
    }
    let ids: Vec<i64> = state.store.entries().map(|v| v.into_iter().map(|e| e.id).collect()).unwrap_or_default();
    let _ = app.emit("entries-reordered", ids);
    crate::commands::apply_display(app, &state, display)?;
    let _ = app.emit("settings-synced", &report);
    Ok(Some(report))
}

/// Applies a newer settings file from another machine on start.
pub fn check_on_start(app: &AppHandle) {
    match import(app, true) {
        Ok(Some(r)) => crate::notify(app, &format!("Settings updated from {}", r.from)),
        Ok(None) => {}
        Err(e) => {
            if let Some(state) = app.try_state::<crate::state::AppState>() {
                set_error(&state.store, Some(&e));
            }
            eprintln!("settings sync: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cownter_core::model::*;
    use cownter_core::secrets::{MemorySecrets, SecretStore};

    fn entry(id: i64, slug: &str, source: SourceCfg) -> Entry {
        Entry {
            id,
            slug: slug.into(),
            name: slug.into(),
            icon: Icon::default(),
            group_id: None,
            source,
            interval_s: 60,
            format: NumberFormat::default(),
            goal_value: Some(100.0),
            celebrate: true,
            sounds: Sounds { on_change: Some("chime".into()), on_goal: None, pack: Some("cowbell".into()) },
            sort: 0,
            text_mode: false,
            goal_text: None,
        }
    }

    fn alert(id: i64, entry_id: i64) -> Alert {
        Alert { id, entry_id, kind: AlertKind::Above, threshold: 50.0, window_s: 3600, enabled: true, last_fired_at: Some(123), notify: true, effect: "confetti".into() }
    }

    fn filled() -> (Store, MemorySecrets) {
        let s = Store::open_memory().unwrap();
        let secrets = MemorySecrets::new();
        s.put_group(&Group { id: 3, name: "Shop".into(), sort: 0 }).unwrap();
        let mut stripe = entry(5, "revenue", SourceCfg::Stripe { api_key: "e5-apiKey".into(), currency: "EUR".into() });
        stripe.group_id = Some(3);
        s.put_entry(&stripe, 0).unwrap();
        let url = SourceCfg::CustomUrl {
            url: "https://example.com/api".into(),
            method: "GET".into(),
            headers: vec![Header { name: "Authorization".into(), value: SecretOrText::Secret("e8-h0".into()) }],
            body: None,
            pick: Pick::FirstNumber,
        };
        s.put_entry(&entry(8, "orders", url), 0).unwrap();
        s.put_alert(&alert(11, 5)).unwrap();
        secrets.set("e5-apiKey", "sk_live_TOPSECRET").unwrap();
        secrets.set("e8-h0", "Bearer HEADERSECRET").unwrap();
        s.set_setting("push_token", "PUSHTOKENSECRET").unwrap();
        (s, secrets)
    }

    #[test]
    fn round_trip_into_an_empty_store() {
        let (a, _) = filled();
        let mut disp = DisplayConfig::default();
        disp.sound_pack = "arcade".into();
        disp.island.tint = "ice".into();
        disp.launch_at_login = true;
        let file = snapshot(&a, &disp, "aaa", "desk", 1000).unwrap();
        let json = serde_json::to_string(&file).unwrap();
        let back: SettingsFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, file);
        assert!(!back.display.launch_at_login);
        assert_eq!(back.alerts[0].last_fired_at, None);

        let b = Store::open_memory().unwrap();
        let local = DisplayConfig { hidden: true, ..DisplayConfig::default() };
        let (d, changed, report) = apply(&b, &back, &local, 2000).unwrap();
        assert_eq!(changed, vec![5, 8]);
        assert_eq!((report.entries, report.groups, report.alerts, report.from.as_str()), (2, 1, 1, "desk"));
        assert!(report.skipped.is_empty());
        assert_eq!(b.entries().unwrap(), a.entries().unwrap());
        assert_eq!(b.groups().unwrap(), a.groups().unwrap());
        assert_eq!(b.alerts_for(5).unwrap()[0].effect, "confetti");
        assert_eq!(d.sound_pack, "arcade");
        assert_eq!(d.island.tint, "ice");
        assert!(d.hidden);
        assert!(!d.launch_at_login);

        let (_, again, r2) = apply(&b, &back, &local, 3000).unwrap();
        assert!(again.is_empty());
        assert_eq!((r2.entries, r2.alerts), (0, 0));
    }

    #[test]
    fn secrets_never_leave_the_machine() {
        let (s, secrets) = filled();
        let file = snapshot(&s, &DisplayConfig::default(), "aaa", "desk", 1).unwrap();
        let json = serde_json::to_string_pretty(&file).unwrap();
        for secret in ["sk_live_TOPSECRET", "HEADERSECRET", "PUSHTOKENSECRET"] {
            assert!(!json.contains(secret), "{secret} leaked");
        }
        assert!(json.contains("e5-apiKey"));
        assert_eq!(secrets.get("e5-apiKey").unwrap().as_deref(), Some("sk_live_TOPSECRET"));
    }

    #[test]
    fn merges_by_id_and_keeps_local_entries() {
        let (a, _) = filled();
        let file = snapshot(&a, &DisplayConfig::default(), "aaa", "desk", 1).unwrap();
        let b = Store::open_memory().unwrap();
        b.put_entry(&entry(8, "old_orders", SourceCfg::Push), 0).unwrap();
        b.put_entry(&entry(20, "revenue", SourceCfg::Push), 0).unwrap();
        let (_, changed, _) = apply(&b, &file, &DisplayConfig::default(), 5).unwrap();
        assert_eq!(changed, vec![5, 8]);
        assert_eq!(b.entry(8).unwrap().unwrap().slug, "orders");
        assert_eq!(b.entry(5).unwrap().unwrap().slug, "revenue_2");
        assert_eq!(b.entry(20).unwrap().unwrap().slug, "revenue");
        assert_eq!(b.entries().unwrap().len(), 3);
    }

    #[test]
    fn writes_only_when_settings_change() {
        let dir = std::env::temp_dir().join(format!("cownter-sync-{:x}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        let (s, _) = filled();
        let f1 = snapshot(&s, &DisplayConfig::default(), "aaa", "desk", 1).unwrap();
        assert!(write(&dir, &f1).unwrap());
        let f2 = snapshot(&s, &DisplayConfig { hidden: true, ..DisplayConfig::default() }, "aaa", "desk", 2).unwrap();
        assert!(!write(&dir, &f2).unwrap());
        assert_eq!(read(&dir).unwrap().unwrap().written_at, 1);
        let f3 = snapshot(&s, &DisplayConfig { muted: true, ..DisplayConfig::default() }, "aaa", "desk", 3).unwrap();
        assert!(write(&dir, &f3).unwrap());
        assert_eq!(read(&dir).unwrap(), Some(f3));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn imports_only_newer_files_from_other_machines() {
        let (s, _) = filled();
        let f = snapshot(&s, &DisplayConfig::default(), "aaa", "desk", 100).unwrap();
        assert!(is_new(&f, "bbb", None));
        assert!(is_new(&f, "bbb", Some(99)));
        assert!(!is_new(&f, "bbb", Some(100)));
        assert!(!is_new(&f, "aaa", None));
    }

    #[test]
    fn machine_id_is_stable() {
        let s = Store::open_memory().unwrap();
        let id = machine_id(&s);
        assert_eq!(id.len(), 16);
        assert_eq!(machine_id(&s), id);
    }
}
