use crate::display::{DisplayConfig, EXTRA_FORMS};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Part of a widget window that always stays on screen, in logical pixels.
const KEEP_VISIBLE: f64 = 24.0;
const MARGIN: f64 = 0.0;
/// Forms that only appear for a moment (on a change or a shortcut).
const TRANSIENT: [&str; 3] = ["pop", "peek", "recap"];
/// Forms docked to the left or right screen edge.
const DOCKED: [&str; 2] = ["edge", "drawer"];
/// Forms that touch the screen edge they are anchored to.
const FLUSH: [&str; 8] = ["island", "notch", "wings", "toptabs", "folders", "tag", "clothesline", "deck"];

/// Labels of every form window.
pub fn forms() -> impl Iterator<Item = &'static str> {
    ["island", "edge", "pop"].into_iter().chain(EXTRA_FORMS.iter().map(|(id, _)| *id))
}

fn enabled(cfg: &DisplayConfig, label: &str) -> bool {
    cfg.form(label).map(|f| f.enabled).unwrap_or(true)
}

fn build_widget(app: &AppHandle, label: &str) -> tauri::Result<WebviewWindow> {
    let mut b = WebviewWindowBuilder::new(app, label, WebviewUrl::App(format!("index.html?w={label}").into()))
        .title(format!("Cownter {label}"))
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(cfg!(target_os = "linux"))
        .shadow(false)
        .focused(false)
        .focusable(label != "overlay" && !cfg!(target_os = "linux"))
        .visible(false)
        .visible_on_all_workspaces(true)
        .inner_size(10.0, 10.0);
    if label == "overlay" {
        b = b.focusable(false);
    }
    let w = b.build()?;
    #[cfg(target_os = "linux")]
    if let Ok(gw) = w.gtk_window() {
        use gtk::prelude::{GtkWindowExt, WidgetExt, WidgetExtManual};
        gw.set_type_hint(gtk::gdk::WindowTypeHint::Dock);
        // Widgets never take keyboard focus, so GNOME lets newly opened apps come to the front.
        gw.set_accept_focus(false);
        gw.set_focus_on_map(false);
        // The input shape is smaller than the window, so the webview misses the pointer crossing it.
        gw.add_events(gtk::gdk::EventMask::ENTER_NOTIFY_MASK | gtk::gdk::EventMask::LEAVE_NOTIFY_MASK);
        let win = w.clone();
        gw.connect_enter_notify_event(move |_, e| {
            if e.detail() != gtk::gdk::NotifyType::Inferior {
                let _ = win.emit_to(win.label(), "pointer-in", ());
            }
            gtk::glib::Propagation::Proceed
        });
        let win = w.clone();
        gw.connect_leave_notify_event(move |_, e| {
            if e.detail() != gtk::gdk::NotifyType::Inferior {
                let _ = win.emit_to(win.label(), "pointer-out", ());
            }
            gtk::glib::Propagation::Proceed
        });
    }
    Ok(w)
}

/// Creates or closes form windows so they match the display config. The overlay window always exists.
pub fn sync(app: &AppHandle, cfg: &DisplayConfig) {
    if app.get_webview_window("overlay").is_none() {
        if let Err(e) = build_widget(app, "overlay") {
            eprintln!("overlay window: {e}");
        }
    }
    for label in forms() {
        let existing = app.get_webview_window(label);
        match (enabled(cfg, label), existing) {
            (true, None) => {
                if let Err(e) = build_widget(app, label) {
                    eprintln!("{label} window: {e}");
                }
            }
            (false, Some(w)) => {
                let _ = w.destroy();
            }
            (true, Some(w)) if cfg.hidden && !TRANSIENT.contains(&label) => {
                let _ = w.hide();
            }
            _ => {}
        }
    }
}

pub fn set_hidden(app: &AppHandle, cfg: &DisplayConfig) {
    for label in forms().filter(|l| !TRANSIENT.contains(l)) {
        if let Some(w) = app.get_webview_window(label) {
            let _ = if cfg.hidden { w.hide() } else { w.show() };
        }
    }
}

struct Area {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

fn area(w: &WebviewWindow, monitor: &str) -> Option<Area> {
    let chosen = if monitor.is_empty() {
        None
    } else {
        w.available_monitors().ok().and_then(|ms| ms.into_iter().find(|m| m.name().is_some_and(|n| n == monitor)))
    };
    let m = chosen
        .or_else(|| w.primary_monitor().ok().flatten())
        .or_else(|| w.current_monitor().ok().flatten())?;
    let s = m.scale_factor();
    let r = m.work_area();
    Some(Area {
        x: r.position.x as f64 / s,
        y: r.position.y as f64 / s,
        w: r.size.width as f64 / s,
        h: r.size.height as f64 / s,
    })
}

fn anchor(position: &str, a: &Area, width: f64, height: f64, margin: f64) -> (f64, f64) {
    let x = if position.ends_with("Left") {
        a.x + margin
    } else if position.ends_with("Center") {
        a.x + (a.w - width) / 2.0
    } else {
        a.x + a.w - width - margin
    };
    let y = if position.starts_with("bottom") {
        a.y + a.h - height - margin
    } else if position.starts_with("middle") {
        a.y + (a.h - height) / 2.0
    } else {
        a.y + margin
    };
    (x, y)
}

/// Sizes a widget window (logical pixels) and places it according to its form and the config.
pub fn place(app: &AppHandle, cfg: &DisplayConfig, label: &str, width: f64, height: f64, show: bool) -> Result<(), String> {
    let w = app.get_webview_window(label).ok_or("window not found")?;
    let form = cfg.form(if label == "recap" { "pop" } else { label });
    let a = area(&w, form.map(|f| f.monitor.as_str()).unwrap_or("")).ok_or("no monitor found")?;
    let travel = label == "train" && form.is_some_and(|f| f.form_opts.get("travel") != Some(&false));
    let (width, height) = if form.is_none() { (a.w, a.h) } else { (width.clamp(1.0, a.w), height.clamp(1.0, a.h)) };
    let width = if travel { a.w } else { width };
    let (x, y) = match form {
        Some(f) if DOCKED.contains(&label) => {
            let right = f.side != "left";
            let x = if right { a.x + a.w - width - f.offset_x } else { a.x + f.offset_x };
            let y = if label == "drawer" { a.y + f.offset * a.h - height / 2.0 } else { a.y + f.offset * (a.h - height).max(0.0) };
            (x, y + f.offset_y)
        }
        Some(f) if travel => {
            let (_, y) = anchor(&f.position, &a, width, height, 0.0);
            (a.x, y + f.offset_y)
        }
        Some(f) => {
            let margin = if FLUSH.contains(&label) { 0.0 } else { MARGIN };
            let (x, mut y) = anchor(&f.position, &a, width, height, margin);
            if label != "island" && f.position == "topCenter" && cfg.island.enabled && cfg.island.position == "topCenter" && cfg.island.monitor == f.monitor && !FLUSH.contains(&label) {
                y += 64.0;
            }
            (x + f.offset_x, y + f.offset_y)
        }
        None => (a.x, a.y),
    };
    let keep = KEEP_VISIBLE.min(width).min(height);
    let x = x.clamp(a.x - width + keep, a.x + a.w - keep);
    let y = y.clamp(a.y - height + keep, a.y + a.h - keep);
    let size = LogicalSize::new(width, height);
    if cfg!(target_os = "linux") {
        let _ = w.set_min_size(None::<LogicalSize<f64>>);
        let _ = w.set_max_size(None::<LogicalSize<f64>>);
    }
    w.set_size(size).map_err(|e| e.to_string())?;
    if cfg!(target_os = "linux") {
        let _ = w.set_min_size(Some(size));
        let _ = w.set_max_size(Some(size));
    }
    w.set_position(LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    let visible = show && !(cfg.hidden && form.is_some() && !TRANSIENT.contains(&label));
    if visible {
        w.show().map_err(|e| e.to_string())?;
        if label == "overlay" || label == "train" {
            let _ = w.set_ignore_cursor_events(label == "overlay" || travel);
        }
        let _ = w.set_always_on_top(true);
    }
    Ok(())
}

/// Moves already visible widgets after a position setting changed.
pub fn replace_all(app: &AppHandle, cfg: &DisplayConfig) {
    for label in forms() {
        let Some(w) = app.get_webview_window(label) else { continue };
        if !w.is_visible().unwrap_or(false) {
            continue;
        }
        let (Ok(size), Ok(scale)) = (w.inner_size(), w.scale_factor()) else { continue };
        let size = size.to_logical::<f64>(scale);
        let _ = place(app, cfg, label, size.width, size.height, false);
    }
}

fn work_areas(app: &AppHandle) -> Vec<(i32, i32, u32, u32)> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let r = m.work_area();
            (r.position.x, r.position.y, r.size.width, r.size.height)
        })
        .collect()
}

/// Re-places visible widgets whenever a monitor's work area changes (panels showing, hiding or moving).
pub fn watch_work_area(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut last = work_areas(&app);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let now = work_areas(&app);
            if now != last {
                last = now;
                if let Some(state) = app.try_state::<crate::state::AppState>() {
                    replace_all(&app, &state.display());
                }
            }
        }
    });
}

/// Names of the connected monitors, primary first.
pub fn monitors(app: &AppHandle) -> Vec<String> {
    let primary = app.primary_monitor().ok().flatten().and_then(|m| m.name().cloned());
    let mut names: Vec<String> = app
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| m.name().cloned())
        .collect();
    names.sort_by_key(|n| Some(n) != primary.as_ref());
    names.dedup();
    names
}

/// Shows the weekly recap card; the window is created on first use and loads the recap when it starts.
pub fn show_recap(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("recap").is_some() {
        return app.emit_to("recap", "recap", ());
    }
    build_widget(app, "recap").map(|_| ())
}

pub fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("settings") {
        w.show()?;
        w.unminimize()?;
        w.set_focus()?;
        return Ok(());
    }
    let icon = tauri::image::Image::from_bytes(include_bytes!("../../assets/icons/256x256.png"))?;
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html?w=settings".into()))
        .title("Cownter")
        .icon(icon)?
        .inner_size(860.0, 580.0)
        .min_inner_size(720.0, 480.0)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .center()
        .build()?;
    Ok(())
}
