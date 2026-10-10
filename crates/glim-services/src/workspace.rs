use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use glim_core::{DirectoryEntry, Repository};
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
        let is_document_package = kind.is_dir() && crate::preview::is_iwork(entry.path());
        entries.push(DirectoryEntry {
            path: entry.into_path(),
            is_dir: kind.is_dir() && !is_document_package,
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

/// Completed discovery can contain useful repositories even when some paths are unreadable.
#[derive(Default)]
pub struct RepositoryScan {
    pub repositories: Vec<Repository>,
    failed_paths: std::collections::BTreeSet<PathBuf>,
    warnings: Vec<String>,
}
impl RepositoryScan {
    fn warn(&mut self, path: &Path, error: impl std::fmt::Display) {
        if !self.failed_paths.insert(path.to_path_buf()) {
            return;
        }
        // Keep diagnostics bounded even when a large disconnected volume fails.
        if self.warnings.len() < 5 {
            self.warnings.push(format!("{}: {error}", path.display()));
        }
    }

    pub fn warning(&self) -> Option<String> {
        (!self.failed_paths.is_empty()).then(|| {
            format!(
                "Git 扫描已完成，{} 处读取失败；已保留可用仓库。可刷新重试。\n{}{}",
                self.failed_paths.len(),
                self.warnings.join("\n"),
                if self.failed_paths.len() > self.warnings.len() {
                    "\n…"
                } else {
                    ""
                },
            )
        })
    }
}

// System-managed data is not an implicit project search location. Explicitly
// opening a directory inside it still scans that selected directory normally.
fn discovery_exclusions(root: &Path) -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let mut paths = vec![home.join(".Trash")];
    #[cfg(target_os = "macos")]
    paths.push(home.join("Library"));
    paths
        .into_iter()
        .filter(|path| !root.starts_with(path))
        .collect()
}

fn unavailable_descendant(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::PermissionDenied
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::NotFound
    )
}

fn walk_error_path(error: &ignore::Error) -> Option<&Path> {
    match error {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            walk_error_path(err)
        }
        _ => None,
    }
}

fn scan_repositories(
    root: &Path,
    scan: &mut RepositoryScan,
    seen: &mut std::collections::BTreeSet<PathBuf>,
) {
    match git::inspect(root) {
        Ok(Some(repo)) => {
            seen.insert(repo.root.clone());
            scan.repositories.push(repo);
        }
        Ok(None) => {}
        Err(error) => scan.warn(root, format!("{error:#}")),
    }
    // No elapsed-time, depth, entry-count or repository-count cutoff. Ignore rules,
    // generated directories and symlink boundaries still define the search scope.
    let excluded = discovery_exclusions(root);
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .filter_entry(move |e| {
            if e.depth() == 0 {
                return true;
            }
            if excluded.iter().any(|path| e.path().starts_with(path)) {
                return false;
            }
            e.file_type().is_some_and(|kind| kind.is_dir())
                && !crate::preview::is_iwork(e.path())
                && !matches!(
                    e.file_name().to_str(),
                    Some(".git" | "node_modules" | "target" | "dist")
                )
        })
        .build()
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let path = walk_error_path(&error).unwrap_or(root);
                let descendant = path != root || error.depth().is_some_and(|depth| depth > 0);
                if descendant && error.io_error().is_some_and(unavailable_descendant) {
                    continue;
                }
                scan.warn(path, &error);
                continue;
            }
        };
        let path = entry.path();
        if seen.contains(path) {
            continue;
        }
        match path.join(".git").try_exists() {
            Ok(false) => continue,
            Err(error) => {
                if path == root || !unavailable_descendant(&error) {
                    scan.warn(path, error);
                }
                continue;
            }
            Ok(true) => {}
        }
        seen.insert(path.to_path_buf());
        match git::inspect(path) {
            Ok(Some(repo)) => scan.repositories.push(repo),
            Ok(None) => {}
            Err(error) => scan.warn(path, format!("{error:#}")),
        }
    }
}

/// Merge nested and overlapping workspace roots without duplicate repositories.
pub fn repositories_for_roots(roots: &[PathBuf]) -> RepositoryScan {
    let mut scan = RepositoryScan::default();
    let mut seen = std::collections::BTreeSet::new();
    for root in roots {
        scan_repositories(root, &mut scan, &mut seen);
    }
    scan.repositories.sort_by(|a, b| a.root.cmp(&b.root));
    scan.repositories.dedup_by(|a, b| a.root == b.root);
    scan
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
        let repos = repositories_for_roots(std::slice::from_ref(&root)).repositories;
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
