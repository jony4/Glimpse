use super::super::minimap::Minimap;
use glim_core::DiffSpan;
use gpui_kit::{
    component::{
        ActiveTheme,
        input::{Editor, EditorState, InputEvent, TextDecoration, TextDecorationCollection},
    },
    *,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Shared basic editor surface for text/code; inline diffs remain read-only.
/// Native gutter, folding and scrollbars share the editor's layout frame.
pub(super) struct SourceReader {
    pub state: Entity<EditorState>,
    pub minimap: Entity<Minimap>,
    spans: Vec<DiffSpan>,
    marks: Option<TextDecorationCollection>,
    palette: Option<(Hsla, Hsla)>,
    readonly: bool,
    locked: bool,
    baseline: Rc<RefCell<String>>,
    dirty: Rc<Cell<bool>>,
    _edit_subscription: Subscription,
}
impl SourceReader {
    pub fn new(
        text: &str,
        language: &'static str,
        diff: bool,
        spans: Vec<DiffSpan>,
        scroll: ScrollHandle,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let state = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language(language)
                .auto_close(false)
                .smart_indent(false)
                .line_number(!diff)
                .folding(!diff)
                .soft_wrap(false)
                .scroll_beyond_last_line(Some(0));
            if super::lexical::supports(language) {
                state.set_highlighter_factory(
                    Rc::new(|name| Some(Box::new(super::lexical::Lexical::new(name)))),
                    cx,
                );
            }
            state.set_value(text.to_owned(), window, cx);
            state.set_readonly(diff, cx);
            state
        });
        let minimap = cx.new(|cx| Minimap::new(state.clone(), text, diff, scroll, cx));
        let marks =
            diff.then(|| state.update(cx, |s, cx| s.create_decorations_collection(Vec::new(), cx)));
        let baseline = Rc::new(RefCell::new(if diff {
            String::new()
        } else {
            text.to_owned()
        }));
        let dirty = Rc::new(Cell::new(false));
        let saved = baseline.clone();
        let changed = dirty.clone();
        let map = minimap.clone();
        let subscription = cx.subscribe(&state, move |state, event, cx| {
            if matches!(event, InputEvent::Change) {
                let text = state.read(cx).value().to_string();
                let dirty = text != *saved.borrow();
                let previous = changed.replace(dirty);
                map.update(cx, |map, cx| map.set_text(&text, diff, cx));
                if previous != dirty {
                    cx.refresh_windows();
                }
            }
        });
        Self {
            readonly: diff,
            locked: false,
            baseline,
            dirty,
            _edit_subscription: subscription,
            state,
            minimap,
            spans: compact_spans(text, spans),
            marks,
            palette: None,
        }
    }
    pub fn set_locked(&mut self, locked: bool, cx: &mut App) {
        self.locked = locked;
        self.state
            .update(cx, |s, cx| s.set_readonly(self.readonly || locked, cx));
    }
    pub fn is_dirty(&self) -> bool {
        !self.readonly && self.dirty.get()
    }
    pub fn mark_saved(&mut self, text: String, cx: &App) {
        self.dirty.set(self.state.read(cx).value().as_ref() != text);
        *self.baseline.borrow_mut() = text;
    }
    pub fn inherit(&mut self, old: &Self, cx: &mut App) {
        let offset = old.state.read(cx).scroll_offset();
        self.state
            .update(cx, |s, cx| s.set_scroll_offset(offset, cx));
    }
    pub fn render(&mut self, cx: &mut App) -> AnyElement {
        let menu_state = self.state.clone();
        self.minimap
            .update(cx, |map, cx| map.set_preview(false, cx));
        let palette = (cx.theme().green, cx.theme().red);
        if self.palette != Some(palette)
            && let Some(marks) = &self.marks
        {
            marks.set(
                self.spans
                    .iter()
                    .map(|s| {
                        let color = if s.addition { palette.0 } else { palette.1 };
                        TextDecoration::new(
                            s.range.clone(),
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
        gpui_kit::component::h_flex()
            .size_full()
            .font_family("Menlo")
            .text_size(px(14.))
            .line_height(px(22.))
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_hidden()
                    .child(
                        Editor::new(&self.state)
                            .context_menu(move |menu, _, cx| {
                                super::editor_menu(menu, &menu_state, cx)
                            })
                            .readonly(self.readonly || self.locked)
                            .appearance(false)
                            .bordered(false)
                            .size_full(),
                    )
                    // Hide the duplicate vertical rail, not the horizontal scroll track.
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .right_0()
                            .w(px(12.))
                            .bg(cx.theme().background),
                    ),
            )
            .child(
                div()
                    .w(px(110.))
                    .flex_shrink_0()
                    .h_full()
                    .child(self.minimap.clone()),
            )
            .into_any_element()
    }
}

// The toolkit scans the decoration collection for each visible range. A block
// of thousands of adjacent changes needs one range, not one per source line.
fn compact_spans(text: &str, spans: Vec<DiffSpan>) -> Vec<DiffSpan> {
    let mut compact: Vec<DiffSpan> = Vec::new();
    for span in spans {
        if let Some(previous) = compact.last_mut()
            && previous.addition == span.addition
            && text.get(previous.range.end..span.range.start) == Some("\n")
        {
            previous.range.end = span.range.end;
        } else {
            compact.push(span);
        }
    }
    compact
}
#[cfg(test)]
mod tests {
    use super::compact_spans;
    #[test]
    fn compaction_keeps_colors_and_hunk_boundaries() {
        let (text, spans) = glim_core::diff_content(
            "@@ -1,2 +1,2 @@\n-旧一\n-旧二\n+新一\n+新二\n@@ -10 +10 @@\n+另一区块\n",
        );
        let compact = compact_spans(&text, spans);
        assert_eq!(
            compact
                .iter()
                .map(|s| (&text[s.range.clone()], s.addition))
                .collect::<Vec<_>>(),
            [
                ("-旧一\n-旧二", false),
                ("+新一\n+新二", true),
                ("+另一区块", true)
            ]
        );
    }
}

#[cfg(test)]
mod format_registry_tests {
    #[test]
    fn new_source_formats_have_compiled_grammars() {
        crate::app::languages::init();
        let registry = gpui_kit::component::highlighter::LanguageRegistry::singleton();
        for name in [
            "java", "sql", "make", "html", "c", "cpp", "python", "json", "ruby", "php", "csharp",
            "swift", "kotlin", "lua", "scala", "elixir", "zig", "graphql", "proto", "cmake",
            "astro", "svelte", "erb", "ejs", "jsdoc",
        ] {
            let grammar = registry
                .language(name)
                .expect("enabled grammar must be registered");
            assert!(grammar.language.is_some(), "{name}");
            assert!(!grammar.highlights.is_empty(), "{name}");
            let highlighter = gpui_kit::component::highlighter::SyntaxHighlighter::new(name);
            assert_eq!(
                highlighter.language().as_ref(),
                name,
                "invalid query for {name}"
            );
        }
    }
}
