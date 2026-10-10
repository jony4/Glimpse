mod diff;
mod json;
mod lexical;
mod markup;
mod media;
mod native;
mod paged;
mod source;
use diff::DiffReader;
use glim_core::{DiffDocument, Document, DocumentKind, language_for_path};
use gpui_kit::*;
use markup::MarkupReader;
use media::{ImageReader, UnavailableReader};
use source::SourceReader;
use std::{path::PathBuf, sync::Arc};

/// Keep native text editing actions without the toolkit's unused LSP entries.
pub(super) fn editor_menu(
    menu: gpui_kit::component::native_menu::NativeMenu,
    editable: bool,
    copyable: bool,
) -> gpui_kit::component::native_menu::NativeMenu {
    use gpui_kit::component::input::{Copy, Cut, Paste, SelectAll};
    menu.menu_with_disabled("Cut", !(editable && copyable), Box::new(Cut))
        .menu_with_disabled("Copy", !copyable, Box::new(Copy))
        .menu_with_disabled("Paste", !editable, Box::new(Paste))
        .separator()
        .menu("Select All", Box::new(SelectAll))
}

pub type OpenLink = Arc<dyn Fn(PathBuf, Option<String>, &mut Window, &mut App) + Send + Sync>;

/// Tab identity is separate from the concrete renderer and its persistent state.
/// No image/error tab allocates an unused editor. JSON shares the code editor's
/// syntax-aware folding; Markdown owns preview state as well as a source editor.
enum Renderer {
    Source(SourceReader),
    Paged(Entity<paged::PagedReader>),
    Bytes(SourceReader),
    Markup(MarkupReader),
    Diff(DiffReader),
    Image(ImageReader),
    Native(Entity<native::NativeReader>),
    Unavailable(UnavailableReader),
}
pub struct Reader {
    pub repository_root: Option<PathBuf>,
    pub snapshot: String,
    pub title: SharedString,
    pub path: PathBuf,
    pub diff: Option<glim_core::GitChange>,
    pub historical: bool,
    pub commit_id: Option<String>,
    pub commit_file: Option<PathBuf>,
    json: Option<(Entity<json::JsonTree>, bool)>,
    renderer: Renderer,
}
impl Reader {
    pub fn new(document: Document, window: &mut Window, cx: &mut App) -> Self {
        let html = document
            .path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("html") || s.eq_ignore_ascii_case("htm"));
        let renderer = if document.kind == DocumentKind::Markdown || html {
            Renderer::Markup(MarkupReader::new(
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
        let json = (language_for_path(&document.path) == "json")
            .then(|| (cx.new(|_| json::JsonTree::new()), false));
        Self {
            title: document.path.display().to_string().into(),
            path: document.path,
            snapshot: document.text,
            repository_root: None,
            diff: None,
            historical: false,
            commit_id: None,
            commit_file: None,
            json,
            renderer,
        }
    }
    pub fn from_native(path: PathBuf, folder: bool, shuffle: bool, cx: &mut App) -> Self {
        Self {
            repository_root: None,
            snapshot: "native-preview".into(),
            title: path.display().to_string().into(),
            path: path.clone(),
            diff: None,
            historical: false,
            commit_id: None,
            commit_file: None,
            json: None,
            renderer: Renderer::Native(
                cx.new(|cx| native::NativeReader::new(path, folder, shuffle, cx)),
            ),
        }
    }

    pub fn from_page(page: glim_services::paged::TextPage, cx: &mut App) -> Self {
        Self {
            title: page.path.display().to_string().into(),
            path: page.path.clone(),
            snapshot: page.fingerprint(),
            repository_root: None,
            diff: None,
            historical: false,
            commit_id: None,
            commit_file: None,
            json: None,
            renderer: Renderer::Paged(cx.new(|_| paged::PagedReader::new(page))),
        }
    }
    pub fn from_commit(
        path: PathBuf,
        text: String,
        commit_file: Option<PathBuf>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let (content, spans) = glim_core::commit_content(&text);
        let source = SourceReader::new(
            &content,
            "plain",
            true,
            spans,
            ScrollHandle::new(),
            window,
            cx,
        );
        let split = glim_core::split_diff::split_commit_patch(&text)
            .map(|patch| cx.new(|cx| crate::views::split_diff::SplitDiff::new(patch, window, cx)));
        let side_by_side = commit_file.is_some() && split.is_some();
        let root = path.parent().map(ToOwned::to_owned);
        let commit_id = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("Commit "))
            .map(str::to_owned);
        Self {
            title: commit_file
                .as_ref()
                .map_or_else(
                    || path.display().to_string(),
                    |file| {
                        format!(
                            "{} · {}",
                            file.display(),
                            path.file_name().unwrap_or_default().to_string_lossy()
                        )
                    },
                )
                .into(),
            path,
            snapshot: text,
            repository_root: root,
            diff: None,
            historical: true,
            commit_id,
            commit_file,
            json: None,
            renderer: Renderer::Diff(DiffReader {
                source,
                split,
                side_by_side,
            }),
        }
    }
    pub fn from_bytes(
        document: glim_services::binary::BytePreview,
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
            historical: false,
            commit_id: None,
            commit_file: None,
            json: None,
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
            historical: false,
            commit_id: None,
            commit_file: None,
            json: None,
            renderer,
        }
    }
    pub fn from_image(
        document: glim_services::media::ImageDocument,
        _: &mut Window,
        _: &mut App,
    ) -> Self {
        Self {
            title: document.path.display().to_string().into(),
            path: document.path,
            snapshot: document.fingerprint,
            repository_root: None,
            diff: None,
            historical: false,
            commit_id: None,
            commit_file: None,
            json: None,
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
            historical: false,
            commit_id: None,
            commit_file: None,
            json: None,
            renderer,
        }
    }
    pub fn is_unavailable(&self) -> bool {
        matches!(self.renderer, Renderer::Unavailable(_))
    }
    fn source(&self) -> Option<&SourceReader> {
        match &self.renderer {
            Renderer::Source(s) | Renderer::Bytes(s) => Some(s),
            Renderer::Markup(m) => Some(&m.source),
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
            Renderer::Markup(m) => Some(m.source.state.clone()),
            _ => None,
        }
    }
    pub fn set_editing_locked(&mut self, locked: bool, cx: &mut App) {
        match &mut self.renderer {
            Renderer::Source(s) => s.set_locked(locked, cx),
            Renderer::Markup(m) => m.source.set_locked(locked, cx),
            _ => {}
        }
    }
    pub fn is_dirty(&self) -> bool {
        match &self.renderer {
            Renderer::Source(s) => s.is_dirty(),
            Renderer::Markup(m) => m.source.is_dirty(),
            _ => false,
        }
    }
    pub fn mark_saved(&mut self, text: String, cx: &App) {
        self.snapshot = text.clone();
        match &mut self.renderer {
            Renderer::Source(s) => s.mark_saved(text, cx),
            Renderer::Markup(m) => m.source.mark_saved(text, cx),
            _ => {}
        }
    }
    pub fn inherit_view(&mut self, old: &Self, cx: &mut App) {
        if old.json.as_ref().is_some_and(|(_, visible)| *visible) {
            self.set_preview(true, cx);
        }
        match (&mut self.renderer, &old.renderer) {
            (Renderer::Source(s), Renderer::Source(o))
            | (Renderer::Bytes(s), Renderer::Bytes(o)) => s.inherit(o, cx),
            (Renderer::Markup(s), Renderer::Markup(o)) => {
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
    pub fn set_word_wrap(&self, enabled: bool, window: &mut Window, cx: &mut App) {
        if let Some(source) = self.source() {
            source
                .state
                .update(cx, |s, cx| s.set_soft_wrap(enabled, window, cx));
        }
        if let Renderer::Diff(diff) = &self.renderer
            && let Some(split) = &diff.split
        {
            split.update(cx, |s, cx| s.set_word_wrap(enabled, window, cx));
        }
    }

    pub fn horizontal_offsets(&self, cx: &App) -> Option<Vec<Pixels>> {
        if self.json.as_ref().is_some_and(|(_, visible)| *visible) {
            return Some(Vec::new());
        }
        match &self.renderer {
            Renderer::Paged(_) => None,
            Renderer::Markup(m) if m.preview => Some(vec![m.scroll.offset().x]),
            Renderer::Diff(d) if d.side_by_side => {
                d.split.as_ref().map(|s| s.read(cx).horizontal_offsets(cx))
            }
            _ => Some(
                self.source()
                    .map(|s| vec![s.state.read(cx).scroll_offset().x])
                    .unwrap_or_default(),
            ),
        }
    }

    pub fn focus_source(&self, window: &mut Window, cx: &mut App) {
        if let Some(source) = self.source() {
            source.state.update(cx, |s, cx| s.focus(window, cx));
        }
    }
    pub fn history_summary(&self) -> Option<(String, String)> {
        if !self.historical {
            return None;
        }
        let header = self.snapshot.split("\ndiff --git ").next().unwrap_or("");
        let subject = header
            .lines()
            .find_map(|line| {
                line.strip_prefix("    ")
                    .filter(|line| !line.trim().is_empty())
            })
            .unwrap_or("Commit changes");
        let author = header
            .lines()
            .find_map(|line| line.strip_prefix("Author:"))
            .map(|author| author.trim().split(" <").next().unwrap_or(author))
            .unwrap_or("");
        let id = self.commit_id.as_deref().unwrap_or("");
        Some((
            subject.to_owned(),
            format!("{} · {}", author, &id[..8.min(id.len())]),
        ))
    }

    pub fn is_json(&self) -> bool {
        self.json.is_some()
    }
    pub fn preview_preference(&self) -> Option<&'static str> {
        match &self.renderer {
            Renderer::Markup(m) => Some(if m.html { "html" } else { "markdown" }),
            _ => None,
        }
    }
    pub fn can_preview(&self) -> bool {
        self.json.is_some() || matches!(self.renderer, Renderer::Markup(_))
    }
    pub fn set_preview(&mut self, preview: bool, cx: &mut App) {
        if self.json.is_some() {
            let text = if preview {
                self.source().map(|s| s.state.read(cx).value().to_string())
            } else {
                None
            };
            if let Some((tree, visible)) = &mut self.json {
                *visible = preview;
                if let Some(text) = text {
                    tree.update(cx, |v, cx| v.load(text, cx));
                }
            }
        }
        if let Renderer::Markup(m) = &mut self.renderer {
            if preview {
                m.sync_preview(cx);
            }
            m.preview = preview;
        }
    }
    pub fn toggle_label(&self) -> &'static str {
        if self.json.as_ref().is_some_and(|(_, visible)| *visible)
            || matches!(&self.renderer, Renderer::Markup(m) if m.preview)
        {
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
        if let Renderer::Markup(m) = &mut self.renderer {
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
        if let Some((tree, true)) = &self.json {
            return tree.clone().into_any_element();
        }
        match &mut self.renderer {
            Renderer::Source(s) | Renderer::Bytes(s) => s.render(cx),
            Renderer::Markup(m) => m.render(open, cx),
            Renderer::Diff(d) => d.render(cx),
            Renderer::Image(i) => i.render(),
            Renderer::Native(n) => n.clone().into_any_element(),
            Renderer::Paged(p) => p.clone().into_any_element(),
            Renderer::Unavailable(e) => e.render(cx),
        }
    }
}

#[cfg(test)]
mod editing_tests {
    use super::{Reader, Renderer};
    use glim_core::{DiffDocument, DiffScope, Document, DocumentKind, GitChange};
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
    fn source_right_click_does_not_reborrow_editor(cx: &mut TestAppContext) {
        for (name, kind) in [
            ("test.rs", DocumentKind::Text),
            ("test.md", DocumentKind::Markdown),
            ("test.html", DocumentKind::Text),
        ] {
            let (handle, view) = open(cx, |window, cx| {
                let mut reader = Reader::new(
                    Document {
                        path: std::path::Path::new("/tmp").join(name),
                        kind,
                        text: "Sample text\n".into(),
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
                window.right_click(("input", editor.entity_id()), cx);
            })
            .unwrap();
            cx.run_until_parked();
        }
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
            let Renderer::Markup(markdown) = &view.read(cx).0.renderer else {
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
                glim_services::binary::BytePreview {
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
            window.right_click(("input", editor.entity_id()), cx);
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
