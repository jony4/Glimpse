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

/// Tree-sitter registry names; unsupported extensions stay readable as plain text.
pub fn language_for_path(path: &Path) -> &'static str {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if matches!(name, "Dockerfile" | "Makefile") {
        return "plain";
    }
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "rs" => "rust",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "json" | "jsonc" => "json",
        "toml" => "toml",
        "py" | "pyi" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "css" => "css",
        "html" | "htm" => "html",
        "yml" | "yaml" => "yaml",
        "sh" | "bash" | "zsh" => "bash",
        "swift" => "swift",
        "md" | "markdown" => "markdown",
        "diff" | "patch" => "diff",
        _ => "plain",
    }
}
