//! UI-independent values shared by the application and its services.

mod diff;
mod document;
mod git_management;
pub mod split_diff;
mod workspace;

pub use diff::{DiffSpan, changed_lines, commit_content, diff_content};
pub use document::{Document, DocumentKind, language_for_path};
pub use git_management::{
    Branch, GitOperation, GitRequest, GitSnapshot, GraphLayout, GraphRow, layout_graph,
};
pub use workspace::{DiffDocument, DiffScope, DirectoryEntry, GitChange, Repository};
