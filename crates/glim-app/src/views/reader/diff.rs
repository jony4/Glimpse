use super::source::SourceReader;
use glim_core::{DiffDocument, diff_content, split_diff::split_patch};
use gpui_kit::*;
pub(super) struct DiffReader {
    pub source: SourceReader,
    pub split: Option<Entity<crate::views::split_diff::SplitDiff>>,
    pub side_by_side: bool,
}
impl DiffReader {
    pub fn new(document: &DiffDocument, window: &mut Window, cx: &mut App) -> Self {
        let (mut text, spans) = diff_content(&document.patch);
        if text == "Binary file changed."
            && document
                .change
                .path
                .file_name()
                .is_some_and(|name| name == ".DS_Store")
        {
            text = ".DS_Store changed.\n\nThis is macOS Finder metadata (folder layout and icon positions), stored in a binary format. Git cannot show a line-by-line text diff.\n\nUse View file to inspect its bytes. If this file should not be versioned, ignore it in Git; already tracked files must also be removed from the index.".into();
        }
        let source =
            SourceReader::new(&text, "plain", true, spans, ScrollHandle::new(), window, cx);
        let split = split_patch(&document.patch)
            .map(|patch| cx.new(|cx| crate::views::split_diff::SplitDiff::new(patch, window, cx)));
        Self {
            source,
            side_by_side: split.is_some()
                && cx
                    .try_global::<crate::app::ReaderPreferences>()
                    .and_then(|p| p.diff_side_by_side)
                    .unwrap_or(true),
            split,
        }
    }
    pub fn render(&mut self, cx: &mut App) -> AnyElement {
        if self.side_by_side
            && let Some(split) = &self.split
        {
            split.clone().into_any_element()
        } else {
            self.source.render(cx)
        }
    }
}
