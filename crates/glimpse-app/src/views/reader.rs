use glimpse_core::{
    DiffDocument, DiffSpan, Document, DocumentKind, changed_lines, language_for_path,
};
use gpui_kit::{
    base::TextView,
    component::{
        ActiveTheme,
        input::{Editor, EditorState, TextDecoration, TextDecorationCollection},
    },
    *,
};

/// Owns persistent text state; rendering never re-creates the editor entity.
pub struct Reader {
    pub title: SharedString,
    pub path: std::path::PathBuf,
    pub diff: Option<glimpse_core::GitChange>,
    pub language: &'static str,
    markdown: Option<SharedString>,
    base_url: Option<url::Url>,
    source: Entity<EditorState>,
    preview: bool,
    diff_spans: Vec<DiffSpan>,
    decorations: Option<TextDecorationCollection>,
    palette: Option<(Hsla, Hsla)>,
}

impl Reader {
    pub fn new(document: Document, window: &mut Window, cx: &mut App) -> Self {
        let markdown = (document.kind == DocumentKind::Markdown)
            .then(|| SharedString::from(document.text.clone()));
        let language = language_for_path(&document.path);
        let source = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language(language)
                .soft_wrap(false);
            state.set_value(document.text, window, cx);
            state.set_readonly(true, cx);
            state
        });
        Self {
            title: document.path.display().to_string().into(),
            base_url: url::Url::from_file_path(&document.path).ok(),
            path: document.path,
            diff: None,
            language,
            diff_spans: Vec::new(),
            decorations: None,
            palette: None,
            preview: markdown.is_some(),
            markdown,
            source,
        }
    }

    pub fn from_diff(
        document: DiffDocument,
        root: &std::path::Path,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let title = format!(
            "{} · {}",
            document.change.path.display(),
            document.change.scope.label()
        );
        let diff_spans = changed_lines(&document.patch);
        let source = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language("plain")
                .line_number(false)
                .soft_wrap(false);
            state.set_value(
                if document.patch.is_empty() {
                    "No textual diff. The change may have been refreshed or resolved.".to_string()
                } else {
                    document.patch
                },
                window,
                cx,
            );
            state.set_readonly(true, cx);
            state
        });
        let decorations = source.update(cx, |state, cx| {
            state.create_decorations_collection(Vec::new(), cx)
        });
        Self {
            diff_spans,
            decorations: Some(decorations),
            palette: None,
            title: title.into(),
            path: root.join(&document.change.path),
            diff: Some(document.change),
            language: "diff",
            markdown: None,
            base_url: None,
            source,
            preview: false,
        }
    }

    pub fn focus_source(&self, window: &mut Window, cx: &mut App) {
        self.source.update(cx, |state, cx| state.focus(window, cx));
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

    pub fn render(&mut self, cx: &mut App) -> AnyElement {
        let palette = (cx.theme().green, cx.theme().red);
        if self.palette != Some(palette)
            && let Some(decorations) = &self.decorations
        {
            decorations.set(
                self.diff_spans
                    .iter()
                    .map(|span| {
                        let color = if span.addition { palette.0 } else { palette.1 };
                        TextDecoration::new(
                            span.range.clone(),
                            HighlightStyle {
                                color: Some(color),
                                background_color: Some(color.opacity(0.10)),
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
                cx,
            );
            self.palette = Some(palette);
        }

        if let Some(markdown) = self.markdown.as_ref().filter(|_| self.preview) {
            let base = self.base_url.clone();
            div()
                .size_full()
                .p_6()
                .child(
                    TextView::markdown("document-preview", markdown.clone())
                        .image_source(move |uri| {
                            base.as_ref()
                                .and_then(|base| base.join(uri.as_ref()).ok())
                                .and_then(|url| url.to_file_path().ok())
                                .map(ImageSource::from)
                                .unwrap_or_else(|| uri.clone().into())
                        })
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
