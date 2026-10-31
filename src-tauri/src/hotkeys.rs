use crate::display::DisplayConfig;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

fn parse(s: &str, what: &str) -> Result<Option<Shortcut>, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    s.parse::<Shortcut>().map(Some).map_err(|_| format!("the {what} shortcut `{s}` is not valid"))
}

/// Replaces the registered global shortcuts with the ones in `cfg`.
pub fn register(app: &AppHandle, cfg: &DisplayConfig) -> Result<(), String> {
    let toggle = parse(&cfg.hotkey_toggle, "show/hide")?;
    let refresh = parse(&cfg.hotkey_refresh, "refresh")?;
    let peek = parse(&cfg.hotkey_peek, "quick peek")?;
    let set: Vec<&Shortcut> = [&toggle, &refresh, &peek].into_iter().flatten().collect();
    if (1..set.len()).any(|i| set[..i].contains(&set[i])) {
        return Err("each shortcut must be different".into());
    }
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if let Some(t) = toggle {
        gs.on_shortcut(t, |app, _, ev| {
            if ev.state == ShortcutState::Pressed {
                crate::toggle_hidden(app);
            }
        })
        .map_err(|e| format!("could not register the show/hide shortcut: {e}"))?;
    }
    if let Some(r) = refresh {
        gs.on_shortcut(r, |app, _, ev| {
            if ev.state == ShortcutState::Pressed {
                if let Some(s) = app.try_state::<AppState>() {
                    s.scheduler.refresh_all();
                }
            }
        })
        .map_err(|e| format!("could not register the refresh shortcut: {e}"))?;
    }
    if let Some(p) = peek {
        gs.on_shortcut(p, |app, _, ev| {
            if ev.state == ShortcutState::Pressed {
                let _ = app.emit_to("peek", "peek", ());
            }
        })
        .map_err(|e| format!("could not register the quick peek shortcut: {e}"))?;
    }
    Ok(())
}
