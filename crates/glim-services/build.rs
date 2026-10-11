use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../../native/Preview.swift");
    println!("cargo:rerun-if-changed=../../native/SQLitePreview.swift");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => panic!("Unsupported macOS architecture"),
    };
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("glim-preview");
    let main = output.with_file_name("main.swift");
    std::fs::copy("../../native/Preview.swift", &main)
        .expect("Cannot prepare native preview entry point");
    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-O",
            "-swift-version",
            "5",
            "-target",
            &format!("{arch}-apple-macos11.0"),
            "../../native/SQLitePreview.swift",
        ])
        .arg(main)
        .arg("-o")
        .arg(output)
        .status()
        .expect("Native previews require Apple Command Line Tools (swiftc)");
    assert!(status.success(), "Native preview helper compilation failed");
}
