use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use glimpse_core::{DirectoryEntry, Repository};
use ignore::WalkBuilder;

use crate::git;

pub struct FolderSnapshot {
    pub root: PathBuf,
    pub entries: Vec<DirectoryEntry>,
}

pub fn open_folder(path: &Path) -> Result<FolderSnapshot> {
    let root = path
        .canonicalize()
        .with_context(|| format!("Cannot open {}", path.display()))?;
    let entries = list_directory(&root)?;
    Ok(FolderSnapshot { root, entries })
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

/// Bounded discovery of repositories within a workspace, including linked worktrees.
pub fn repositories(root: &Path) -> Result<Vec<Repository>> {
    let current = git::inspect(root)?;
    let mut roots = std::collections::BTreeSet::new();
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .max_depth(Some(6))
        .filter_entry(|e| {
            e.file_type().is_some_and(|kind| kind.is_dir())
                && !matches!(
                    e.file_name().to_str(),
                    Some(".git" | "node_modules" | "target" | "dist")
                )
        })
        .build()
        .take(50_000)
    {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_dir()) && entry.path().join(".git").exists()
        {
            roots.insert(entry.path().to_path_buf());
        }
        if roots.len() >= 64 {
            break;
        }
    }
    let mut repositories = Vec::new();
    if let Some(repo) = current {
        roots.remove(&repo.root);
        repositories.push(repo);
    }
    for root in roots {
        if let Some(repo) = git::inspect(&root)? {
            repositories.push(repo);
        }
    }
    repositories.sort_by(|a, b| a.root.cmp(&b.root));
    Ok(repositories)
}

pub fn search_files(root: &Path, query: &str) -> Vec<PathBuf> {
    let query = query.to_lowercase();
    if query.trim().is_empty() {
        return Vec::new();
    }
    WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build()
        .take(100_000)
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .map(|e| e.into_path())
        .filter(|p| {
            p.strip_prefix(root)
                .unwrap_or(p)
                .to_string_lossy()
                .to_lowercase()
                .contains(&query)
        })
        .take(60)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn opening_folder_does_not_wait_for_git_or_scan_descendants() -> Result<()> {
        let temp = tempfile::tempdir()?;
        fs::write(temp.path().join(".git"), "gitdir: /does/not/exist")?;
        fs::create_dir(temp.path().join("nested"))?;
        fs::write(temp.path().join("nested/hidden-from-first-level.txt"), "")?;
        let snapshot = open_folder(temp.path())?;
        assert_eq!(snapshot.root, temp.path().canonicalize()?);
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].path.file_name().unwrap(), "nested");
        Ok(())
    }

    #[test]
    fn discovery_keeps_nested_repositories_without_duplicates() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().canonicalize()?;
        for path in [&root, &root.join("nested")] {
            fs::create_dir_all(path)?;
            let status = std::process::Command::new("git")
                .args(["init", "-q", "-b", "main"])
                .arg(path)
                .status()?;
            assert!(status.success());
        }
        let repos = repositories(&root)?;
        assert_eq!(
            repos.iter().map(|r| r.root.clone()).collect::<Vec<_>>(),
            [root.clone(), root.join("nested")]
        );
        Ok(())
    }

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
