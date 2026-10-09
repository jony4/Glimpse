use glimpse_core::split_diff::SplitPatch;
use gpui_kit::{
    component::{
        ActiveTheme, h_flex,
        input::{Editor, EditorState, TextDecoration, TextDecorationCollection},
        v_flex,
    },
    *,
};

pub struct SplitDiff {
    left: Entity<EditorState>,
    right: Entity<EditorState>,
    _subscriptions: Vec<Subscription>,
    left_marks: TextDecorationCollection,
    right_marks: TextDecorationCollection,
    removed: Vec<std::ops::Range<usize>>,
    added: Vec<std::ops::Range<usize>>,
    palette: Option<(Hsla, Hsla)>,
    last_y: [Pixels; 2],
}
impl SplitDiff {
    pub fn offset(&self, cx: &App) -> Point<Pixels> {
        self.left.read(cx).scroll_offset()
    }
    pub fn restore_offset(&mut self, offset: Point<Pixels>, cx: &mut Context<Self>) {
        for state in [&self.left, &self.right] {
            state.update(cx, |s, cx| s.set_scroll_offset(offset, cx));
        }
    }

    pub fn new(patch: SplitPatch, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut make = |text| {
            cx.new(|cx| {
                let mut state = EditorState::new(window, cx)
                    .language("plain")
                    .line_number(false)
                    .folding(false)
                    .soft_wrap(false)
                    .scroll_beyond_last_line(Some(0));
                state.set_value(text, window, cx);
                state.set_readonly(true, cx);
                state
            })
        };
        let left = make(patch.before);
        let right = make(patch.after);
        let mut subscriptions = Vec::new();
        for (index, (from, to)) in [(left.clone(), right.clone()), (right.clone(), left.clone())]
            .into_iter()
            .enumerate()
        {
            subscriptions.push(cx.observe(&from, move |view, from, cx| {
                let y = from.read(cx).scroll_offset().y;
                if (y - view.last_y[index]).abs() > px(0.5) {
                    view.last_y[index] = y;
                    if (to.read(cx).scroll_offset().y - y).abs() > px(0.5) {
                        to.update(cx, |state, cx| {
                            state.set_scroll_offset(point(state.scroll_offset().x, y), cx)
                        });
                    }
                }
            }));
        }
        let left_marks = left.update(cx, |s, cx| s.create_decorations_collection(Vec::new(), cx));
        let right_marks = right.update(cx, |s, cx| s.create_decorations_collection(Vec::new(), cx));
        Self {
            left,
            right,
            _subscriptions: subscriptions,
            left_marks,
            right_marks,
            removed: patch.removed,
            added: patch.added,
            palette: None,
            last_y: [px(0.); 2],
        }
    }
}
impl Render for SplitDiff {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = (cx.theme().red, cx.theme().green);
        if self.palette != Some(palette) {
            for (marks, ranges, color) in [
                (&self.left_marks, &self.removed, palette.0),
                (&self.right_marks, &self.added, palette.1),
            ] {
                marks.set(
                    ranges
                        .iter()
                        .map(|r| {
                            TextDecoration::new(
                                r.clone(),
                                HighlightStyle {
                                    background_color: Some(color.opacity(0.12)),
                                    color: Some(color),
                                    ..Default::default()
                                },
                            )
                        })
                        .collect(),
                    cx,
                );
            }
            self.palette = Some(palette);
        }
        h_flex().size_full().children(
            [("Before", self.left.clone()), ("After", self.right.clone())]
                .into_iter()
                .map(|(label, state)| {
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .border_r_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .h(px(26.))
                                .px_3()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(label),
                        )
                        .child(
                            div().flex_1().min_h_0().w_full().child(
                                Editor::new(&state)
                                    .readonly(true)
                                    .appearance(false)
                                    .bordered(false)
                                    .size_full(),
                            ),
                        )
                }),
        )
    }
}
