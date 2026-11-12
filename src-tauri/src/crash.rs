//! What happens when Loudr hits a bug it cannot recover from.
//!
//! Release builds stop at once on such a bug, and a windowed app has nowhere
//! to print why. Without this, the only trace is an exception code in the
//! system's event log. So the reason is written to a file next to the normal
//! log, and on Windows it is also shown in a message box, before the app
//! stops.

use std::io::Write;
use std::path::PathBuf;

/// Folder of the normal log file (the same place the log plugin uses).
fn log_dir() -> Option<PathBuf> {
    const APP: &str = "app.loudr.desktop";
    #[cfg(target_os = "windows")]
    return std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join(APP).join("logs"));
    #[cfg(target_os = "macos")]
    return std::env::var_os("HOME").map(|dir| PathBuf::from(dir).join("Library/Logs").join(APP));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return std::env::var_os("HOME").map(|dir| PathBuf::from(dir).join(".local/share").join(APP).join("logs"));
}

pub fn install_reporter() {
    std::panic::set_hook(Box::new(|info| {
        let place = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown place".into());
        let thread = std::thread::current();
        let report = format!(
            "Loudr {} stopped because of a bug.\n{info}\nat {place}, thread \"{}\"\n",
            env!("CARGO_PKG_VERSION"),
            thread.name().unwrap_or("unnamed"),
        );
        eprintln!("{report}");

        let file = log_dir().map(|dir| {
            let _ = std::fs::create_dir_all(&dir);
            dir.join("crash.log")
        });
        if let Some(file) = &file {
            let saved = std::fs::OpenOptions::new().create(true).append(true).open(file);
            if let Ok(mut out) = saved {
                let _ = writeln!(out, "{report}");
            }
        }

        #[cfg(target_os = "windows")]
        show_message(&match &file {
            Some(file) => format!(
                "{report}\nThis was saved to:\n{}\n\nPlease report it at github.com/opixdown/loudr/issues",
                file.display()
            ),
            None => report,
        });
    }));
}

#[cfg(target_os = "windows")]
fn show_message(text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    // Automated runs set this so a crash can't wait forever on a click.
    if std::env::var_os("LOUDR_NO_CRASH_DIALOG").is_some() {
        return;
    }
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (text, title) = (wide(text), wide("Loudr"));
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}
