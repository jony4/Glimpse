//! Git management values only; execution belongs to services.
#[derive(Clone, Debug)]
pub struct Branch {
    pub name: String,
    pub reference: String,
    pub remote: bool,
    pub current: bool,
    pub upstream: String,
}
#[derive(Clone, Debug, Default)]
pub struct GitSnapshot {
    pub head: Option<String>,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub branches: Vec<Branch>,
    pub remotes: Vec<String>,
    pub stashes: Vec<(String, String)>,
    pub merging: bool,
}
#[derive(Clone, Debug)]
pub enum GitOperation {
    Fetch,
    Pull,
    Push,
    Sync,
    Publish(String),
    Switch(String),
    Track { reference: String, name: String },
    Create { name: String, start: Option<String> },
    Rename(String),
    Delete(String),
    Merge(String),
    AbortMerge,
    Stash,
    ApplyStash(String),
}
impl GitOperation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fetch => "Fetch",
            Self::Pull => "Pull",
            Self::Push => "Push",
            Self::Sync => "Sync Changes",
            Self::Publish(_) => "Publish Branch",
            Self::Switch(_) | Self::Track { .. } => "Checkout Branch",
            Self::Create { .. } => "Create Branch",
            Self::Rename(_) => "Rename Branch",
            Self::Delete(_) => "Delete Branch",
            Self::Merge(_) => "Merge Branch",
            Self::AbortMerge => "Abort Merge",
            Self::Stash => "Stash (Include Untracked)",
            Self::ApplyStash(_) => "Apply Stash",
        }
    }
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::Publish(name)
            | Self::Switch(name)
            | Self::Rename(name)
            | Self::Delete(name)
            | Self::Merge(name)
            | Self::ApplyStash(name) => Some(name),
            Self::Track { name, .. } | Self::Create { name, .. } => Some(name),
            _ => None,
        }
    }
    pub fn changes_worktree(&self) -> bool {
        matches!(
            self,
            Self::Pull
                | Self::Sync
                | Self::Switch(_)
                | Self::Track { .. }
                | Self::Create { .. }
                | Self::Merge(_)
                | Self::AbortMerge
                | Self::Stash
                | Self::ApplyStash(_)
        )
    }
    pub fn confirmation(&self) -> Option<&'static str> {
        match self {
            Self::Delete(_) => Some(
                "Delete this local branch? Git will refuse if it is checked out or not fully merged. Remote branches are not deleted.",
            ),
            Self::Merge(_) => Some(
                "Merge the selected branch into the current branch? This may create a merge commit or leave conflicts to resolve.",
            ),
            Self::AbortMerge => Some(
                "Abort the current merge? Edits made while resolving this merge may be discarded.",
            ),
            Self::Sync => Some(
                "Pull with fast-forward only, then push the current branch to its configured upstream? If pushing fails, the successful pull is kept.",
            ),
            Self::Stash => Some(
                "Store tracked and untracked changes in a stash and clean them from the working tree? Ignored files are not included.",
            ),
            Self::ApplyStash(_) => Some(
                "Apply this stash to the current working tree? The stash is kept, including if conflicts occur.",
            ),
            _ => None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct GitRequest {
    pub operation: GitOperation,
    pub expected_head: Option<String>,
    pub expected_branch: Option<String>,
}
#[derive(Clone, Debug)]
pub struct GraphRow {
    pub graph: String,
    pub commit: Option<String>,
    pub short_id: String,
    pub author: String,
    pub age: String,
    pub references: String,
    pub subject: String,
}
