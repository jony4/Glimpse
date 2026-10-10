use std::{
    ffi::{OsStr, OsString},
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail, ensure};
use glim_core::{DiffDocument, DiffScope, GitChange, Repository};

const MAX_GIT_OUTPUT: u64 = 8 * 1024 * 1024;

struct GitOutput {
    code: Option<i32>,
    bytes: Vec<u8>,
    stderr: String,
}

/// No shell, pager, optional index writes, external diff drivers, or textconv execution.
fn run(root: &Path, args: &[&OsStr]) -> Result<GitOutput> {
    let mut child = Command::new("git")
        .arg("--no-pager")
        .args(["-c", "core.fsmonitor=false"])
        .arg("-C")
        .arg(root)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Cannot run git; install Git or Apple Command Line Tools")?;
    let mut stderr = child.stderr.take().context("Missing Git stderr")?;
    let errors = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.by_ref().take(64 * 1024).read_to_end(&mut bytes);
        // Keep draining so a verbose hook cannot block the child on a full pipe.
        let _ = std::io::copy(&mut stderr, &mut std::io::sink());
        String::from_utf8_lossy(&bytes).into_owned()
    });
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .context("Missing Git stdout")?
        .take(MAX_GIT_OUTPUT + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() as u64 > MAX_GIT_OUTPUT {
        let _ = child.kill();
        let _ = child.wait();
        let _ = errors.join();
        read?;
        bail!("Git output exceeds the 8 MiB viewer limit; select a smaller change");
    }
    let status = child.wait()?;
    Ok(GitOutput {
        code: status.code(),
        bytes,
        stderr: errors.join().unwrap_or_default(),
    })
}

fn checked(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let args: Vec<_> = args.iter().map(OsStr::new).collect();
    let output = run(root, &args)?;
    if output.code != Some(0) {
        // Merge conflict explanations can be written to stdout, not stderr.
        let stdout = String::from_utf8_lossy(&output.bytes);
        let message = if output.stderr.trim().is_empty() {
            stdout.trim()
        } else {
            output.stderr.trim()
        };
        bail!(
            "Git exited with {:?}: {}",
            output.code,
            message.chars().take(16_384).collect::<String>()
        );
    }
    Ok(output.bytes)
}

fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(OsString::from_vec(bytes.to_vec()))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

pub fn inspect(directory: &Path) -> Result<Option<Repository>> {
    let output = run(
        directory,
        &[OsStr::new("rev-parse"), OsStr::new("--show-toplevel")],
    )?;
    if output.code != Some(0) {
        if output.stderr.contains("not a git repository")
            || output.stderr.contains("must be run in a work tree")
        {
            return Ok(None);
        }
        bail!("{}", output.stderr.trim());
    }
    let root = path_from_bytes(output.bytes.strip_suffix(b"\n").unwrap_or(&output.bytes));
    let branch = checked(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .or_else(|_| checked(&root, &["rev-parse", "--short", "HEAD"]))?;
    let status = checked(
        &root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    Ok(Some(Repository {
        root,
        branch: String::from_utf8_lossy(&branch).trim().to_string(),
        changes: parse_status(&status)?,
    }))
}

fn parse_status(bytes: &[u8]) -> Result<Vec<GitChange>> {
    let mut records = bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    let mut changes = Vec::new();
    while let Some(record) = records.next() {
        ensure!(
            record.len() >= 4 && record[2] == b' ',
            "Invalid Git status record"
        );
        let x = record[0];
        let y = record[1];
        let path = path_from_bytes(&record[3..]);
        let original_path = if [x, y].iter().any(|s| matches!(s, b'R' | b'C')) {
            Some(path_from_bytes(
                records.next().context("Missing rename source")?,
            ))
        } else {
            None
        };
        let mut push = |scope, status| {
            changes.push(GitChange {
                path: path.clone(),
                original_path: match scope {
                    DiffScope::Index if matches!(x, b'R' | b'C') => original_path.clone(),
                    DiffScope::Worktree if matches!(y, b'R' | b'C') => original_path.clone(),
                    _ => None,
                },
                scope,
                status,
            })
        };
        if x == b'?' && y == b'?' {
            push(DiffScope::Untracked, '?');
        } else if matches!(
            &record[..2],
            b"DD" | b"AU" | b"UD" | b"UA" | b"DU" | b"AA" | b"UU"
        ) {
            push(DiffScope::Conflict, 'U');
        } else {
            if x != b' ' && x != b'!' {
                push(DiffScope::Index, x as char);
            }
            if y != b' ' && y != b'!' {
                push(DiffScope::Worktree, y as char);
            }
        }
    }
    Ok(changes)
}

pub fn read_diff(root: &Path, change: &GitChange) -> Result<DiffDocument> {
    for path in std::iter::once(&change.path).chain(change.original_path.iter()) {
        ensure!(
            !path.is_absolute() && !path.components().any(|c| matches!(c, Component::ParentDir)),
            "Invalid repository-relative path"
        );
    }
    let mut args: Vec<OsString> = [
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--unified=3",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    match change.scope {
        DiffScope::Index => args.push("--cached".into()),
        DiffScope::Untracked => args.push("--no-index".into()),
        DiffScope::Conflict => args.push("--cc".into()),
        DiffScope::Worktree => {}
    }
    args.push("--".into());
    if change.scope == DiffScope::Untracked {
        args.push("/dev/null".into());
        args.push(root.join(&change.path).into_os_string());
    } else {
        args.push(change.path.as_os_str().to_owned());
        if let Some(original) = &change.original_path {
            args.push(original.as_os_str().to_owned());
        }
    }
    let refs: Vec<_> = args.iter().map(OsString::as_os_str).collect();
    let output = run(root, &refs)?;
    ensure!(
        output.code == Some(0) || (change.scope == DiffScope::Untracked && output.code == Some(1)),
        "{}",
        output.stderr.trim()
    );
    Ok(DiffDocument {
        change: change.clone(),
        patch: String::from_utf8_lossy(&output.bytes).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git(root: &Path, args: &[&str]) -> Result<()> {
        checked(root, args).map(|_| ())
    }
    fn repo() -> Result<tempfile::TempDir> {
        let temp = tempfile::tempdir()?;
        git(temp.path(), &["init", "-b", "main"])?;
        git(
            temp.path(),
            &["config", "user.email", "test@example.invalid"],
        )?;
        git(temp.path(), &["config", "user.name", "Test"])?;
        Ok(temp)
    }

    #[test]
    fn distinguishes_index_worktree_untracked_and_deletion() -> Result<()> {
        let temp = repo()?;
        let root = temp.path();
        fs::write(root.join("code.rs"), "fn old() {}\n")?;
        fs::write(root.join("delete.txt"), "deleted\n")?;
        git(root, &["add", "."])?;
        git(root, &["commit", "-m", "base"])?;
        fs::write(root.join("code.rs"), "fn staged() {}\n")?;
        git(root, &["add", "code.rs"])?;
        fs::write(root.join("code.rs"), "fn working() {}\n")?;
        fs::write(root.join("new file.md"), "# hello\n")?;
        fs::remove_file(root.join("delete.txt"))?;
        let repository = inspect(root)?.unwrap();
        assert_eq!(repository.changes.len(), 4);
        for change in &repository.changes {
            let patch = read_diff(&repository.root, change)?.patch;
            match (change.path.to_str().unwrap(), change.scope) {
                ("code.rs", DiffScope::Index) => {
                    assert!(patch.contains("-fn old()"));
                    assert!(patch.contains("+fn staged()"));
                    assert!(!patch.contains("working"));
                }
                ("code.rs", DiffScope::Worktree) => {
                    assert!(patch.contains("-fn staged()"));
                    assert!(patch.contains("+fn working()"));
                }
                ("new file.md", DiffScope::Untracked) => assert!(patch.contains("+# hello")),
                ("delete.txt", DiffScope::Worktree) => assert!(patch.contains("-deleted")),
                _ => panic!("Unexpected change: {change:?}"),
            }
        }
        assert_eq!(
            fs::read_to_string(root.join("code.rs"))?,
            "fn working() {}\n"
        );
        Ok(())
    }

    #[test]
    fn handles_unborn_repo_rename_binary_nested_and_worktrees() -> Result<()> {
        let temp = repo()?;
        let root = temp.path();
        assert_eq!(inspect(root)?.unwrap().branch, "main");
        fs::write(root.join("old name.txt"), "hello\n")?;
        git(root, &["add", "."])?;
        assert!(
            read_diff(root, &inspect(root)?.unwrap().changes[0])?
                .patch
                .contains("+hello")
        );
        git(root, &["commit", "-m", "base"])?;
        git(root, &["mv", "old name.txt", "新 name.txt"])?;
        let rename = inspect(root)?.unwrap().changes.remove(0);
        assert_eq!(
            rename.original_path.as_deref(),
            Some(Path::new("old name.txt"))
        );
        assert!(read_diff(root, &rename)?.patch.contains("rename from"));
        fs::write(root.join("binary"), [0, 1, 2])?;
        let binary = inspect(root)?
            .unwrap()
            .changes
            .into_iter()
            .find(|c| c.path == Path::new("binary"))
            .unwrap();
        assert!(read_diff(root, &binary)?.patch.contains("Binary files"));
        fs::create_dir(root.join("nested"))?;
        assert_eq!(
            inspect(&root.join("nested"))?.unwrap().root,
            root.canonicalize()?
        );
        let linked = tempfile::tempdir()?;
        let worktree = linked.path().join("linked");
        git(
            root,
            &[
                "worktree",
                "add",
                "-b",
                "linked",
                worktree.to_str().unwrap(),
            ],
        )?;
        assert_eq!(inspect(&worktree)?.unwrap().branch, "linked");
        assert!(inspect(tempfile::tempdir()?.path())?.is_none());
        Ok(())
    }

    #[test]
    fn parses_conflicts_and_preserves_newlines_in_paths() -> Result<()> {
        let changes = parse_status(b"UU conflict.rs\0R  new\nname.rs\0old name.rs\0")?;
        assert_eq!(changes[0].scope, DiffScope::Conflict);
        assert_eq!(changes[1].path, Path::new("new\nname.rs"));
        assert_eq!(
            changes[1].original_path.as_deref(),
            Some(Path::new("old name.rs"))
        );
        Ok(())
    }

    #[test]
    fn literal_pathspecs_do_not_include_other_files() -> Result<()> {
        let temp = repo()?;
        fs::write(temp.path().join("[a].txt"), "a\n")?;
        fs::write(temp.path().join("a.txt"), "b\n")?;
        git(temp.path(), &["add", "."])?;
        git(temp.path(), &["commit", "-m", "base"])?;
        fs::write(temp.path().join("[a].txt"), "literal\n")?;
        fs::write(temp.path().join("a.txt"), "other\n")?;
        let change = inspect(temp.path())?
            .unwrap()
            .changes
            .into_iter()
            .find(|c| c.path == Path::new("[a].txt"))
            .unwrap();
        let patch = read_diff(temp.path(), &change)?.patch;
        assert!(patch.contains("+literal"));
        assert!(!patch.contains("+other"));
        Ok(())
    }
    #[test]
    fn reads_real_merge_conflicts_without_resolving_them() -> Result<()> {
        let temp = repo()?;
        let root = temp.path();
        fs::write(root.join("conflict.txt"), "base\n")?;
        git(root, &["add", "."])?;
        git(root, &["commit", "-m", "base"])?;
        git(root, &["checkout", "-b", "other"])?;
        fs::write(root.join("conflict.txt"), "other\n")?;
        git(root, &["commit", "-am", "other"])?;
        git(root, &["checkout", "main"])?;
        fs::write(root.join("conflict.txt"), "main\n")?;
        git(root, &["commit", "-am", "main"])?;
        let result = run(root, &[OsStr::new("merge"), OsStr::new("other")])?;
        assert_eq!(result.code, Some(1));
        let change = inspect(root)?.unwrap().changes.remove(0);
        assert_eq!(change.scope, DiffScope::Conflict);
        let before = fs::read(root.join("conflict.txt"))?;
        let patch = read_diff(root, &change)?.patch;
        assert!(patch.contains("@@@"));
        assert!(patch.contains("<<<<<<<"));
        assert_eq!(fs::read(root.join("conflict.txt"))?, before);
        assert_eq!(
            inspect(root)?.unwrap().changes[0].scope,
            DiffScope::Conflict
        );
        Ok(())
    }

    #[test]
    fn rename_source_only_belongs_to_its_comparison_scope() -> Result<()> {
        let changes = parse_status(b"RM new.rs\0old.rs\0")?;
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].scope, DiffScope::Index);
        assert!(changes[0].original_path.is_some());
        assert_eq!(changes[1].scope, DiffScope::Worktree);
        assert!(changes[1].original_path.is_none());
        Ok(())
    }
}

/// Change only the requested index entries; never discard worktree contents.
pub fn set_staged(root: &Path, changes: &[GitChange], staged: bool) -> Result<()> {
    ensure!(!changes.is_empty(), "No changes selected");
    let mut paths = std::collections::BTreeSet::new();
    for change in changes {
        for path in std::iter::once(&change.path).chain(change.original_path.iter()) {
            ensure!(
                !path.as_os_str().is_empty()
                    && !path.is_absolute()
                    && path.components().all(|c| matches!(c, Component::Normal(_))),
                "Invalid repository-relative path"
            );
            paths.insert(path.clone());
        }
    }
    let unborn = !staged && checked(root, &["rev-parse", "--verify", "HEAD"]).is_err();
    let mut args: Vec<OsString> = if staged {
        ["add", "-A", "--"].into_iter().map(Into::into).collect()
    } else if unborn {
        ["rm", "--cached", "-f", "--ignore-unmatch", "--"]
            .into_iter()
            .map(Into::into)
            .collect()
    } else {
        ["reset", "-q", "HEAD", "--"]
            .into_iter()
            .map(Into::into)
            .collect()
    };
    args.extend(paths.into_iter().map(|p| p.into_os_string()));
    let output = run(
        root,
        &args.iter().map(OsString::as_os_str).collect::<Vec<_>>(),
    )?;
    ensure!(output.code == Some(0), "{}", output.stderr.trim());
    Ok(())
}

pub fn commit(root: &Path, message: &str) -> Result<()> {
    ensure!(!message.trim().is_empty(), "Enter a commit message");
    let repo = inspect(root)?.context("Repository no longer exists")?;
    ensure!(
        !repo.changes.iter().any(|c| c.scope == DiffScope::Conflict),
        "Resolve merge conflicts before committing"
    );
    ensure!(
        repo.changes.iter().any(|c| c.scope == DiffScope::Index),
        "Stage changes before committing"
    );
    let output = run(
        root,
        &[OsStr::new("commit"), OsStr::new("-m"), OsStr::new(message)],
    )?;
    ensure!(output.code == Some(0), "{}", output.stderr.trim());
    Ok(())
}

pub(crate) fn metadata_directories(root: &Path) -> Result<Vec<PathBuf>> {
    ["--absolute-git-dir", "--git-common-dir"]
        .into_iter()
        .map(|option| {
            let bytes = checked(root, &["rev-parse", "--path-format=absolute", option])?;
            Ok(path_from_bytes(bytes.strip_suffix(b"\n").unwrap_or(&bytes)))
        })
        .collect()
}

#[cfg(test)]
mod write_tests {
    use super::*;
    use std::fs;
    #[test]
    fn batch_stage_and_unstage_preserve_worktree_and_literal_paths() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        checked(root, &["init", "-b", "main"])?;
        checked(root, &["config", "user.name", "Test"])?;
        checked(root, &["config", "user.email", "test@example.invalid"])?;
        fs::create_dir(root.join("dir"))?;
        fs::write(root.join("dir/[a].txt"), "one")?;
        fs::write(root.join("other.txt"), "other")?;
        let selected = inspect(root)?
            .unwrap()
            .changes
            .into_iter()
            .filter(|c| c.path.starts_with("dir"))
            .collect::<Vec<_>>();
        set_staged(root, &selected, true)?;
        let staged = inspect(root)?
            .unwrap()
            .changes
            .into_iter()
            .filter(|c| c.scope == DiffScope::Index)
            .collect::<Vec<_>>();
        assert_eq!(staged.len(), 1);
        set_staged(root, &staged, false)?; // unborn HEAD
        assert_eq!(fs::read_to_string(root.join("dir/[a].txt"))?, "one");
        assert!(
            inspect(root)?
                .unwrap()
                .changes
                .iter()
                .all(|c| c.scope != DiffScope::Index)
        );
        set_staged(root, &selected, true)?;
        commit(root, "base")?;
        fs::rename(root.join("dir/[a].txt"), root.join("dir/new.txt"))?;
        let all = inspect(root)?.unwrap().changes;
        set_staged(root, &all, true)?;
        let staged = inspect(root)?.unwrap().changes;
        assert!(staged.iter().all(|c| c.scope == DiffScope::Index));
        set_staged(root, &staged, false)?;
        assert!(!root.join("dir/[a].txt").exists());
        assert_eq!(fs::read_to_string(root.join("dir/new.txt"))?, "one");
        assert_eq!(fs::read_to_string(root.join("other.txt"))?, "other");
        let invalid = GitChange {
            path: "../escape".into(),
            original_path: None,
            scope: DiffScope::Worktree,
            status: 'M',
        };
        assert!(set_staged(root, &[invalid], true).is_err());
        Ok(())
    }

    #[test]
    fn commits_only_staged_contents_without_touching_worktree() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        checked(root, &["init", "-b", "main"])?;
        checked(root, &["config", "user.name", "Test"])?;
        checked(root, &["config", "user.email", "test@example.invalid"])?;
        fs::write(root.join("[a].txt"), "base\n")?;
        fs::write(root.join("other.txt"), "other\n")?;
        assert!(commit(root, "not staged").is_err());
        checked(root, &["add", "--", "[a].txt"])?;
        fs::write(root.join("[a].txt"), "working\n")?;
        assert!(commit(root, "  ").is_err());
        commit(root, "fixture commit")?;
        let files = checked(root, &["ls-tree", "--name-only", "HEAD"])?;
        assert_eq!(String::from_utf8(files)?, "[a].txt\n");
        assert_eq!(checked(root, &["show", "HEAD:[a].txt"])?, b"base\n");
        assert_eq!(fs::read_to_string(root.join("[a].txt"))?, "working\n");
        assert!(
            inspect(root)?
                .unwrap()
                .changes
                .iter()
                .any(|c| c.path == Path::new("other.txt"))
        );
        Ok(())
    }
}

pub mod management;
