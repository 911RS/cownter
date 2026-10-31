const COMMANDS: &[&str] = &[
    "list_monitors",
    "list_entries",
    "get_display",
    "widget_resize",
    "widget_done",
    "open_settings",
    "refresh_all",
    "save_entry",
    "delete_entry",
    "reorder_entries",
    "list_groups",
    "save_group",
    "delete_group",
    "list_alerts",
    "save_alert",
    "delete_alert",
    "test_source",
    "fetch_favicon",
    "entry_history",
    "simulate_change",
    "weekly_recap",
    "show_recap",
    "save_snapshot",
    "export_csv",
    "keyring_available",
    "has_secrets",
    "save_display",
    "platform_info",
    "quit_app",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
