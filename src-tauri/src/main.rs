#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(windows)]
    {
        // Release builds have no console of their own; print to the terminal that started us.
        if cownter_lib::cli::parse(&args).is_some() {
            use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
            unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
        }
    }
    if let Some(code) = cownter_lib::cli::run(&args) {
        std::process::exit(code);
    }
    #[cfg(target_os = "linux")]
    {
        // Widgets need exact placement and always-on-top, which Wayland does not allow for normal windows.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("GDK_BACKEND").is_none() {
            unsafe { std::env::set_var("GDK_BACKEND", "x11") };
        }
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
        }
    }
    cownter_lib::run()
}
