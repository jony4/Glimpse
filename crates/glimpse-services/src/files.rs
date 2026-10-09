use std::{fs::File, io::Read, path::Path};

use anyhow::{Context, Result, ensure};
use glimpse_core::{Document, DocumentKind};

/// Initial viewer limit until chunked loading and large-file UX are implemented.
pub const MAX_DOCUMENT_BYTES: u64 = 2 * 1024 * 1024;

pub fn read_document(path: &Path) -> Result<Document> {
    let file = File::open(path).with_context(|| format!("Cannot open {}", path.display()))?;
    ensure!(
        file.metadata()?.is_file(),
        "Not a regular file: {}",
        path.display()
    );
    // Limit the read itself, so a file growing after metadata inspection is bounded too.
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_DOCUMENT_BYTES,
        "File exceeds the initial 2 MiB limit: {}",
        path.display()
    );
    ensure!(
        !bytes.contains(&0),
        "Binary files are not supported: {}",
        path.display()
    );
    let text = String::from_utf8(bytes)
        .with_context(|| format!("File is not UTF-8: {}", path.display()))?;
    Ok(Document {
        path: path.to_path_buf(),
        kind: DocumentKind::from_path(path),
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reads_markdown_without_modifying_the_file() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("notes.MD");
        let content = "# 你好\n\nRead-only.\n";
        fs::write(&path, content)?;
        let document = read_document(&path)?;
        assert_eq!(document.kind, DocumentKind::Markdown);
        assert_eq!(document.text, content);
        assert_eq!(fs::read_to_string(path)?, content);
        Ok(())
    }

    #[test]
    fn rejects_unsupported_input() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("input");
        for content in [
            vec![0, 1, 2],
            vec![0xff],
            vec![b'x'; MAX_DOCUMENT_BYTES as usize + 1],
        ] {
            fs::write(&path, content)?;
            assert!(read_document(&path).is_err());
        }
        assert!(read_document(directory.path()).is_err());
        assert!(read_document(&directory.path().join("missing")).is_err());
        Ok(())
    }
}
