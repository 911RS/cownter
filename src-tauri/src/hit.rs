use serde::Deserialize;
use tauri::{AppHandle, WebviewWindow};

/// Part of a widget window that takes mouse input, in logical pixels relative to the window.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Restricts mouse input to `r`; clicks elsewhere in the window reach whatever is below it.
#[cfg(target_os = "linux")]
pub fn set(w: &WebviewWindow, r: Rect) -> Result<(), String> {
    let win = w.clone();
    w.run_on_main_thread(move || {
        use gtk::prelude::*;
        let Ok(gw) = win.gtk_window() else { return };
        let rect = gtk::cairo::RectangleInt::new(r.x.floor() as i32, r.y.floor() as i32, r.w.ceil() as i32 + 1, r.h.ceil() as i32 + 1);
        let region = gtk::cairo::Region::create_rectangle(&rect);
        gw.input_shape_combine_region(Some(&region));
    })
    .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "linux"))]
static HITS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, Rect>>> = std::sync::LazyLock::new(Default::default);

#[cfg(not(target_os = "linux"))]
pub fn set(w: &WebviewWindow, r: Rect) -> Result<(), String> {
    HITS.lock().unwrap_or_else(|p| p.into_inner()).insert(w.label().to_string(), r);
    Ok(())
}

/// Windows and macOS have no input shapes, so the cursor is polled and the whole window
/// ignores the mouse while the cursor is outside its rect.
#[cfg(not(target_os = "linux"))]
pub fn start(app: &AppHandle) {
    use tauri::{Emitter, Manager};
    let app = app.clone();
    std::thread::spawn(move || {
        let mut inside: std::collections::HashMap<String, bool> = Default::default();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(40));
            let Ok(p) = app.cursor_position() else { continue };
            let hits = HITS.lock().unwrap_or_else(|p| p.into_inner()).clone();
            for (label, r) in hits {
                let Some(w) = app.get_webview_window(&label) else { continue };
                if !w.is_visible().unwrap_or(false) {
                    continue;
                }
                let (Ok(pos), Ok(scale)) = (w.inner_position(), w.scale_factor()) else { continue };
                let x = (p.x - pos.x as f64) / scale;
                let y = (p.y - pos.y as f64) / scale;
                let now = x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
                if inside.get(&label) != Some(&now) {
                    inside.insert(label.clone(), now);
                    let _ = w.set_ignore_cursor_events(!now);
                    if !now {
                        let _ = app.emit_to(label.as_str(), "pointer-out", ());
                    }
                }
            }
        }
    });
}

#[cfg(target_os = "linux")]
pub fn start(_: &AppHandle) {}
