//! Bounded, read-only byte inspection for dotfiles that cannot be decoded as text.
use anyhow::{Result, ensure};
use std::{
    fmt::Write as _,
    io::Read,
    path::{Path, PathBuf},
};

pub const PREVIEW_BYTES: usize = 64 * 1024;
pub struct BytePreview {
    pub path: PathBuf,
    pub text: String,
}
pub fn read_preview(path: &Path) -> Result<BytePreview> {
    ensure!(path.is_file(), "Not a regular file");
    let file = std::fs::File::open(path)?;
    ensure!(file.metadata()?.is_file(), "Not a regular file");
    let mut bytes = Vec::new();
    file.take(PREVIEW_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let truncated = bytes.len() > PREVIEW_BYTES;
    bytes.truncate(PREVIEW_BYTES);
    let mut text = format!(
        "Read-only byte preview · {} bytes shown{}\nOffset    Hex bytes                                         ASCII\n",
        bytes.len(),
        if truncated {
            " · truncated to first 64 KiB"
        } else {
            ""
        }
    );
    for (row, chunk) in bytes.chunks(16).enumerate() {
        write!(text, "{:08x}  ", row * 16)?;
        for i in 0..16 {
            match chunk.get(i) {
                Some(byte) => write!(text, "{byte:02x} ")?,
                None => text.push_str("   "),
            }
        }
        text.push(' ');
        for byte in chunk {
            text.push(if byte.is_ascii_graphic() || *byte == b' ' {
                char::from(*byte)
            } else {
                '.'
            });
        }
        text.push('\n');
    }
    Ok(BytePreview {
        path: std::path::absolute(path)?,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_binary_is_bounded_and_never_modified() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(".DS_Store");
        let bytes = [0, 0xff, b'A', b'\n'];
        std::fs::write(&path, bytes)?;
        let preview = read_preview(&path)?;
        assert!(preview.text.contains("00000000  00 ff 41 0a"));
        assert!(preview.text.contains("..A."));
        assert_eq!(std::fs::read(&path)?, bytes);
        std::fs::write(&path, vec![0; PREVIEW_BYTES + 100])?;
        let preview = read_preview(&path)?;
        assert!(preview.text.contains("truncated"));
        assert!(!preview.text.contains("00010000"));
        assert!(read_preview(dir.path()).is_err());
        Ok(())
    }
}
