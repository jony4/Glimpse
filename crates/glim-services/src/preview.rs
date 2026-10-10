//! Native PDFKit / AVKit / Quick Look windows. Run from a retained background task.
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

pub fn is_media(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "mp3" | "mp4" | "m4a" | "m4v" | "mov" | "aac" | "wav" | "aif" | "aiff" | "caf" | "flac"
        )
    })
}
pub fn is_font(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "ttf" | "otf" | "ttc" | "otc" | "dfont"
        )
    })
}

pub fn is_office(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "rtf" | "key" | "pages" | "numbers"
        )
    })
}

pub fn is_iwork(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "key" | "pages" | "numbers"))
}

/// `.key` is also used by plain-text cryptographic keys. Only document packages
/// and ZIP containers should be handed to Keynote's Quick Look provider.
pub fn can_open(path: &Path) -> bool {
    if !supports(path) {
        return false;
    }
    if path.is_dir() {
        return is_iwork(path);
    }
    if !path.is_file() {
        return false;
    }
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("key"))
    {
        use std::io::Read;
        let mut prefix = [0; 4];
        return std::fs::File::open(path)
            .and_then(|mut file| file.read_exact(&mut prefix))
            .is_ok()
            && prefix == *b"PK\x03\x04";
    }
    true
}

pub fn supports(path: &Path) -> bool {
    cfg!(target_os = "macos")
        && (is_media(path)
            || is_font(path)
            || is_office(path)
            || path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("pdf")))
}

fn playlist(path: &Path, folder: bool) -> Result<Vec<PathBuf>> {
    if !folder {
        ensure!(can_open(path), "Unsupported or missing preview file");
        if is_font(path) {
            ensure!(
                std::fs::metadata(path)?.len() <= 64 * 1024 * 1024,
                "Font preview is limited to 64 MiB"
            );
        }
        return Ok(vec![path.to_path_buf()]);
    }
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        if is_media(&entry.path()) && entry.file_type()?.is_file() {
            ensure!(
                paths.len() < 10_000,
                "A playlist supports up to 10,000 files; choose a smaller folder"
            );
            paths.push(entry.path());
        }
    }
    paths.sort();
    ensure!(
        !paths.is_empty(),
        "This folder contains no supported audio or video files"
    );
    Ok(paths)
}

pub fn open(
    path: &Path,
    folder: bool,
    shuffle: bool,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    show: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    let paths = playlist(path, folder)?;
    let files = paths
        .iter()
        .map(|path| path.to_str().context("Native preview requires UTF-8 paths"))
        .collect::<Result<Vec<_>>>()?;
    launch(
        serde_json::json!({"files": files, "shuffle": shuffle, "office": !folder && is_office(path), "font": !folder && is_font(path)}),
        cancelled,
        show,
    )
}

pub fn open_associations(cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<()> {
    let executable = std::env::current_exe()?;
    let bundle = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .context("Cannot locate Glim.app")?;
    ensure!(
        bundle.extension().is_some_and(|s| s == "app")
            && bundle.join("Contents/Info.plist").is_file(),
        "Install Glim.app before configuring default applications"
    );
    launch(
        serde_json::json!({"files": [], "shuffle": false, "associations": bundle.to_str().context("Invalid application path")?}),
        cancelled,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    )
}

fn launch(
    configuration: serde_json::Value,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    show: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::{
            os::unix::fs::PermissionsExt,
            process::{Command, Stdio},
        };
        let directory = tempfile::tempdir()?;
        // A real helper bundle prevents AppKit from adopting the parent's
        // application identity and sharing its Dock/window activation behavior.
        let settings = configuration.get("associations").is_some();
        let helper_name = if settings {
            "Glim Settings"
        } else {
            "Glim Preview"
        };
        let contents = directory.path().join(format!("{helper_name}.app/Contents"));
        let binaries = contents.join("MacOS");
        std::fs::create_dir_all(&binaries)?;
        let resources = contents.join("Resources");
        std::fs::create_dir_all(&resources)?;
        std::fs::write(
            resources.join("Glim.icns"),
            include_bytes!("../../../assets/macos/Glim.icns"),
        )?;
        let info = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Glim Preview</string>
<key>CFBundleDisplayName</key><string>Glim Preview</string>
<key>CFBundleIdentifier</key><string>io.github.jony4.glim.preview</string>
<key>CFBundleExecutable</key><string>glim-preview</string>
<key>CFBundleIconFile</key><string>Glim</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>"#;
        let info = if settings {
            info.replace("Glim Preview", "Glim Settings").replace(
                "io.github.jony4.glim.preview",
                "io.github.jony4.glim.settings",
            )
        } else {
            info.to_owned()
        };
        std::fs::write(contents.join("Info.plist"), info)?;
        let executable = binaries.join("glim-preview");
        std::fs::write(
            &executable,
            include_bytes!(concat!(env!("OUT_DIR"), "/glim-preview")),
        )?;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))?;
        let manifest = directory.path().join("request.json");
        std::fs::write(&manifest, serde_json::to_vec(&configuration)?)?;
        // Keep the temporary executable and manifest alive until its window closes.
        let mut child = Command::new(executable)
            .arg(manifest)
            .env_remove("__CFBundleIdentifier")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        loop {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(());
            }
            if let Some(status) = child.try_wait()? {
                ensure!(
                    status.success(),
                    "The native preview could not open this file"
                );
                break;
            }
            if show.swap(false, std::sync::atomic::Ordering::AcqRel) {
                use std::io::Write;
                if let Some(input) = &mut child.stdin {
                    // A closing helper may race this request; try_wait handles exit.
                    let _ = input.write_all(b"show\n");
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (configuration, cancelled, show);
        anyhow::bail!("Native preview requires macOS")
    }
}
