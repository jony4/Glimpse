use super::Workspace;
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, TitleBar,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        h_flex,
        input::Input,
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Target {
    pub path: PathBuf,
    pub diff: Option<glimpse_core::GitChange>,
    pub root: Option<PathBuf>,
}
impl Workspace {
    pub(super) fn record_history(&mut self) {
        if self.navigating {
            self.navigating = false;
            return;
        }
        let Some(r) = self.active_reader() else {
            return;
        };
        let target = Target {
            path: r.path.clone(),
            diff: r.diff.clone(),
            root: r.repository_root.clone(),
        };
        if self.history_cursor.and_then(|i| self.history.get(i)) == Some(&target) {
            return;
        }
        self.history
            .truncate(self.history_cursor.map_or(0, |i| i + 1));
        self.history.push(target);
        self.history_cursor = Some(self.history.len() - 1);
    }
    fn navigate(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cursor) = self.history_cursor else {
            return;
        };
        let next = if forward {
            cursor + 1
        } else {
            cursor.saturating_sub(1)
        };
        let Some(target) = self.history.get(next).cloned() else {
            return;
        };
        if next == cursor {
            return;
        }
        self.history_cursor = Some(next);
        self.navigating = true;
        if let (Some(root), Some(change)) = (target.root, target.diff) {
            self.open_diff(root, change, window, cx);
        } else {
            self.open_path(target.path, window, cx);
        }
    }
    pub(super) fn search_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().to_string();
        self.results.clear();
        self.search_task = None;
        if query.trim().is_empty() {
            cx.notify();
            return;
        }
        let Some(root) = self.root.clone() else {
            cx.notify();
            return;
        };
        self.search_task = Some(cx.spawn_in(window, async move |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(150))
                .await;
            let results = cx
                .background_executor()
                .spawn(async move { glimpse_services::workspace::search_files(&root, &query) })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.results = results;
                cx.notify();
            });
        }));
        cx.notify();
    }
    pub(super) fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w_full()
            .flex_shrink_0()
            .child(
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .gap_2()
                        .child(
                            Button::new("back")
                                .ghost()
                                .label("←")
                                .accessibility_label("Back")
                                .disabled(self.history_cursor.is_none_or(|i| i == 0))
                                .on_click(cx.listener(|v, _, w, cx| v.navigate(false, w, cx))),
                        )
                        .child(
                            Button::new("forward")
                                .ghost()
                                .label("→")
                                .accessibility_label("Forward")
                                .disabled(
                                    self.history_cursor
                                        .is_none_or(|i| i + 1 >= self.history.len()),
                                )
                                .on_click(cx.listener(|v, _, w, cx| v.navigate(true, w, cx))),
                        )
                        .child(div().w(px(380.)).child(Input::new(&self.search))),
                ),
            )
            .when(!self.results.is_empty(), |v| {
                v.child(
                    h_flex().w_full().justify_center().child(
                        v_flex()
                            .id("file-search-results")
                            .w(px(560.))
                            .max_h(px(280.))
                            .overflow_y_scroll()
                            .bg(cx.theme().background)
                            .border_1()
                            .border_color(cx.theme().border)
                            .children(self.results.iter().enumerate().map(|(i, path)| {
                                let target = path.clone();
                                let label = path
                                    .strip_prefix(
                                        self.root.as_deref().unwrap_or(std::path::Path::new("")),
                                    )
                                    .unwrap_or(path)
                                    .to_string_lossy()
                                    .into_owned();
                                Button::new(("search-result", i))
                                    .custom(ButtonCustomVariant::new(cx))
                                    .accessibility_label(format!("Open {label}"))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .gap_2()
                                            .child(crate::views::file_icons::file_icon(path))
                                            .child(div().truncate().child(label)),
                                    )
                                    .w_full()
                                    .on_click(cx.listener(move |v, _, w, cx| {
                                        v.search.update(cx, |s, cx| s.set_value("", w, cx));
                                        v.open_path(target.clone(), w, cx);
                                    }))
                            })),
                    ),
                )
            })
            .into_any_element()
    }
}
