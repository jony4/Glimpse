use super::{Sidebar, Workspace};
use crate::{
    app::actions::{CloseWindow, OpenFile, OpenFolder, Refresh},
    views::welcome::welcome,
};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        h_flex,
        resizable::{h_resizable, resizable_panel},
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let format = self
            .active_reader()
            .map(|r| {
                if r.diff.is_some() {
                    "DIFF".to_owned()
                } else {
                    r.path
                        .extension()
                        .and_then(|v| v.to_str())
                        .unwrap_or("Text")
                        .to_uppercase()
                }
            })
            .unwrap_or_default();
        let path = self.active_reader().map(|r| {
            r.path
                .strip_prefix(self.root.as_deref().unwrap_or(std::path::Path::new("")))
                .unwrap_or(&r.path)
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("  ›  ")
        });
        let header = self.header(cx);
        let tab_bar = self.tab_bar(cx);
        let content = match self.active.and_then(|i| self.tabs.get_mut(i)) {
            Some(reader) => reader.render(cx),
            None => welcome(cx).into_any_element(),
        };
        let viewer = v_flex()
            .size_full()
            .min_w_0()
            .overflow_hidden()
            .child(tab_bar)
            .when_some(path, |view, path| {
                view.child(
                    h_flex()
                        .h(px(30.))
                        .flex_shrink_0()
                        .px_4()
                        .text_xs()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .text_color(cx.theme().muted_foreground)
                        .child(div().flex_1().min_w_0().truncate().child(path))
                        .when(
                            self.active_reader().is_some_and(|r| r.diff.is_some()),
                            |bar| {
                                bar.child(
                                    Button::new("show-file")
                                        .ghost()
                                        .label("View file")
                                        .on_click(cx.listener(|view, _, window, cx| {
                                            if let Some(reader) = view.active_reader() {
                                                view.open_path(reader.path.clone(), window, cx);
                                            }
                                        })),
                                )
                            },
                        ),
                )
            })
            .child(div().flex_1().min_h_0().w_full().child(content));
        let panel_content = match self.sidebar {
            Sidebar::Files => self.explorer.as_ref().map(|v| v.clone().into_any_element()),
            Sidebar::Changes => self.changes.as_ref().map(|v| v.clone().into_any_element()),
        }
        .unwrap_or_else(|| {
            v_flex()
                .p_5()
                .gap_5()
                .text_sm()
                .child(div().font_weight(FontWeight::SEMIBOLD).child(
                    if self.sidebar == Sidebar::Files {
                        "No Folder Opened"
                    } else {
                        "Source Control"
                    },
                ))
                .child(div().text_color(cx.theme().muted_foreground).child(
                    if self.sidebar == Sidebar::Files {
                        "You have not yet opened a folder."
                    } else {
                        "Open a folder containing a Git repository, or clone one from a URL."
                    },
                ))
                .child(
                    Button::new("empty-open-folder")
                        .primary()
                        .w_full()
                        .label("Open Folder")
                        .on_click(cx.listener(|v, _, w, cx| v.choose_path(true, w, cx))),
                )
                .child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child("You can also clone a repository locally."),
                )
                .child(gpui_kit::component::input::Input::new(&self.clone_url).w_full())
                .child(
                    Button::new("clone-repository")
                        .primary()
                        .w_full()
                        .label("Clone Repository")
                        .disabled(self.loading)
                        .on_click(cx.listener(|v, _, w, cx| v.clone_repository(w, cx))),
                )
                .into_any_element()
        });
        let sidebar = v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .child(
                h_flex()
                    .h(px(40.))
                    .flex_shrink_0()
                    .px_3()
                    .gap_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(if self.sidebar == Sidebar::Files {
                                "EXPLORER"
                            } else {
                                "SOURCE CONTROL"
                            }),
                    )
                    .child(
                        Button::new("open-folder")
                            .ghost()
                            .label("＋")
                            .accessibility_label("Open folder")
                            .on_click(cx.listener(|v, _, w, cx| v.choose_path(true, w, cx))),
                    )
                    .child(
                        Button::new("refresh")
                            .ghost()
                            .label("↻")
                            .accessibility_label("Refresh")
                            .disabled(self.loading)
                            .on_click(cx.listener(|v, _, w, cx| v.refresh(w, cx))),
                    ),
            )
            .child(
                div()
                    .h(px(30.))
                    .px_3()
                    .text_xs()
                    .content_center()
                    .truncate()
                    .child(
                        self.root
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                    ),
            )
            .child(div().flex_1().min_h_0().w_full().child(panel_content));
        let activity =
            v_flex()
                .w(px(52.))
                .h_full()
                .flex_shrink_0()
                .py_2()
                .gap_3()
                .items_center()
                .bg(cx.theme().sidebar)
                .border_r_1()
                .border_color(cx.theme().border)
                .children(
                    [
                        (
                            Sidebar::Files,
                            "files-activity",
                            "navigation/files.svg",
                            "Files",
                        ),
                        (
                            Sidebar::Changes,
                            "git-activity",
                            "navigation/git.svg",
                            "Git changes",
                        ),
                    ]
                    .into_iter()
                    .map(|(mode, id, icon, label)| {
                        Button::new(id)
                            .custom(ButtonCustomVariant::new(cx))
                            .w(px(52.))
                            .h(px(48.))
                            .rounded_none()
                            .border_l_2()
                            .border_color(if self.sidebar == mode {
                                cx.theme().primary
                            } else {
                                gpui_kit::transparent_black()
                            })
                            .accessibility_label(label)
                            .child(svg().path(icon).size(px(25.)).text_color(
                                if self.sidebar == mode {
                                    cx.theme().foreground
                                } else {
                                    cx.theme().muted_foreground
                                },
                            ))
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.sidebar = mode;
                                cx.notify();
                            }))
                    }),
                );
        v_flex()
            .size_full()
            .track_focus(&self.focus)
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(header)
            .on_action(cx.listener(|v, _: &OpenFile, w, cx| v.choose_path(false, w, cx)))
            .on_action(cx.listener(|v, _: &OpenFolder, w, cx| v.choose_path(true, w, cx)))
            .on_action(cx.listener(|v, _: &Refresh, w, cx| v.refresh(w, cx)))
            .on_action(cx.listener(|v, _: &CloseWindow, w, cx| {
                if let Some(i) = v.active {
                    v.close_tab(i, w, cx);
                } else {
                    w.remove_window();
                }
            }))
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    h_flex()
                        .px_3()
                        .py_2()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(error),
                        )
                        .child(
                            Button::new("dismiss-error")
                                .ghost()
                                .label("Dismiss")
                                .on_click(cx.listener(|v, _, _, cx| {
                                    v.error = None;
                                    cx.notify();
                                })),
                        ),
                )
            })
            .child(
                h_flex().flex_1().min_h_0().w_full().child(activity).child(
                    div().flex_1().min_w_0().h_full().child(
                        h_resizable("workspace-panels")
                            .child(
                                resizable_panel()
                                    .size(px(240.))
                                    .size_range(px(180.)..px(520.))
                                    .child(sidebar),
                            )
                            .child(
                                resizable_panel()
                                    .size_range(px(280.)..px(10000.))
                                    .child(viewer),
                            ),
                    ),
                ),
            )
            .child(
                h_flex()
                    .h(px(24.))
                    .flex_shrink_0()
                    .px_3()
                    .gap_3()
                    .text_xs()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_color(cx.theme().muted_foreground)
                    .child(div().flex_1())
                    .child(format),
            )
    }
}
