use super::source::SourceReader;
use glimpse_core::{DiffDocument, diff_content, split_diff::split_patch};
use gpui_kit::*;
pub(super) struct DiffReader {
    pub source: SourceReader,
    pub split: Option<Entity<crate::views::split_diff::SplitDiff>>,
    pub side_by_side: bool,
}
impl DiffReader {
    pub fn new(document: &DiffDocument, window: &mut Window, cx: &mut App) -> Self {
        let (text, spans) = diff_content(&document.patch);
        let source =
            SourceReader::new(&text, "plain", true, spans, ScrollHandle::new(), window, cx);
        let split = split_patch(&document.patch)
            .map(|patch| cx.new(|cx| crate::views::split_diff::SplitDiff::new(patch, window, cx)));
        Self {
            source,
            side_by_side: split.is_some(),
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
