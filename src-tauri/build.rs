fn main() {
    // whisper.cpp's Metal backend uses Objective-C `@available` checks, which
    // call a helper that lives in clang's runtime library. Rust does not link
    // that library by default, so add it for macOS targets.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let out = std::process::Command::new("clang")
            .arg("--print-runtime-dir")
            .output()
            .expect("clang not found");
        let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-lib=static=clang_rt.osx");
    }

    tauri_build::build()
}
