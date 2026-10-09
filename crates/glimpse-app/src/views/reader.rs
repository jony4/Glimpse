use glimpse_core::{Document, DocumentKind};
use gpui_kit::{
    component::{
        input::{Editor, EditorState},
        text::TextView,
    },
    *,
};

/// Owns persistent text state; rendering never re-creates the editor entity.
pub struct Reader {
    pub title: SharedString,
    markdown: Option<SharedString>,
    source: Entity<EditorState>,
    preview: bool,
}

impl Reader {
    pub fn new(document: Document, window: &mut Window, cx: &mut App) -> Self {
        let markdown = (document.kind == DocumentKind::Markdown)
            .then(|| SharedString::from(document.text.clone()));
        let source = cx.new(|cx| {
            let mut state = EditorState::new(window, cx).soft_wrap(false);
            state.set_value(document.text, window, cx);
            state.set_readonly(true, cx);
            state
        });
        Self {
            title: document.path.display().to_string().into(),
            preview: markdown.is_some(),
            markdown,
            source,
        }
    }

    pub fn can_preview(&self) -> bool {
        self.markdown.is_some()
    }

    pub fn toggle_preview(&mut self) {
        self.preview = !self.preview;
    }

    pub fn toggle_label(&self) -> &'static str {
        if self.preview { "Source" } else { "Preview" }
    }

    pub fn render(&self) -> AnyElement {
        if let Some(markdown) = self.markdown.as_ref().filter(|_| self.preview) {
            div()
                .size_full()
                .p_6()
                .child(
                    TextView::markdown("document-preview", markdown.clone())
                        .scrollable(true)
                        .size_full(),
                )
                .into_any_element()
        } else {
            Editor::new(&self.source)
                .readonly(true)
                .appearance(false)
                .bordered(false)
                .size_full()
                .into_any_element()
        }
    }
}
