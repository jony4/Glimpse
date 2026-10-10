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
            Renderer::Source(s) => Some(s),
            Renderer::Markdown(m) => Some(&m.source),
            Renderer::Diff(d) => Some(&d.source),
            _ => None,
        }
    }
    pub fn inherit_view(&mut self, old: &Self, cx: &mut App) {
        match (&mut self.renderer, &old.renderer) {
            (Renderer::Source(s), Renderer::Source(o)) => s.inherit(o, cx),
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
    pub fn set_preview(&mut self, preview: bool) {
        if let Renderer::Markdown(m) = &mut self.renderer {
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
            Renderer::Source(s) => s.render(cx),
            Renderer::Markdown(m) => m.render(open, cx),
            Renderer::Diff(d) => d.render(cx),
            Renderer::Image(i) => i.render(),
            Renderer::Unavailable(e) => e.render(cx),
        }
    }
}
