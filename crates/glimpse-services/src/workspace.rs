use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use glimpse_core::{DirectoryEntry, Repository};
use ignore::WalkBuilder;

use crate::git;

pub struct FolderSnapshot {
    pub root: PathBuf,
    pub entries: Vec<DirectoryEntry>,
    pub repository: Option<Repository>,
    pub git_warning: Option<String>,
}

pub fn open_folder(path: &Path) -> Result<FolderSnapshot> {
    let root = path
        .canonicalize()
        .with_context(|| format!("Cannot open {}", path.display()))?;
    let entries = list_directory(&root)?;
    // A missing/broken Git installation must not prevent ordinary folder browsing.
    let (repository, git_warning) = match git::inspect(&root) {
        Ok(repository) => (repository, None),
        Err(error) => (None, Some(format!("Git: {error:#}"))),
    };
    Ok(FolderSnapshot {
        root,
        entries,
        repository,
        git_warning,
    })
}

/// Read only immediate children; do not follow directory symlinks or scan the whole tree.
pub fn list_directory(path: &Path) -> Result<Vec<DirectoryEntry>> {
    ensure!(path.is_dir(), "Not a directory: {}", path.display());
    let mut entries = Vec::new();
    for entry in WalkBuilder::new(path)
        .max_depth(Some(1))
        .hidden(false)
        .follow_links(false)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build()
    {
        let entry = entry.with_context(|| format!("Cannot read {}", path.display()))?;
        if entry.depth() == 0 {
            continue;
        }
        let Some(kind) = entry.file_type() else {
            continue;
        };
        entries.push(DirectoryEntry {
            path: entry.into_path(),
            is_dir: kind.is_dir(),
            is_symlink: kind.is_symlink(),
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.path.file_name().cmp(&b.path.file_name()))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn lists_one_level_respects_ignore_and_keeps_dotfiles() -> Result<()> {
        let temp = tempfile::tempdir()?;
        fs::create_dir(temp.path().join("src"))?;
        fs::write(temp.path().join("src/nested.rs"), "")?;
        fs::create_dir(temp.path().join(".git"))?;
        fs::write(temp.path().join(".gitignore"), "ignored\n")?;
        fs::write(temp.path().join("ignored"), "")?;
        fs::write(temp.path().join("main.rs"), "")?;
        let entries = list_directory(temp.path())?;
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy())
            .collect();
        assert_eq!(names, ["src", ".gitignore", "main.rs"]);
        assert!(entries[0].is_dir);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn directory_symlinks_do_not_create_recursive_trees() -> Result<()> {
        let temp = tempfile::tempdir()?;
        std::os::unix::fs::symlink(temp.path(), temp.path().join("loop"))?;
        let entries = list_directory(temp.path())?;
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_symlink);
        assert!(!entries[0].is_dir);
        Ok(())
    }
}
