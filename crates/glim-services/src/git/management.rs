//! Explicit branch/remote operations; never force, auto-stash or rewrite history.
use super::{checked, run};
use anyhow::{Context, Result, ensure};
use glim_core::{Branch, GitOperation, GitRequest, GitSnapshot, GraphRow};
use std::{ffi::OsStr, path::Path};
fn optional(root: &Path, args: &[&str]) -> Result<Option<String>> {
    let result = run(root, &args.iter().map(OsStr::new).collect::<Vec<_>>())?;
    if result.code == Some(0) {
        Ok(Some(String::from_utf8(result.bytes)?.trim().to_owned()))
    } else {
        Ok(None)
    }
}
pub fn snapshot(root: &Path) -> Result<GitSnapshot> {
    let head = optional(root, &["rev-parse", "--verify", "HEAD"])?;
    let branch = optional(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    let upstream = optional(
        root,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    )?;
    let mut result = GitSnapshot {
        head,
        branch,
        upstream,
        ..Default::default()
    };
    if result.upstream.is_some() && result.head.is_some() {
        let count = checked(
            root,
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        )?;
        let count = String::from_utf8(count)?;
        let mut counts = count.split_whitespace();
        result.ahead = counts.next().context("Missing ahead count")?.parse()?;
        result.behind = counts.next().context("Missing behind count")?.parse()?;
    }
    let refs = checked(
        root,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--count=1000",
            "--format=%(refname)%00%(refname:short)%00%(HEAD)%00%(upstream:short)%00%(symref)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;
    for line in String::from_utf8(refs)?.lines() {
        let fields: Vec<_> = line.split('\0').collect();
        ensure!(fields.len() == 5, "Invalid Git branch record");
        if !fields[4].is_empty() {
            continue;
        }
        result.branches.push(Branch {
            reference: fields[0].into(),
            name: fields[1].into(),
            current: fields[2] == "*",
            upstream: fields[3].into(),
            remote: fields[0].starts_with("refs/remotes/"),
        });
    }
    result.remotes = String::from_utf8(checked(root, &["remote"])?)?
        .lines()
        .map(str::to_owned)
        .collect();
    let stashes = checked(root, &["stash", "list", "-n", "100", "--format=%H%x00%gs"])?;
    for line in String::from_utf8_lossy(&stashes).lines() {
        if let Some((oid, subject)) = line.split_once('\0') {
            result.stashes.push((oid.into(), subject.into()));
        }
    }
    result.merging = optional(root, &["rev-parse", "--verify", "MERGE_HEAD"])?.is_some();
    Ok(result)
}
fn branch_name(root: &Path, name: &str) -> Result<()> {
    ensure!(
        !name.starts_with('-') && !name.starts_with("@{") && !name.trim().is_empty(),
        "Enter a literal branch name"
    );
    checked(root, &["check-ref-format", "--branch", name])?;
    Ok(())
}
fn commit_id(root: &Path, reference: &str) -> Result<String> {
    let rev = format!("{reference}^{{commit}}");
    Ok(String::from_utf8(checked(
        root,
        &["rev-parse", "--verify", "--end-of-options", &rev],
    )?)?
    .trim()
    .to_owned())
}
fn clean(root: &Path) -> Result<()> {
    ensure!(
        checked(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"]
        )?
        .is_empty(),
        "Commit or stash local changes before this operation. No changes have been discarded."
    );
    Ok(())
}
fn push_upstream(root: &Path, state: &GitSnapshot) -> Result<()> {
    let branch = state
        .branch
        .as_deref()
        .context("Checkout a branch before pushing")?;
    let remote = String::from_utf8(checked(
        root,
        &["config", "--get", &format!("branch.{branch}.remote")],
    )?)?;
    let target = String::from_utf8(checked(
        root,
        &["config", "--get", &format!("branch.{branch}.merge")],
    )?)?;
    ensure!(
        target.trim().starts_with("refs/heads/"),
        "Upstream must be a branch"
    );
    checked(
        root,
        &[
            "push",
            "--",
            remote.trim(),
            &format!("HEAD:{}", target.trim()),
        ],
    )?;
    Ok(())
}
pub fn execute(root: &Path, request: &GitRequest) -> Result<()> {
    let state = snapshot(root)?;
    ensure!(
        state.head == request.expected_head && state.branch == request.expected_branch,
        "HEAD changed since this action was selected. Refresh and try again."
    );
    match &request.operation {
        GitOperation::Fetch => {
            checked(root, &["fetch", "--all"])?;
        }
        GitOperation::Pull | GitOperation::Sync => {
            clean(root)?;
            ensure!(
                state.upstream.is_some(),
                "No upstream configured. Use Publish Branch first."
            );
            checked(
                root,
                &[
                    "-c",
                    "pull.autostash=false",
                    "-c",
                    "merge.autostash=false",
                    "pull",
                    "--ff-only",
                    "--no-rebase",
                ],
            )?;
            if matches!(request.operation, GitOperation::Sync) {
                push_upstream(root, &state)
                    .context("Pull completed, but push failed. The pulled changes remain local")?;
            }
        }
        GitOperation::Push => {
            ensure!(
                state.upstream.is_some(),
                "No upstream configured. Use Publish Branch first."
            );
            push_upstream(root, &state)?;
        }
        GitOperation::Publish(remote) => {
            ensure!(state.remotes.contains(remote), "Remote no longer exists");
            let branch = state
                .branch
                .as_deref()
                .context("Checkout a branch before publishing")?;
            checked(
                root,
                &[
                    "push",
                    "--set-upstream",
                    "--",
                    remote,
                    &format!("HEAD:refs/heads/{branch}"),
                ],
            )?;
        }
        GitOperation::Switch(name) => {
            branch_name(root, name)?;
            clean(root)?;
            checked(root, &["switch", "--no-guess", name])?;
        }
        GitOperation::Track { reference, name } => {
            branch_name(root, name)?;
            clean(root)?;
            ensure!(
                state
                    .branches
                    .iter()
                    .any(|b| b.remote && &b.reference == reference),
                "Remote branch no longer exists"
            );
            checked(root, &["switch", "--track", "-c", name, reference])?;
        }
        GitOperation::Create { name, start } => {
            branch_name(root, name)?;
            clean(root)?;
            if let Some(start) = start {
                let oid = commit_id(root, start)?;
                checked(root, &["switch", "-c", name, &oid])?;
            } else {
                checked(root, &["switch", "-c", name])?;
            }
        }
        GitOperation::Rename(name) => {
            branch_name(root, name)?;
            ensure!(state.branch.is_some(), "Checkout a branch before renaming");
            checked(root, &["branch", "-m", name])?;
        }
        GitOperation::Delete(name) => {
            branch_name(root, name)?;
            checked(root, &["branch", "-d", "--", name])?;
        }
        GitOperation::Merge(reference) => {
            clean(root)?;
            let oid = commit_id(root, reference)?;
            checked(
                root,
                &["-c", "merge.autostash=false", "merge", "--no-edit", &oid],
            )?;
        }
        GitOperation::AbortMerge => {
            ensure!(state.merging, "No merge is in progress");
            checked(root, &["merge", "--abort"])?;
        }
        GitOperation::Stash => {
            checked(
                root,
                &["stash", "push", "--include-untracked", "-m", "Glim stash"],
            )?;
        }
        GitOperation::ApplyStash(oid) => {
            ensure!(
                state.stashes.iter().any(|(id, _)| id == oid),
                "Stash no longer exists"
            );
            clean(root)?;
            checked(root, &["stash", "apply", "--index", oid])?;
        }
    }
    Ok(())
}
pub fn graph(root: &Path, limit: usize, all: bool) -> Result<Vec<GraphRow>> {
    ensure!(
        (1..=1000).contains(&limit),
        "Graph limit must be between 1 and 1000 commits"
    );
    if checked(
        root,
        &["for-each-ref", "--count=1", "--format=%(objectname)"],
    )?
    .is_empty()
        && optional(root, &["rev-parse", "--verify", "HEAD"])?.is_none()
    {
        return Ok(Vec::new());
    }
    let count = format!("--max-count={limit}");
    let scope = if all { "--all" } else { "HEAD" };
    if !all && optional(root, &["rev-parse", "--verify", "HEAD"])?.is_none() {
        return Ok(Vec::new());
    }
    let bytes = checked(
        root,
        &[
            "log",
            "--topo-order",
            "--no-color",
            "--decorate=short",
            &count,
            "--format=%H%x00%h%x00%an%x00%ar%x00%d%x00%P%x00%s",
            scope,
            "--",
        ],
    )?;
    let mut rows = Vec::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        let fields: Vec<_> = line.splitn(7, '\0').collect();
        ensure!(fields.len() == 7, "Invalid Git history record");
        rows.push(GraphRow {
            commit: Some(fields[0].into()),
            short_id: fields[1].into(),
            author: fields[2].into(),
            age: fields[3].into(),
            references: fields[4].trim().into(),
            parents: fields[5].split_whitespace().map(str::to_owned).collect(),
            subject: fields[6].into(),
            layout: Default::default(),
        });
    }
    glim_core::layout_graph(&mut rows);
    Ok(rows)
}
pub fn commit_details(root: &Path, oid: &str) -> Result<String> {
    ensure!(
        matches!(oid.len(), 40 | 64) && oid.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid commit ID"
    );
    let bytes = checked(
        root,
        &[
            "show",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--format=fuller",
            "--stat",
            "--patch",
            "--diff-merges=first-parent",
            oid,
            "--",
        ],
    )?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[derive(Clone)]
pub struct CommitFile {
    pub path: std::path::PathBuf,
    pub previous_path: Option<std::path::PathBuf>,
    pub status: char,
}

/// Read names only; expanding a commit never loads its patch or file contents.
pub fn commit_files(root: &Path, oid: &str) -> Result<Vec<CommitFile>> {
    ensure!(
        matches!(oid.len(), 40 | 64) && oid.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid commit ID"
    );
    let bytes = checked(
        root,
        &[
            "diff-tree",
            "--root",
            "--first-parent",
            "-m",
            "-r",
            "--no-commit-id",
            "--name-status",
            "-z",
            "-M",
            oid,
            "--",
        ],
    )?;
    let mut parts = bytes.split(|b| *b == 0).filter(|p| !p.is_empty());
    let mut files = Vec::new();
    while let Some(status) = parts.next() {
        let status = *status.first().context("Missing change status")? as char;
        let first = super::path_from_bytes(parts.next().context("Missing changed path")?);
        let (path, previous_path) = if matches!(status, 'R' | 'C') {
            (
                super::path_from_bytes(parts.next().context("Missing destination path")?),
                Some(first),
            )
        } else {
            (first, None)
        };
        files.push(CommitFile {
            path,
            previous_path,
            status,
        });
    }
    Ok(files)
}

pub fn commit_file_details(root: &Path, oid: &str, path: &Path) -> Result<String> {
    let file = commit_files(root, oid)?
        .into_iter()
        .find(|file| file.path == path)
        .context("File is not part of this commit")?;
    let mut args: Vec<&OsStr> = [
        "show",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--format=fuller",
        "--patch",
        "--diff-merges=first-parent",
        "-M",
        oid,
        "--",
    ]
    .into_iter()
    .map(OsStr::new)
    .collect();
    args.push(file.path.as_os_str());
    if let Some(previous) = &file.previous_path {
        args.push(previous.as_os_str());
    }
    let output = run(root, &args)?;
    ensure!(output.code == Some(0), "{}", output.stderr);
    Ok(String::from_utf8_lossy(&output.bytes).into_owned())
}
