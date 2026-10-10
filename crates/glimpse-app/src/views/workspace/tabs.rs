use super::Workspace;
use crate::views::reader::Reader;
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, Icon, Selectable, Sizable,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        h_flex,
    },
    prelude::FluentBuilder,
    *,
};

impl Workspace {
    pub(super) fn active_reader(&self) -> Option<&Reader> {
        self.active.and_then(|index| self.tabs.get(index))
    }

    pub(super) fn activate_tab(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.get(index).is_some() {
            self.load_task = None;
            self.loading = false;
            self.active = Some(index);
            window.focus(&self.focus, cx);
            self.tab_scroll.scroll_to_item(index);
            window.set_window_title("Glimpse");
            self.record_history();
            cx.notify();
        }
    }

    pub(super) fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.load_task = None;
        self.loading = false;
        self.active = selection_after_close(self.active, index, self.tabs.len());
        self.tabs.remove(index);
        if let Some(active) = self.active {
            self.activate_tab(active, window, cx);
        } else {
            window.set_window_title("Glimpse");
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }
}

impl Workspace {
    pub(super) fn tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let tabs =
            self.tabs
                .iter()
                .enumerate()
                .map(|(index, reader)| {
                    let active = self.active == Some(index);
                    let mut label = reader
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    if let Some(diff) = &reader.diff {
                        label.push_str(&format!(" · {}", diff.scope.label()));
                    }
                    h_flex()
                        .id(("document-tab", index))
                        .h_full()
                        .flex_shrink_0()
                        .gap_2()
                        .px_3()
                        .border_r_1()
                        .border_color(cx.theme().border)
                        .relative()
                        .child(div().absolute().top_0().left_0().right_0().h(px(2.)).bg(
                            if active {
                                cx.theme().primary
                            } else {
                                cx.theme().border
                            },
                        ))
                        .bg(if active {
                            cx.theme().background
                        } else {
                            cx.theme().sidebar
                        })
                        .cursor_pointer()
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.activate_tab(index, window, cx)
                        }))
                        .child(
                            Button::new(("activate-tab", index))
                                .custom(ButtonCustomVariant::new(cx))
                                .accessibility_label(format!("Show {}", reader.title))
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(crate::views::file_icons::file_icon(&reader.path))
                                        .child(
                                            div().max_w(px(210.)).truncate().text_sm().child(label),
                                        ),
                                )
                                .on_click(cx.listener(move |view, _, window, cx| {
                                    cx.stop_propagation();
                                    view.activate_tab(index, window, cx);
                                })),
                        )
                        .child(
                            Button::new(("close-tab", index))
                                .ghost()
                                .label("×")
                                .accessibility_label(format!("Close {}", reader.title))
                                .on_click(cx.listener(move |view, _, window, cx| {
                                    cx.stop_propagation();
                                    view.close_tab(index, window, cx);
                                })),
                        )
                })
                .collect::<Vec<_>>();
        h_flex()
            .h(px(40.))
            .w_full()
            .flex_shrink_0()
            .bg(cx.theme().sidebar)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .id("document-tabs")
                    .track_scroll(&self.tab_scroll)
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_x_scroll()
                    .children(tabs),
            )
            .when(
                self.active_reader().is_some_and(|r| r.diff.is_some()),
                |bar| {
                    let split = self.active_reader().is_some_and(|r| r.side_by_side());
                    let available = self.active_reader().is_some_and(|r| r.split_available());
                    bar.child(
                        Button::new("diff-split")
                            .ghost()
                            .icon(Icon::new(gpui_kit::assets::IconName::Columns2))
                            .xsmall()
                            .tooltip("Side by side")
                            .accessibility_label("Side by side")
                            .selected(split)
                            .disabled(!available)
                            .on_click(cx.listener(|v, _, _, cx| {
                                if let Some(i) = v.active {
                                    v.tabs[i].set_side_by_side(true);
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("diff-inline")
                            .ghost()
                            .icon(Icon::new(gpui_kit::assets::IconName::Rows2))
                            .xsmall()
                            .tooltip("Inline")
                            .accessibility_label("Inline")
                            .selected(!split)
                            .on_click(cx.listener(|v, _, _, cx| {
                                if let Some(i) = v.active {
                                    v.tabs[i].set_side_by_side(false);
                                }
                                cx.notify();
                            })),
                    )
                },
            )
            .when(
                self.active_reader().is_some_and(|r| r.can_preview()),
                |bar| {
                    let preview = self
                        .active_reader()
                        .is_some_and(|r| r.toggle_label() == "Source");
                    bar.child(
                        h_flex()
                            .px_2()
                            .gap_1()
                            .child(
                                Button::new("markdown-rendered")
                                    .ghost()
                                    .icon(Icon::new(gpui_kit::assets::IconName::Eye))
                                    .xsmall()
                                    .accessibility_label("Preview")
                                    .tooltip("Preview")
                                    .selected(preview)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        if let Some(i) = view.active {
                                            view.tabs[i].set_preview(true);
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("markdown-source")
                                    .ghost()
                                    .icon(Icon::new(gpui_kit::assets::IconName::Code))
                                    .xsmall()
                                    .accessibility_label("Source")
                                    .tooltip("Source")
                                    .selected(!preview)
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        if let Some(i) = view.active {
                                            view.tabs[i].set_preview(false);
                                            view.tabs[i].focus_source(window, cx);
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                },
            )
            .into_any_element()
    }
}

fn selection_after_close(active: Option<usize>, closed: usize, count: usize) -> Option<usize> {
    active.and_then(|active| {
        if count <= 1 {
            None
        } else if closed < active {
            Some(active - 1)
        } else {
            Some(active.min(count - 2))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::selection_after_close;

    #[test]
    fn closing_another_tab_preserves_the_selected_document() {
        for count in 2..10 {
            let tabs = (0..count).collect::<Vec<_>>();
            for active in 0..count {
                for closed in 0..count {
                    if active == closed {
                        continue;
                    }
                    let next = selection_after_close(Some(active), closed, count).unwrap();
                    let mut remaining = tabs.clone();
                    remaining.remove(closed);
                    assert_eq!(remaining[next], tabs[active]);
                }
            }
        }
    }

    #[test]
    fn closing_active_selects_a_neighbor_and_last_close_clears_selection() {
        assert_eq!(selection_after_close(Some(1), 1, 3), Some(1));
        assert_eq!(selection_after_close(Some(2), 2, 3), Some(1));
        assert_eq!(selection_after_close(Some(0), 0, 1), None);
    }
}
