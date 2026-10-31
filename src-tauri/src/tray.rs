use crate::pixfont;
use crate::state::AppState;
use cownter_core::model::EntryView;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "main";

fn base_icon() -> Image<'static> {
    #[cfg(target_os = "macos")]
    let bytes: &[u8] = include_bytes!("../../assets/icons/tray-dark-44.png");
    #[cfg(target_os = "linux")]
    let bytes: &[u8] = include_bytes!("../../assets/icons/tray-light-64.png");
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let bytes: &[u8] = include_bytes!("../../assets/icons/tray-light-32.png");
    Image::from_bytes(bytes).expect("tray icon")
}

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let open = MenuItem::with_id(app, "open", "Open Cownter", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Show / hide widgets", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh all", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &refresh, &sep, &quit])?;
    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(base_icon())
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Cownter")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "open" => {
                let _ = crate::windows::open_settings(app);
            }
            "toggle" => crate::toggle_hidden(app),
            "refresh" => {
                if let Some(s) = app.try_state::<AppState>() {
                    s.scheduler.refresh_all();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                let _ = crate::windows::open_settings(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(tray)
}

/// Shows the tray entry's number: as text next to the icon on macOS/Linux, drawn into the icon on Windows.
pub fn update(app: &AppHandle, view: Option<&EntryView>, show_number: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    if show_number && view.is_some_and(|v| v.value.is_none() && v.text.is_none()) {
        return;
    }
    let label = view.and_then(|v| v.text.as_ref()).filter(|_| show_number).map(|t| {
        let short: String = t.chars().take(20).collect();
        if short.chars().count() < t.chars().count() { format!("{short}…") } else { short }
    });
    let text = match (show_number, view.and_then(|v| v.value)) {
        _ if label.is_some() && !cfg!(target_os = "windows") => label,
        (true, Some(v)) if cfg!(target_os = "windows") => Some(pixfont::compact(v)),
        (true, Some(v)) => Some(pixfont::full(v)),
        _ => None,
    };
    let tooltip = match (view, &text) {
        (Some(v), Some(t)) => format!("Cownter · {}: {t}", v.entry.name),
        _ => "Cownter".to_string(),
    };
    let _ = tray.set_tooltip(Some(&tooltip));
    if cfg!(target_os = "windows") {
        match text {
            Some(t) => {
                let px = pixfont::render(&t, 32, [255, 255, 255, 255], [32, 36, 46, 255]);
                let _ = tray.set_icon(Some(Image::new_owned(px, 32, 32)));
            }
            None => {
                let _ = tray.set_icon(Some(base_icon()));
            }
        }
    } else {
        let _ = tray.set_title(text.as_deref());
    }
}

/// Clears the tray label so the next update is sent again, for panels that missed the first one.
pub fn clear_title(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if !cfg!(target_os = "windows") {
            let _ = tray.set_title(None::<&str>);
        }
    }
}
