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
    if matches!(name, "Makefile" | "GNUmakefile" | "makefile") {
        return "make";
    }
    if matches!(name, ".bashrc" | ".bash_profile" | ".zshrc" | ".zprofile") {
        return "bash";
    }
    if matches!(name, ".eslintrc" | ".babelrc" | ".czrc") {
        return "json";
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
        "json" | "jsonc" | "jsonl" | "code-workspace" | "tsbuildinfo" => "json",
        "toml" => "toml",
        "py" | "pyi" | "pyx" | "pxd" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "cu" | "cuh" | "metal" => "cpp",
        "css" | "less" => "css",
        // HTML provides basic tag highlighting, not full Vue/XML language semantics.
        "html" | "htm" | "vue" | "xml" | "mtlx" | "tmx" | "hbs" => "html",
        "yml" | "yaml" | "ocio" => "yaml",
        "sh" | "bash" | "zsh" | "command" => "bash",
        "swift" => "swift",
        "java" => "java",
        "sql" => "sql",
        "mk" => "make",
        // C-like syntax only; these retain shader-specific tokens as plain text.
        "glsl" | "vert" | "frag" | "geom" | "osl" => "c",
        "md" | "markdown" => "markdown",
        "diff" | "patch" => "diff",
        _ => "plain",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_source_aliases_without_misclassifying_unknown_files() {
        for (path, language) in [
            ("Main.JAVA", "java"),
            ("schema.sql", "sql"),
            ("Component.vue", "html"),
            ("material.mtlx", "html"),
            ("shader.glsl", "c"),
            ("kernel.cu", "cpp"),
            ("types.pxd", "python"),
            ("events.jsonl", "json"),
            ("Makefile", "make"),
            (".zshrc", "bash"),
            ("table.csv", "plain"),
            ("unknown.blend", "plain"),
        ] {
            assert_eq!(language_for_path(Path::new(path)), language, "{path}");
        }
    }
}
