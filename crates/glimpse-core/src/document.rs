use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentKind {
    Markdown,
    Text,
}

impl DocumentKind {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some(extension)
                if extension.eq_ignore_ascii_case("md")
                    || extension.eq_ignore_ascii_case("markdown") =>
            {
                Self::Markdown
            }
            _ => Self::Text,
        }
    }
}

/// A successfully decoded, read-only document. No GPUI types cross this boundary.
#[derive(Debug)]
pub struct Document {
    pub path: PathBuf,
    pub kind: DocumentKind,
    pub text: String,
}
