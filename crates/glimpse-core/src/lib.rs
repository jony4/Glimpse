//! UI-independent values shared by the application and its services.

mod document;

pub use document::{Document, DocumentKind, language_for_path};

mod workspace;
pub use workspace::{DiffDocument, DiffScope, DirectoryEntry, GitChange, Repository};

mod diff;
pub use diff::{DiffSpan, changed_lines};

pub mod split_diff;
