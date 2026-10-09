use super::minimap::Minimap;
use glimpse_core::{
    DiffDocument, DiffSpan, Document, DocumentKind, changed_lines, language_for_path,
};
use gpui_kit::component::scroll::Scrollbar;
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
    pub repository_root: Option<std::path::PathBuf>,
    failure: Option<SharedString>,
    image: Option<std::sync::Arc<Image>>,
    pub snapshot: String,
    pub split: Option<Entity<super::split_diff::SplitDiff>>,
    pub side_by_side: bool,
    pub title: SharedString,
    pub path: std::path::PathBuf,
    pub diff: Option<glimpse_core::GitChange>,
    markdown: Option<SharedString>,
    base_url: Option<url::Url>,
    source: Entity<EditorState>,
    preview: bool,
    preview_scroll: ScrollHandle,
    minimap: Entity<Minimap>,
    diff_spans: Vec<DiffSpan>,
    decorations: Option<TextDecorationCollection>,
    palette: Option<(Hsla, Hsla)>,
}

impl Reader {
    pub fn new(document: Document, window: &mut Window, cx: &mut App) -> Self {
        let markdown = (document.kind == DocumentKind::Markdown)
            .then(|| SharedString::from(document.text.clone()));
        let language = language_for_path(&document.path);
        let map_text = document.text.clone();
        let source = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language(language)
                .soft_wrap(false)
                .folding(false)
                .scroll_beyond_last_line(Some(0));
            state.set_value(document.text, window, cx);
            state.set_readonly(true, cx);
            state
        });
        let preview_scroll = ScrollHandle::new();
        let minimap =
            cx.new(|cx| Minimap::new(source.clone(), &map_text, false, preview_scroll.clone(), cx));
        Self {
            repository_root: None,
            failure: None,
            image: None,
            snapshot: map_text,
            split: None,
            side_by_side: false,
            preview_scroll,
            minimap,
            title: document.path.display().to_string().into(),
            base_url: url::Url::from_file_path(&document.path).ok(),
            path: document.path,
            diff: None,
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
        let map_text = document.patch.clone();
        let source = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language("plain")
                .line_number(false)
                .soft_wrap(false)
                .folding(false)
                .scroll_beyond_last_line(Some(0));
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
        let preview_scroll = ScrollHandle::new();
        let minimap =
            cx.new(|cx| Minimap::new(source.clone(), &map_text, true, preview_scroll.clone(), cx));
        let split = glimpse_core::split_diff::split_patch(&map_text)
            .map(|patch| cx.new(|cx| super::split_diff::SplitDiff::new(patch, window, cx)));
        Self {
            repository_root: Some(root.to_path_buf()),
            failure: None,
            image: None,
            snapshot: map_text,
            side_by_side: split.is_some(),
            split,
            preview_scroll,
            minimap,
            diff_spans,
            decorations: Some(decorations),
            palette: None,
            title: title.into(),
            path: root.join(&document.change.path),
            diff: Some(document.change),
            markdown: None,
            base_url: None,
            source,
            preview: false,
        }
    }

    pub fn from_image(
        document: glimpse_services::media::ImageDocument,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let mut reader = Self::new(
            Document {
                path: document.path,
                kind: DocumentKind::Text,
                text: String::new(),
            },
            window,
            cx,
        );
        reader.snapshot = document.fingerprint;
        reader.image = Some(std::sync::Arc::new(Image::from_bytes(
            ImageFormat::Png,
            document.png,
        )));
        reader
    }

    pub fn unavailable(
        path: std::path::PathBuf,
        reason: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let mut reader = Self::new(
            Document {
                path,
                kind: DocumentKind::Text,
                text: String::new(),
            },
            window,
            cx,
        );
        reader.failure = Some(reason.into());
        reader
    }

    pub fn is_unavailable(&self) -> bool {
        self.failure.is_some()
    }

    pub fn inherit_view(&mut self, old: &Self, cx: &mut App) {
        self.preview = old.preview && self.markdown.is_some();
        self.side_by_side = old.side_by_side && self.split.is_some();
        if let (Some(new), Some(old)) = (&self.split, &old.split) {
            let offset = old.read(cx).offset(cx);
            new.update(cx, |v, cx| v.restore_offset(offset, cx));
        }
        let offset = old.source.read(cx).scroll_offset();
        self.source
            .update(cx, |state, cx| state.set_scroll_offset(offset, cx));
        self.preview_scroll.set_offset(old.preview_scroll.offset());
    }

    pub fn focus_source(&self, window: &mut Window, cx: &mut App) {
        self.source.update(cx, |state, cx| state.focus(window, cx));
    }

    pub fn can_preview(&self) -> bool {
        self.markdown.is_some()
    }

    pub fn set_preview(&mut self, preview: bool) {
        self.preview = preview;
    }

    pub fn toggle_label(&self) -> &'static str {
        if self.preview { "Source" } else { "Preview" }
    }

    pub fn render(&mut self, cx: &mut App) -> AnyElement {
        if let Some(image) = &self.image {
            return div()
                .relative()
                .size_full()
                .overflow_hidden()
                .child(
                    div().absolute().inset_0().p_6().child(
                        img(image.clone())
                            .absolute()
                            .inset_0()
                            .size_full()
                            .min_w_0()
                            .min_h_0()
                            .object_fit(ObjectFit::Contain),
                    ),
                )
                .into_any_element();
        }
        if let Some(reason) = &self.failure {
            use gpui_kit::component::{
                button::{Button, ButtonVariants},
                v_flex,
            };
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .p_8()
                .child(div().text_xl().child("暂时打不开这个文件"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(reason.clone()),
                )
                .child(
                    Button::new("unsupported-issue")
                        .ghost()
                        .label("在 GitHub 提交 Issue")
                        .on_click(|_, _, cx| {
                            cx.open_url("https://github.com/jony4/Glimpse/issues/new")
                        }),
                )
                .into_any_element();
        }
        if self.side_by_side
            && let Some(split) = &self.split
        {
            return split.clone().into_any_element();
        }

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

        self.minimap
            .update(cx, |map, cx| map.set_preview(self.preview, cx));
        let content = if let Some(markdown) = self.markdown.as_ref().filter(|_| self.preview) {
            let base = self.base_url.clone();
            let minimap = self.minimap.clone();
            div()
                .id(("markdown-scroll", self.source.entity_id()))
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.preview_scroll)
                .on_scroll_wheel(move |_, _, cx| minimap.update(cx, |_, cx| cx.notify()))
                .px_6()
                .py_4()
                .pr(px(124.))
                .child(
                    TextView::markdown(
                        ("document-preview", self.source.entity_id()),
                        markdown.clone(),
                    )
                    .image_source(move |uri| {
                        base.as_ref()
                            .and_then(|base| base.join(uri.as_ref()).ok())
                            .and_then(|url| url.to_file_path().ok())
                            .map(ImageSource::from)
                            .unwrap_or_else(|| uri.clone().into())
                    })
                    .scrollable(false)
                    .w_full(),
                )
                .into_any_element()
        } else {
            Editor::new(&self.source)
                .readonly(true)
                .appearance(false)
                .bordered(false)
                .size_full()
                .pr(px(110.))
                .into_any_element()
        };
        let mut body = div()
            .relative()
            .size_full()
            .overflow_hidden()
            .child(content)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w(px(110.))
                    .child(self.minimap.clone()),
            );
        if self.preview {
            body = body.child(Scrollbar::vertical(&self.preview_scroll));
        } else {
            // GPUI Kit keeps its vertical scrollbar inside the text padding.
            // Cover that rail; the minimap owns the rail at the far outer edge.
            body = body.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom(px(14.))
                    .right(px(110.))
                    .w(px(12.))
                    .bg(cx.theme().background),
            );
        }
        body.into_any_element()
    }
}
