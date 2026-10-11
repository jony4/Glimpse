use anyhow::{Result, ensure};
use glim_core::{GitOperation, GitRequest};
use glim_services::{git::management, paged, safetensors};
use std::{fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let configuration = tempfile::NamedTempFile::new()?;
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", configuration.path())
        .output()?;
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
fn repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-b", "main"])?;
    git(dir.path(), &["config", "user.name", "Glim Test"])?;
    git(
        dir.path(),
        &["config", "user.email", "test@example.invalid"],
    )?;
    git(dir.path(), &["config", "commit.gpgsign", "false"])?;
    git(dir.path(), &["config", "core.autocrlf", "false"])?;
    git(dir.path(), &["config", "core.hooksPath", ".git/no-hooks"])?;
    fs::write(dir.path().join("file.txt"), "base\n")?;
    git(dir.path(), &["add", "."])?;
    git(dir.path(), &["commit", "-m", "initial"])?;
    Ok(dir)
}
fn execute(root: &Path, operation: GitOperation) -> Result<()> {
    let state = management::snapshot(root)?;
    management::execute(
        root,
        &GitRequest {
            operation,
            expected_head: state.head,
            expected_branch: state.branch,
        },
    )
}
#[test]
fn branches_graph_and_stale_requests() -> Result<()> {
    let dir = repo()?;
    let root = dir.path();
    let original = management::snapshot(root)?;
    execute(
        root,
        GitOperation::Create {
            name: "feature".into(),
            start: None,
        },
    )?;
    execute(root, GitOperation::Rename("renamed".into()))?;
    assert_eq!(
        management::snapshot(root)?.branch.as_deref(),
        Some("renamed")
    );
    let stale = GitRequest {
        operation: GitOperation::Switch("main".into()),
        expected_head: original.head,
        expected_branch: original.branch,
    };
    assert!(management::execute(root, &stale).is_err());
    fs::write(root.join("file.txt"), "draft\n")?;
    assert!(execute(root, GitOperation::Switch("main".into())).is_err());
    execute(root, GitOperation::Stash)?;
    let stash = management::snapshot(root)?.stashes[0].0.clone();
    execute(root, GitOperation::Switch("main".into()))?;
    execute(root, GitOperation::Delete("renamed".into()))?;
    execute(root, GitOperation::ApplyStash(stash))?;
    assert_eq!(fs::read_to_string(root.join("file.txt"))?, "draft\n");
    assert_eq!(management::snapshot(root)?.stashes.len(), 1);
    let rows = management::graph(root, 200, true)?;
    let commit = rows
        .iter()
        .find(|row| row.subject == "initial")
        .and_then(|row| row.commit.as_deref())
        .unwrap();
    assert!(management::commit_details(root, commit)?.contains("initial"));
    assert!(management::commit_details(root, "--help").is_err());
    Ok(())
}
#[test]
fn remote_publish_fetch_pull_and_push() -> Result<()> {
    let dir = repo()?;
    let remote = tempfile::tempdir()?;
    git(remote.path(), &["init", "--bare", "-b", "main"])?;
    git(
        dir.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    )?;
    execute(dir.path(), GitOperation::Publish("origin".into()))?;
    let peer = repo()?;
    git(
        peer.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    )?;
    git(peer.path(), &["fetch", "origin"])?;
    git(peer.path(), &["reset", "--hard", "origin/main"])?;
    fs::write(peer.path().join("remote.txt"), "remote\n")?;
    git(peer.path(), &["add", "."])?;
    git(peer.path(), &["commit", "-m", "remote update"])?;
    git(peer.path(), &["push", "origin", "main"])?;
    execute(dir.path(), GitOperation::Fetch)?;
    assert_eq!(management::snapshot(dir.path())?.behind, 1);
    execute(dir.path(), GitOperation::Pull)?;
    assert_eq!(
        fs::read_to_string(dir.path().join("remote.txt"))?,
        "remote\n"
    );
    fs::write(dir.path().join("local.txt"), "local\n")?;
    git(dir.path(), &["add", "."])?;
    git(dir.path(), &["commit", "-m", "local update"])?;
    execute(dir.path(), GitOperation::Push)?;
    assert_eq!(
        git(dir.path(), &["rev-parse", "HEAD"])?,
        git(remote.path(), &["rev-parse", "main"])?
    );
    Ok(())
}
#[test]
fn paging_completes_utf8_and_bounds_full_reads() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("large.json");
    let text = "a".repeat(paged::PAGE_BYTES as usize - 1) + "界tail";
    fs::write(&path, &text)?;
    let first = paged::read_page(&path, 0, false)?;
    assert!(first.text.ends_with('界'));
    let next = paged::read_page(&path, first.end, false)?;
    assert_eq!(first.text + &next.text, text);
    fs::File::create(&path)?.set_len(paged::FULL_BYTES + 1)?;
    assert!(paged::read_page(&path, 0, true).is_err());
    fs::write(&path, [0xff])?;
    assert!(paged::read_page(&path, 0, false).is_err());
    Ok(())
}
#[test]
fn safetensors_metadata_without_loading_large_payload() -> Result<()> {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new()?;
    let header = br#"{"weight":{"dtype":"F32","shape":[2],"data_offsets":[0,8]},"__metadata__":{"format":"pt"}}"#;
    file.write_all(&(header.len() as u64).to_le_bytes())?;
    file.write_all(header)?;
    file.as_file().set_len(2 * 1024 * 1024 * 1024)?;
    let preview = safetensors::read_preview(file.path())?;
    assert!(preview.text.contains("Parameters      2"));
    assert!(preview.text.contains("format: pt"));
    assert!(preview.text.len() < 4096);
    file.as_file().set_len(8 + header.len() as u64 + 4)?;
    assert!(safetensors::read_preview(file.path()).is_err());
    Ok(())
}
