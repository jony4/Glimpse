//! Bounded UTF-8 windows; offsets are byte offsets, never character indices.
use anyhow::{Result, ensure};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::SystemTime,
};
pub const PAGE_BYTES: u64 = 256 * 1024;
pub const FULL_BYTES: u64 = 64 * 1024 * 1024;
pub struct TextPage {
    pub path: PathBuf,
    pub text: String,
    pub start: u64,
    pub end: u64,
    pub total: u64,
    pub modified: Option<SystemTime>,
}
pub fn read_page(path: &Path, start: u64, all: bool) -> Result<TextPage> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    ensure!(metadata.is_file(), "Not a regular file");
    let total = metadata.len();
    ensure!(
        !all || total <= FULL_BYTES,
        "Full view is limited to 64 MiB. Continue using pages for this file."
    );
    ensure!(start <= total, "File changed; reopen it to restart reading");
    file.seek(SeekFrom::Start(start))?;
    let limit = if all { FULL_BYTES } else { PAGE_BYTES };
    let mut bytes = Vec::new();
    (&mut file).take(limit).read_to_end(&mut bytes)?;
    // Complete a codepoint split at the page boundary without swallowing invalid UTF-8.
    for _ in 0..3 {
        match std::str::from_utf8(&bytes) {
            Ok(_) => break,
            Err(e) if e.error_len().is_none() => {
                let mut byte = [0];
                if file.read(&mut byte)? == 0 {
                    break;
                }
                bytes.push(byte[0]);
            }
            Err(e) => return Err(e.into()),
        }
    }
    ensure!(
        !bytes.contains(&0),
        "Binary content cannot be displayed as UTF-8 text"
    );
    let end = start + bytes.len() as u64;
    let text = String::from_utf8(bytes)?;
    ensure!(
        !all || text.bytes().filter(|b| *b == b'\n').count() <= 1_000_000,
        "Full view exceeds one million rows. Continue using pages."
    );
    let after = file.metadata()?;
    ensure!(
        after.len() == total && after.modified().ok() == metadata.modified().ok(),
        "File changed while reading; retry"
    );
    Ok(TextPage {
        path: std::path::absolute(path)?,
        text,
        start,
        end,
        total,
        modified: metadata.modified().ok(),
    })
}

impl TextPage {
    pub fn fingerprint(&self) -> String {
        format!("paged:{}:{:?}", self.total, self.modified)
    }
}
