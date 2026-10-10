use super::Workspace;
use crate::views::reader::Reader;
use gpui_kit::{
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        h_flex,
        menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    },
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
            window.set_window_title("Glim");
            self.record_history();
            cx.notify();
        }
    }

    fn close_tabs(
        &mut self,
        targets: Vec<super::header::Target>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if targets.is_empty() {
            return;
        }
        if self.is_saving()
            || self
                .tabs
                .iter()
                .any(|r| r.is_dirty() && targets.contains(&super::header::Target::from_reader(r)))
        {
            self.confirm_discard(
                super::editing::DiscardAction::CloseTabs(targets),
                window,
                cx,
            );
        } else {
            self.close_tabs_unchecked(targets, window, cx);
        }
    }
    pub(super) fn close_tabs_unchecked(
        &mut self,
        targets: Vec<super::header::Target>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() {
            self.error = Some(
                "A save or Git operation is in progress. Please retry after it finishes.".into(),
            );
            cx.notify();
            return;
        }
        let active = self.active_reader().map(super::header::Target::from_reader);
        let previous = self.active.unwrap_or(0);
        self.tabs
            .retain(|r| !targets.contains(&super::header::Target::from_reader(r)));
        self.load_task = None;
        self.loading = false;
        self.active = active
            .and_then(|target| {
                self.tabs
                    .iter()
                    .position(|r| super::header::Target::from_reader(r) == target)
            })
            .or_else(|| (!self.tabs.is_empty()).then(|| previous.min(self.tabs.len() - 1)));
        self.observe_autosave(window, cx);
        if let Some(index) = self.active {
            self.activate_tab(index, window, cx);
        } else {
            window.set_window_title("Glim");
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }
    fn tab_context_menu(
        &self,
        mut menu: PopupMenu,
        target: &super::header::Target,
        cx: &mut Context<Self>,
    ) -> PopupMenu {
        let Some(index) = self
            .tabs
            .iter()
            .position(|r| super::header::Target::from_reader(r) == *target)
        else {
            return menu;
        };
        for (label, mode) in [
            ("Close", 0),
            ("Close Tabs to the Right", 1),
            ("Close Other Tabs", 2),
            ("Close All Tabs", 3),
        ] {
            let targets: Vec<_> = self
                .tabs
                .iter()
                .enumerate()
                .filter(|(i, _)| match mode {
                    0 => *i == index,
                    1 => *i > index,
                    2 => *i != index,
                    _ => true,
                })
                .map(|(_, r)| super::header::Target::from_reader(r))
                .collect();
            let view = cx.entity().downgrade();
            menu = menu.item(
                PopupMenuItem::new(label)
                    .disabled(targets.is_empty())
                    .on_click(move |_, w, cx| {
                        let _ = view.update(cx, |v, cx| v.close_tabs(targets.clone(), w, cx));
                    }),
            );
        }
        menu
    }

    pub(super) fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.get(index).is_some_and(|r| r.is_dirty()) || self.is_saving() {
            self.confirm_discard(super::editing::DiscardAction::CloseTab(index), window, cx);
        } else {
            self.close_tab_unchecked(index, window, cx);
        }
    }

    pub(super) fn close_tab_unchecked(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index >= self.tabs.len() {
            return;
        }
        self.load_task = None;
        self.loading = false;
        self.active = selection_after_close(self.active, index, self.tabs.len());
        self.tabs.remove(index);
        self.observe_autosave(window, cx);
        if let Some(active) = self.active {
            self.activate_tab(active, window, cx);
        } else {
            window.set_window_title("Glim");
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
                    let target = super::header::Target::from_reader(reader);
                    let menu_view = cx.entity().downgrade();
                    let mut label = reader
                        .commit_file
                        .as_ref()
                        .unwrap_or(&reader.path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    if reader.commit_file.is_some()
                        && let Some(oid) = &reader.commit_id
                    {
                        label.push_str(&format!(" · {}", &oid[..8.min(oid.len())]));
                    }
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
                            h_flex()
                                .gap_2()
                                .child(crate::views::file_icons::file_icon(&reader.path))
                                .child(div().max_w(px(210.)).truncate().text_sm().child(label)),
                        )
                        .child(
                            Button::new(("close-tab", index))
                                .ghost()
                                .label(if reader.is_dirty() { "·" } else { "×" })
                                .text_color(cx.theme().muted_foreground)
                                .tooltip(if reader.is_dirty() {
                                    "Unsaved changes · Close"
                                } else {
                                    "Close"
                                })
                                .accessibility_label(format!("Close {}", reader.title))
                                .on_click(cx.listener(move |view, _, window, cx| {
                                    cx.stop_propagation();
                                    view.close_tab(index, window, cx);
                                })),
                        )
                        .context_menu(move |menu, _, cx| {
                            menu_view
                                .update(cx, |v, cx| v.tab_context_menu(menu, &target, cx))
                                .expect("Tab menu owner is alive")
                        })
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
