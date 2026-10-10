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
pub fn is_office(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "rtf"
        )
    })
}

pub fn supports(path: &Path) -> bool {
    cfg!(target_os = "macos")
        && (is_media(path)
            || is_office(path)
            || path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("pdf")))
}

fn playlist(path: &Path, folder: bool) -> Result<Vec<PathBuf>> {
    if !folder {
        ensure!(
            supports(path) && path.is_file(),
            "Unsupported or missing preview file"
        );
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
) -> Result<()> {
    let paths = playlist(path, folder)?;
    #[cfg(target_os = "macos")]
    {
        use std::{
            os::unix::fs::PermissionsExt,
            process::{Command, Stdio},
        };
        let directory = tempfile::tempdir()?;
        let executable = directory.path().join("glim-preview");
        std::fs::write(
            &executable,
            include_bytes!(concat!(env!("OUT_DIR"), "/glim-preview")),
        )?;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))?;
        let files = paths
            .iter()
            .map(|path| path.to_str().context("Native preview requires UTF-8 paths"))
            .collect::<Result<Vec<_>>>()?;
        let manifest = directory.path().join("playlist.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec(
                &serde_json::json!({"files": files, "shuffle": shuffle, "office": !folder && is_office(path)}),
            )?,
        )?;
        // Keep the temporary executable and manifest alive until its window closes.
        let mut child = Command::new(executable)
            .arg(manifest)
            .stdin(Stdio::null())
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
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (paths, shuffle, cancelled);
        anyhow::bail!("Native preview requires macOS")
    }
}
