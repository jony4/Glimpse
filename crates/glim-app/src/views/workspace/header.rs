use super::Workspace;
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, Icon, Selectable, Sizable, TitleBar,
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
    pub diff: Option<glim_core::GitChange>,
    pub root: Option<PathBuf>,
    pub commit_id: Option<String>,
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
            commit_id: r.commit_id.clone(),
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
        if let (Some(root), Some(oid)) = (target.root.clone(), target.commit_id) {
            self.open_commit(root, oid, window, cx);
        } else if let (Some(root), Some(change)) = (target.root, target.diff) {
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
        let roots = self.roots.clone();
        self.search_task = Some(cx.spawn_in(window, async move |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(150))
                .await;
            let results = cx
                .background_executor()
                .spawn(async move {
                    let mut results = roots
                        .iter()
                        .flat_map(|root| glim_services::workspace::search_files(root, &query))
                        .collect::<Vec<_>>();
                    results.sort();
                    results.dedup();
                    results.truncate(60);
                    results
                })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.results = results;
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn toggle_word_wrap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.word_wrap = !self.word_wrap;
        let enabled = self.word_wrap;
        cx.set_global(crate::app::ReaderPreferences { word_wrap: enabled });
        for reader in &self.tabs {
            reader.set_word_wrap(enabled, window, cx);
        }
        let write = cx
            .background_executor()
            .spawn(async move { glim_services::preferences::save_word_wrap(enabled) });
        self.preference_task = Some(cx.spawn(async move |view, cx| {
            if let Err(error) = write.await {
                let _ = view.update(cx, |v, cx| {
                    v.error = Some(format!("Cannot remember word wrap: {error:#}").into());
                    cx.notify();
                });
            }
        }));
        cx.notify();
    }

    fn reader_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let subtle = ButtonCustomVariant::new(cx)
            .hover(cx.theme().foreground.opacity(0.04))
            .active(cx.theme().foreground.opacity(0.07));
        h_flex()
            .gap_1()
            .child(
                Button::new("word-wrap")
                    .custom(subtle)
                    .small()
                    .child(svg().path("navigation/wrap.svg").size(px(16.)))
                    .accessibility_label("Word wrap")
                    .tooltip("Word wrap")
                    .selected(self.word_wrap)
                    .on_click(cx.listener(|v, _, window, cx| v.toggle_word_wrap(window, cx))),
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
                                    .custom(subtle)
                                    .icon(Icon::new(gpui_kit::assets::IconName::Eye))
                                    .xsmall()
                                    .accessibility_label("Preview")
                                    .tooltip("Preview")
                                    .selected(preview)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        if let Some(i) = view.active {
                                            view.tabs[i].set_preview(true, cx);
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("markdown-source")
                                    .custom(subtle)
                                    .icon(Icon::new(gpui_kit::assets::IconName::Code))
                                    .xsmall()
                                    .accessibility_label("Source")
                                    .tooltip("Source")
                                    .selected(!preview)
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        if let Some(i) = view.active {
                                            view.tabs[i].set_preview(false, cx);
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

    pub(super) fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .relative()
            .on_key_down(cx.listener(|v, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    v.results.clear();
                    v.search_task = None;
                    cx.notify();
                }
            }))
            .w_full()
            .flex_shrink_0()
            .child(
                TitleBar::new().h(px(44.)).pl_0().pr_0().child(
                    h_flex()
                        .relative()
                        .w_full()
                        .h_full()
                        .justify_center()
                        .child(
                            div()
                                .relative()
                                .w(px(380.))
                                .child(Input::new(&self.search).small().focus_bordered(false))
                                .child(
                                    h_flex()
                                        .absolute()
                                        .right(relative(1.))
                                        .top_0()
                                        .h_full()
                                        .mr_2()
                                        .gap_1()
                                        .child(
                                            Button::new("back")
                                                .ghost()
                                                .small()
                                                .label("←")
                                                .accessibility_label("Back")
                                                .disabled(
                                                    self.history_cursor.is_none_or(|i| i == 0),
                                                )
                                                .on_click(cx.listener(|v, _, w, cx| {
                                                    v.navigate(false, w, cx)
                                                })),
                                        )
                                        .child(
                                            Button::new("forward")
                                                .ghost()
                                                .small()
                                                .label("→")
                                                .accessibility_label("Forward")
                                                .disabled(
                                                    self.history_cursor.is_none_or(|i| {
                                                        i + 1 >= self.history.len()
                                                    }),
                                                )
                                                .on_click(cx.listener(|v, _, w, cx| {
                                                    v.navigate(true, w, cx)
                                                })),
                                        ),
                                ),
                        )
                        .child(
                            h_flex()
                                .absolute()
                                .right(px(12.))
                                .top_0()
                                .h_full()
                                .child(self.reader_controls(cx)),
                        ),
                ),
            )
            .when(!self.results.is_empty(), |v| {
                v.child(
                    deferred(
                        h_flex()
                            .absolute()
                            .top(px(44.))
                            .left_0()
                            .w_full()
                            .justify_center()
                            .child(
                                v_flex()
                                    .id("file-search-results")
                                    .occlude()
                                    .shadow_lg()
                                    .on_mouse_down_out(cx.listener(|v, _, _, cx| {
                                        v.results.clear();
                                        v.search_task = None;
                                        cx.notify();
                                    }))
                                    .w(px(380.))
                                    .max_h(px(280.))
                                    .overflow_y_scroll()
                                    .bg(cx.theme().background)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .children(self.results.iter().enumerate().map(|(i, path)| {
                                        let target = path.clone();
                                        let label = path
                                            .strip_prefix(
                                                self.root
                                                    .as_deref()
                                                    .unwrap_or(std::path::Path::new("")),
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
                                                    .child(crate::views::file_icons::file_icon(
                                                        path,
                                                    ))
                                                    .child(div().truncate().child(label)),
                                            )
                                            .w_full()
                                            .on_click(cx.listener(move |v, _, w, cx| {
                                                v.search.update(cx, |s, cx| s.set_value("", w, cx));
                                                v.sidebar = super::Sidebar::Files;
                                                v.sidebar_visible = true;
                                                v.open_path(target.clone(), w, cx);
                                            }))
                                    })),
                            ),
                    )
                    .with_priority(1),
                )
            })
            .into_any_element()
    }
}
