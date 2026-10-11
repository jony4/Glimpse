//! Small user preferences stored separately from workspace contents.
use anyhow::{Context, Result};
use std::{io::Write, path::PathBuf};

fn wrap_path_for(application: &str) -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(
        std::env::var_os("APPDATA").context("Application data directory is unavailable")?,
    );
    #[cfg(not(target_os = "windows"))]
    let base = PathBuf::from(std::env::var_os("HOME").context("Home directory is unavailable")?)
        .join("Library/Application Support");
    Ok(base.join(application).join("word-wrap"))
}

fn wrap_path() -> Result<PathBuf> {
    wrap_path_for("Glim")
}

pub fn word_wrap() -> bool {
    wrap_path()
        .ok()
        .and_then(|path| match std::fs::read_to_string(path) {
            Ok(value) => Some(value),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Keep the previous preference until Glim saves its own value.
                wrap_path_for("Glimpse")
                    .ok()
                    .and_then(|legacy| std::fs::read_to_string(legacy).ok())
            }
            Err(_) => None,
        })
        .is_some_and(|value| value.trim() == "true")
}

pub fn save_word_wrap(enabled: bool) -> Result<()> {
    let path = wrap_path()?;
    let parent = path.parent().context("Invalid preferences path")?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    writeln!(file, "{enabled}")?;
    file.persist(path)?;
    Ok(())
}

pub fn preview_mode(kind: &str) -> bool {
    preview_path(kind)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .is_none_or(|value| value.trim() != "source")
}
fn preview_path(kind: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        matches!(kind, "html" | "markdown"),
        "Unknown preview preference"
    );
    Ok(wrap_path()?.with_file_name(format!("{kind}-view")))
}
pub fn save_preview_mode(kind: &str, preview: bool) -> Result<()> {
    let path = preview_path(kind)?;
    let parent = path.parent().context("Invalid preferences path")?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    writeln!(file, "{}", if preview { "preview" } else { "source" })?;
    file.persist(path)?;
    Ok(())
}

pub fn diff_side_by_side() -> Option<bool> {
    let path = wrap_path().ok()?.with_file_name("git-diff-view");
    match std::fs::read_to_string(path).ok()?.trim() {
        "side-by-side" => Some(true),
        "inline" => Some(false),
        _ => None,
    }
}
pub fn save_diff_mode(side_by_side: bool) -> Result<()> {
    let path = wrap_path()?.with_file_name("git-diff-view");
    let parent = path.parent().context("Invalid preferences path")?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    writeln!(
        file,
        "{}",
        if side_by_side {
            "side-by-side"
        } else {
            "inline"
        }
    )?;
    file.persist(path)?;
    Ok(())
}
