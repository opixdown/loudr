// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // On Wayland, apps can't place a window or keep it on top of other apps,
    // so the floating pill would sit hidden behind the app being typed into.
    // Running through XWayland restores both. Set GDK_BACKEND yourself to
    // override this.
    #[cfg(target_os = "linux")]
    if std::env::var_os("GDK_BACKEND").is_none() && std::env::var_os("DISPLAY").is_some() {
        // SAFETY: nothing else is running yet, so no other thread can be
        // reading the environment.
        unsafe { std::env::set_var("GDK_BACKEND", "x11") };
    }
    loudr_lib::run()
}
