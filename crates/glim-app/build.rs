use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=../../assets/windows/Glim.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let directory = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../assets/windows/Glim.ico")
        .canonicalize()
        .unwrap();
    let resource = directory.join("glim.rc");
    std::fs::write(
        &resource,
        format!(
            "1 ICON \"{}\"\n",
            icon.to_string_lossy()
                .trim_start_matches(r"\\?\")
                .replace('\\', "/")
        ),
    )
    .unwrap();
    let compiled = directory.join("glim.res");
    let status = Command::new("rc.exe")
        .arg("/nologo")
        .arg("/fo")
        .arg(&compiled)
        .arg(resource)
        .status()
        .expect("Windows SDK resource compiler is required");
    assert!(status.success(), "Cannot compile Windows icon resource");
    println!("cargo:rustc-link-arg-bins={}", compiled.display());
}
