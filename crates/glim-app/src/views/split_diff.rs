use glim_core::split_diff::SplitPatch;
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
    scroll_sync: ScrollSync,
}
/// Only a pane receiving user input may drive its peer. Editor scroll writes
/// are deferred until layout, so notifications from the follower are never input.
#[derive(Default)]
struct ScrollSync {
    leader: Option<usize>,
    observed: [f32; 2],
}
impl ScrollSync {
    fn activate(&mut self, pane: usize) {
        self.leader = Some(pane);
    }
    fn changed(&mut self, pane: usize, y: f32) -> bool {
        let moved = (y - self.observed[pane]).abs() > 0.5;
        self.observed[pane] = y;
        moved && self.leader == Some(pane)
    }
}

impl SplitDiff {
    pub fn horizontal_offsets(&self, cx: &App) -> Vec<Pixels> {
        vec![
            self.left.read(cx).scroll_offset().x,
            self.right.read(cx).scroll_offset().x,
        ]
    }

    pub fn set_word_wrap(&self, enabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        for editor in [&self.left, &self.right] {
            editor.update(cx, |s, cx| s.set_soft_wrap(enabled, window, cx));
        }
    }

    pub fn offset(&self, cx: &App) -> Point<Pixels> {
        self.left.read(cx).scroll_offset()
    }
    pub fn restore_offset(&mut self, offset: Point<Pixels>, cx: &mut Context<Self>) {
        self.scroll_sync = ScrollSync {
            leader: None,
            observed: [f32::from(offset.y); 2],
        };
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
                if view.scroll_sync.changed(index, f32::from(y))
                    && (to.read(cx).scroll_offset().y - y).abs() > px(0.5)
                {
                    to.update(cx, |state, cx| {
                        state.set_scroll_offset(point(state.scroll_offset().x, y), cx)
                    });
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
            scroll_sync: ScrollSync::default(),
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
        h_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .children(
                [("Before", self.left.clone()), ("After", self.right.clone())]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, state))| {
                        let view = cx.entity().downgrade();
                        let menu_copyable =
                            state.read(cx).context_menu_capabilities().is_copyable();
                        v_flex()
                            .id(("diff-pane", index))
                            .relative()
                            .min_h_0()
                            .overflow_hidden()
                            .capture_any_mouse_down(
                                cx.listener(move |v, _, _, _| v.scroll_sync.activate(index)),
                            )
                            .capture_key_down(
                                cx.listener(move |v, _, _, _| v.scroll_sync.activate(index)),
                            )
                            .child(
                                canvas(
                                    |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        let view = view.clone();
                                        // The editor stops wheel propagation. Capture before it handles the
                                        // gesture, including momentum, without consuming the event.
                                        window.on_mouse_event(
                                            move |event: &ScrollWheelEvent, phase, _, cx| {
                                                if phase == DispatchPhase::Capture
                                                    && bounds.contains(&event.position)
                                                {
                                                    let _ = view.update(cx, |v, _| {
                                                        v.scroll_sync.activate(index)
                                                    });
                                                }
                                            },
                                        );
                                    },
                                )
                                .absolute()
                                .size_full(),
                            )
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .border_r_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .h(px(26.))
                                    .flex_shrink_0()
                                    .px_3()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(label),
                            )
                            .child(
                                div().flex_1().min_h_0().w_full().child(
                                    Editor::new(&state)
                                        .context_menu(move |menu, _, _| {
                                            super::reader::editor_menu(menu, false, menu_copyable)
                                        })
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

#[cfg(test)]
mod tests {
    use super::ScrollSync;
    #[test]
    fn delayed_follower_notifications_cannot_pull_the_driver_back() {
        let mut sync = ScrollSync::default();
        sync.activate(0);
        assert!(sync.changed(0, -100.));
        assert!(sync.changed(0, -180.));
        assert!(!sync.changed(1, -100.)); // first queued layout completes late
        assert!(!sync.changed(1, -180.));
        assert!(!sync.changed(0, -180.));
        assert!(sync.changed(0, -250.));
        assert!(!sync.changed(1, -230.)); // follower hits its bottom boundary
        assert!(!sync.changed(1, -250.));
    }
    #[test]
    fn either_pane_can_drive_after_input_but_restoration_cannot() {
        let mut sync = ScrollSync::default();
        assert!(!sync.changed(0, -400.));
        assert!(!sync.changed(1, -400.));
        sync.activate(1);
        assert!(sync.changed(1, -500.));
        assert!(!sync.changed(0, -500.));
        sync.activate(0);
        assert!(sync.changed(0, -420.));
        assert!(!sync.changed(1, -420.));
    }
}
