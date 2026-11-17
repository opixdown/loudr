//! OS permission checks. macOS gates the microphone and the keyboard; Linux
//! gates the keyboard (reading /dev/input); on Windows everything reports as
//! granted.

/// Whether this process may observe and send keyboard events (macOS
/// Accessibility). With `prompt`, macOS shows its "open System Settings"
/// dialog if permission has not been granted yet.
#[cfg(target_os = "macos")]
pub fn accessibility_granted(prompt: bool) -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::{CFString, CFStringRef};

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        static kAXTrustedCheckOptionPrompt: CFStringRef;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }

    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let options = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::from(prompt))]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
    }
}

/// On Linux the hold key is read from /dev/input, which only members of the
/// `input` group may open. True if at least one input device can be read (or
/// there are none to read, as on a machine without a keyboard).
#[cfg(target_os = "linux")]
pub fn accessibility_granted(_prompt: bool) -> bool {
    let Ok(dir) = std::fs::read_dir("/dev/input") else {
        return true;
    };
    let mut devices = dir
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("event"))
        })
        .peekable();
    devices.peek().is_none() || devices.any(|path| std::fs::File::open(path).is_ok())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn accessibility_granted(_prompt: bool) -> bool {
    true
}

/// Microphone permission: "granted", "denied", or "unknown" (not asked yet).
#[cfg(target_os = "macos")]
pub fn microphone_status() -> &'static str {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    #[link(name = "AVFoundation", kind = "framework")]
    unsafe extern "C" {
        static AVMediaTypeAudio: *const AnyObject;
    }

    // AVAuthorizationStatus: 0 not determined, 1 restricted, 2 denied, 3 authorized
    let status: isize = unsafe {
        msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: AVMediaTypeAudio]
    };
    match status {
        3 => "granted",
        0 => "unknown",
        _ => "denied",
    }
}

#[cfg(not(target_os = "macos"))]
pub fn microphone_status() -> &'static str {
    "granted"
}

/// Ask the system for microphone access. macOS shows its prompt (naming
/// Loudr) the first time; afterwards this does nothing.
#[cfg(target_os = "macos")]
pub fn request_microphone() {
    use block2::RcBlock;
    use objc2::runtime::{AnyObject, Bool};
    use objc2::{class, msg_send};

    #[link(name = "AVFoundation", kind = "framework")]
    unsafe extern "C" {
        static AVMediaTypeAudio: *const AnyObject;
    }

    log::info!("microphone: requesting access (status: {})", microphone_status());
    let done = RcBlock::new(|granted: Bool| {
        log::info!("microphone: access granted = {}", granted.as_bool());
    });
    unsafe {
        let _: () = msg_send![
            class!(AVCaptureDevice),
            requestAccessForMediaType: AVMediaTypeAudio,
            completionHandler: &*done
        ];
    }
}

/// Windows has no per-app prompt; opening the microphone is enough.
#[cfg(not(target_os = "macos"))]
pub fn request_microphone() {}

/// Open the Privacy page of System Settings for "microphone" or "accessibility".
pub fn open_privacy_settings(pane: &str) {
    #[cfg(target_os = "macos")]
    {
        let anchor = match pane {
            "microphone" => "Privacy_Microphone",
            _ => "Privacy_Accessibility",
        };
        let url = format!("x-apple.systempreferences:com.apple.preference.security?{anchor}");
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = pane;
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "ms-settings:privacy-microphone"])
            .spawn();
    }
}
