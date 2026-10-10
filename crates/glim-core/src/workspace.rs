use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiffScope {
    Worktree,
    Index,
    Untracked,
    Conflict,
}

impl DiffScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::Worktree => "Unstaged",
            Self::Index => "Staged",
            Self::Untracked => "Untracked",
            Self::Conflict => "Conflict",
        }
    }
}

/// Paths are relative to the repository root, including for linked worktrees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitChange {
    pub path: PathBuf,
    pub original_path: Option<PathBuf>,
    pub status: char,
    pub scope: DiffScope,
}

#[derive(Clone, Debug)]
pub struct Repository {
    pub root: PathBuf,
    pub branch: String,
    pub changes: Vec<GitChange>,
}

#[derive(Debug)]
pub struct DiffDocument {
    pub change: GitChange,
    pub patch: String,
}
