//! Small user preferences stored separately from workspace contents.
use anyhow::{Context, Result};
use std::{io::Write, path::PathBuf};

fn wrap_path_for(application: &str) -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("Home directory is unavailable")?;
    Ok(PathBuf::from(home)
        .join("Library/Application Support")
        .join(application)
        .join("word-wrap"))
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
