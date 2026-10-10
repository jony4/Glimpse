mod diff;
mod markdown;
mod media;
mod source;
use diff::DiffReader;
use glimpse_core::{DiffDocument, Document, DocumentKind, language_for_path};
use gpui_kit::*;
use markdown::MarkdownReader;
use media::{ImageReader, UnavailableReader};
use source::SourceReader;
use std::{path::PathBuf, sync::Arc};

pub type OpenLink = Arc<dyn Fn(PathBuf, Option<String>, &mut Window, &mut App) + Send + Sync>;

/// Tab identity is separate from the concrete renderer and its persistent state.
/// No image/error tab allocates an unused editor. JSON shares the code editor's
/// syntax-aware folding; Markdown owns preview state as well as a source editor.
enum Renderer {
    Source(SourceReader),
    Bytes(SourceReader),
    Markdown(MarkdownReader),
    Diff(DiffReader),
    Image(ImageReader),
    Unavailable(UnavailableReader),
}
pub struct Reader {
    pub repository_root: Option<PathBuf>,
    pub snapshot: String,
    pub title: SharedString,
    pub path: PathBuf,
    pub diff: Option<glimpse_core::GitChange>,
    renderer: Renderer,
}
impl Reader {
    pub fn new(document: Document, window: &mut Window, cx: &mut App) -> Self {
        let renderer = if document.kind == DocumentKind::Markdown {
            Renderer::Markdown(MarkdownReader::new(
                &document.path,
                &document.text,
                window,
                cx,
            ))
        } else {
            Renderer::Source(SourceReader::new(
                &document.text,
                language_for_path(&document.path),
                false,
                Vec::new(),
                ScrollHandle::new(),
                window,
                cx,
            ))
        };
        Self {
            title: document.path.display().to_string().into(),
            path: document.path,
            snapshot: document.text,
            repository_root: None,
            diff: None,
            renderer,
        }
    }
    pub fn from_bytes(
        document: glimpse_services::binary::BytePreview,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let source = SourceReader::new(
            &document.text,
            "plain",
            true,
            Vec::new(),
            ScrollHandle::new(),
            window,
            cx,
        );
        Self {
            title: document.path.display().to_string().into(),
            path: document.path,
            snapshot: document.text,
            repository_root: None,
            diff: None,
            renderer: Renderer::Bytes(source),
        }
    }
    pub fn from_diff(
        document: DiffDocument,
        root: &std::path::Path,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let renderer = Renderer::Diff(DiffReader::new(&document, window, cx));
        Self {
            title: format!(
                "{} · {}",
                document.change.path.display(),
                document.change.scope.label()
            )
            .into(),
            path: root.join(&document.change.path),
            snapshot: document.patch,
            repository_root: Some(root.to_path_buf()),
            diff: Some(document.change),
            renderer,
        }
    }
    pub fn from_image(
        document: glimpse_services::media::ImageDocument,
        _: &mut Window,
        _: &mut App,
    ) -> Self {
        Self {
            title: document.path.display().to_string().into(),
            path: document.path,
            snapshot: document.fingerprint,
            repository_root: None,
            diff: None,
            renderer: Renderer::Image(ImageReader(Arc::new(Image::from_bytes(
                ImageFormat::Png,
                document.png,
            )))),
        }
    }
    pub fn unavailable(path: PathBuf, reason: String, _: &mut Window, _: &mut App) -> Self {
        let renderer = Renderer::Unavailable(UnavailableReader::new(&path, reason));
        Self {
            title: path.display().to_string().into(),
            path,
            snapshot: String::new(),
            repository_root: None,
            diff: None,
            renderer,
        }
    }
    pub fn is_unavailable(&self) -> bool {
        matches!(self.renderer, Renderer::Unavailable(_))
    }
    fn source(&self) -> Option<&SourceReader> {
        match &self.renderer {
            Renderer::Source(s) | Renderer::Bytes(s) => Some(s),
            Renderer::Markdown(m) => Some(&m.source),
            Renderer::Diff(d) => Some(&d.source),
            _ => None,
        }
    }
    pub fn editor(&self) -> Option<Entity<gpui_kit::component::input::EditorState>> {
        if self.diff.is_some() {
            return None;
        }
        match &self.renderer {
            Renderer::Source(s) => Some(s.state.clone()),
            Renderer::Markdown(m) => Some(m.source.state.clone()),
            _ => None,
        }
    }
    pub fn is_dirty(&self) -> bool {
        match &self.renderer {
            Renderer::Source(s) => s.is_dirty(),
            Renderer::Markdown(m) => m.source.is_dirty(),
            _ => false,
        }
    }
    pub fn mark_saved(&mut self, text: String, cx: &App) {
        self.snapshot = text.clone();
        match &mut self.renderer {
            Renderer::Source(s) => s.mark_saved(text, cx),
            Renderer::Markdown(m) => m.source.mark_saved(text, cx),
            _ => {}
        }
    }
    pub fn inherit_view(&mut self, old: &Self, cx: &mut App) {
        match (&mut self.renderer, &old.renderer) {
            (Renderer::Source(s), Renderer::Source(o))
            | (Renderer::Bytes(s), Renderer::Bytes(o)) => s.inherit(o, cx),
            (Renderer::Markdown(s), Renderer::Markdown(o)) => {
                s.source.inherit(&o.source, cx);
                s.preview = o.preview;
                s.scroll.set_offset(o.scroll.offset());
            }
            (Renderer::Diff(s), Renderer::Diff(o)) => {
                s.source.inherit(&o.source, cx);
                s.side_by_side = o.side_by_side && s.split.is_some();
                if let (Some(new), Some(old)) = (&s.split, &o.split) {
                    let offset = old.read(cx).offset(cx);
                    new.update(cx, |v, cx| v.restore_offset(offset, cx));
                }
            }
            _ => {}
        }
    }
    pub fn focus_source(&self, window: &mut Window, cx: &mut App) {
        if let Some(source) = self.source() {
            source.state.update(cx, |s, cx| s.focus(window, cx));
        }
    }
    pub fn can_preview(&self) -> bool {
        matches!(self.renderer, Renderer::Markdown(_))
    }
    pub fn set_preview(&mut self, preview: bool, cx: &mut App) {
        if let Renderer::Markdown(m) = &mut self.renderer {
            if preview {
                m.sync_preview(cx);
            }
            m.preview = preview;
        }
    }
    pub fn toggle_label(&self) -> &'static str {
        if matches!(&self.renderer, Renderer::Markdown(m) if m.preview) {
            "Source"
        } else {
            "Preview"
        }
    }
    pub fn split_available(&self) -> bool {
        matches!(&self.renderer, Renderer::Diff(d) if d.split.is_some())
    }
    pub fn side_by_side(&self) -> bool {
        matches!(&self.renderer, Renderer::Diff(d) if d.side_by_side)
    }
    pub fn set_side_by_side(&mut self, enabled: bool) {
        if let Renderer::Diff(d) = &mut self.renderer {
            d.side_by_side = enabled && d.split.is_some();
        }
    }
    pub fn reveal_anchor(&mut self, anchor: &str, cx: &mut App) {
        if let Renderer::Markdown(m) = &mut self.renderer {
            m.sync_preview(cx);
            m.reveal(anchor);
        } else if let Some(source) = self.source()
            && let Some(line) = anchor
                .strip_prefix('L')
                .and_then(|s| s.split('-').next())
                .and_then(|s| s.parse::<usize>().ok())
        {
            source.state.update(cx, |s, cx| {
                let height = s.line_height().unwrap_or(px(20.));
                s.set_scroll_offset(point(px(0.), -height * line.saturating_sub(1) as f32), cx);
            });
        }
    }
    pub fn render(&mut self, open: OpenLink, cx: &mut App) -> AnyElement {
        match &mut self.renderer {
            Renderer::Source(s) | Renderer::Bytes(s) => s.render(cx),
            Renderer::Markdown(m) => m.render(open, cx),
            Renderer::Diff(d) => d.render(cx),
            Renderer::Image(i) => i.render(),
            Renderer::Unavailable(e) => e.render(cx),
        }
    }
}

#[cfg(test)]
mod editing_tests {
    use super::{Reader, Renderer};
    use glimpse_core::{DiffDocument, DiffScope, Document, DocumentKind, GitChange};
    use gpui_kit::{
        App, AppContext, Bounds, Context, Entity, IntoElement, Point, Render, TestAppContext,
        Window, WindowBounds, WindowOptions, px, size, test::TestWindowExt,
    };

    struct Fixture(Reader);
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.0.render(std::sync::Arc::new(|_, _, _, _| {}), cx)
        }
    }
    fn open(
        cx: &mut TestAppContext,
        build: impl FnOnce(&mut Window, &mut App) -> Reader,
    ) -> (gpui_kit::AnyWindowHandle, Entity<Fixture>) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(900.), px(600.)),
                    })),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| Fixture(build(window, cx))),
            )
            .unwrap()
        })
    }
    #[gpui_kit::test]
    fn markdown_preview_uses_unsaved_edits(cx: &mut TestAppContext) {
        let (handle, view) = open(cx, |window, cx| {
            let mut reader = Reader::new(
                Document {
                    path: "/tmp/test-note.md".into(),
                    kind: DocumentKind::Markdown,
                    text: "# Original\n".into(),
                },
                window,
                cx,
            );
            reader.set_preview(false, cx);
            reader
        });
        let editor = cx.update(|cx| view.read(cx).0.editor().unwrap());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(("input", editor.entity_id()), cx);
            window.press("cmd-a", cx);
            window.input("# Edited heading\n", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| view.read(cx).0.is_dirty()));
        cx.update(|cx| view.update(cx, |v, cx| v.0.set_preview(true, cx)));
        cx.run_until_parked();
        cx.update(|cx| {
            let Renderer::Markdown(markdown) = &view.read(cx).0.renderer else {
                unreachable!()
            };
            assert_eq!(
                markdown.source.state.read(cx).value().as_ref(),
                "# Edited heading\n"
            );
            assert!(
                markdown
                    .state
                    .read(cx)
                    .rendered_text()
                    .as_str()
                    .contains("Edited heading")
            );
            assert_eq!(view.read(cx).0.snapshot, "# Original\n");
        });
    }
    #[gpui_kit::test]
    fn byte_preview_rejects_typing_and_has_no_save_buffer(cx: &mut TestAppContext) {
        let (handle, view) = open(cx, |window, cx| {
            Reader::from_bytes(
                glimpse_services::binary::BytePreview {
                    path: "/tmp/.DS_Store".into(),
                    text: "00000000  00 ff 41".into(),
                },
                window,
                cx,
            )
        });
        let editor = cx.update(|cx| view.read(cx).0.source().unwrap().state.clone());
        let original = cx.update(|cx| editor.read(cx).value());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(("input", editor.entity_id()), cx);
            window.press("cmd-a", cx);
            window.input("must not overwrite binary bytes", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(editor.read(cx).value(), original);
            assert!(view.read(cx).0.editor().is_none());
            assert!(!view.read(cx).0.is_dirty());
        });
    }

    #[gpui_kit::test]
    fn diff_rejects_typing_and_has_no_save_buffer(cx: &mut TestAppContext) {
        let (handle, view) = open(cx, |window, cx| {
            let mut reader = Reader::from_diff(
                DiffDocument {
                    change: GitChange {
                        path: "text.txt".into(),
                        original_path: None,
                        scope: DiffScope::Worktree,
                        status: 'M',
                    },
                    patch: "@@ -1 +1 @@\n-old\n+new\n".into(),
                },
                std::path::Path::new("/tmp"),
                window,
                cx,
            );
            reader.set_side_by_side(false);
            reader
        });
        let editor = cx.update(|cx| view.read(cx).0.source().unwrap().state.clone());
        let original = cx.update(|cx| editor.read(cx).value());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(("input", editor.entity_id()), cx);
            window.press("cmd-a", cx);
            window.input("must not replace the diff", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(editor.read(cx).value(), original);
            assert!(view.read(cx).0.editor().is_none());
            assert!(!view.read(cx).0.is_dirty());
        });
    }
}
