use gpui_kit::{
    component::{
        ActiveTheme, Disableable, Selectable,
        button::{Button, ButtonVariants},
        h_flex,
        resizable::{h_resizable, resizable_panel},
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};

use super::{Sidebar, Workspace};
use crate::{
    app::actions::{CloseWindow, OpenFile, OpenFolder, Refresh},
    views::{reader::Reader, welcome::welcome},
};

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self
            .reader
            .as_ref()
            .map(|r| r.title.clone())
            .unwrap_or("Welcome".into());
        let repo = self.changes.as_ref().map(|v| &v.read(cx).repository);
        let git_status = repo
            .map(|r| format!("⑂ {} · {} changes", r.branch, r.changes.len()))
            .unwrap_or_else(|| "No Git repository".into());
        let changes_count = repo.map_or(0, |r| r.changes.len());
        let language = self.reader.as_ref().map_or("", |r| r.language);
        let content = div()
            .size_full()
            .overflow_hidden()
            .child(match &mut self.reader {
                Some(reader) => reader.render(cx),
                None => welcome(cx).into_any_element(),
            });
        let body = if let Some(explorer) = &self.explorer {
            let sidebar = v_flex()
                .size_full()
                .bg(cx.theme().sidebar)
                .child(
                    div()
                        .px_3()
                        .py_3()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
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
                .child(
                    h_flex()
                        .px_2()
                        .gap_1()
                        .pb_2()
                        .child(
                            Button::new("files-tab")
                                .ghost()
                                .label("Files")
                                .selected(self.sidebar == Sidebar::Files)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.sidebar = Sidebar::Files;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("changes-tab")
                                .ghost()
                                .label(format!("Changes {changes_count}"))
                                .disabled(self.changes.is_none())
                                .selected(self.sidebar == Sidebar::Changes)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.sidebar = Sidebar::Changes;
                                    cx.notify();
                                })),
                        ),
                )
                .child(div().flex_1().min_h_0().w_full().child(
                    match (self.sidebar, &self.changes) {
                        (Sidebar::Changes, Some(changes)) => changes.clone().into_any_element(),
                        _ => explorer.clone().into_any_element(),
                    },
                ));
            h_resizable("workspace-panels")
                .child(
                    resizable_panel()
                        .size(px(280.))
                        .size_range(px(200.)..px(520.))
                        .child(sidebar),
                )
                .child(
                    resizable_panel()
                        .size_range(px(300.)..px(10000.))
                        .child(content),
                )
                .into_any_element()
        } else {
            content.into_any_element()
        };

        v_flex()
            .size_full()
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|view, _: &OpenFile, window, cx| view.choose_path(false, window, cx)),
            )
            .on_action(
                cx.listener(|view, _: &OpenFolder, window, cx| view.choose_path(true, window, cx)),
            )
            .on_action(cx.listener(|view, _: &Refresh, window, cx| view.refresh(window, cx)))
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                h_flex()
                    .h(px(52.))
                    .flex_shrink_0()
                    .px_3()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(Button::new("open-folder").label("Open folder").on_click(
                        cx.listener(|view, _, window, cx| view.choose_path(true, window, cx)),
                    ))
                    .child(
                        Button::new("open-file")
                            .ghost()
                            .label("Open file")
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.choose_path(false, window, cx)
                            })),
                    )
                    .child(
                        Button::new("refresh")
                            .ghost()
                            .label("Refresh")
                            .disabled(self.loading)
                            .on_click(cx.listener(|view, _, window, cx| view.refresh(window, cx))),
                    )
                    .child(div().flex_1().min_w_0().text_sm().truncate().child(title))
                    .when(
                        self.reader
                            .as_ref()
                            .is_some_and(|reader| reader.diff.is_some()),
                        |bar| {
                            bar.child(Button::new("show-file").label("View file").on_click(
                                cx.listener(|view, _, window, cx| {
                                    if let Some(reader) = &view.reader {
                                        view.open_path(reader.path.clone(), window, cx);
                                    }
                                }),
                            ))
                        },
                    )
                    .when(
                        self.reader.as_ref().is_some_and(Reader::can_preview),
                        |bar| {
                            let label = self
                                .reader
                                .as_ref()
                                .map(Reader::toggle_label)
                                .unwrap_or("Preview");
                            bar.child(Button::new("toggle-preview").label(label).on_click(
                                cx.listener(|view, _, window, cx| {
                                    if let Some(reader) = &mut view.reader {
                                        reader.toggle_preview();
                                        if reader.toggle_label() == "Preview" {
                                            reader.focus_source(window, cx);
                                        }
                                        cx.notify();
                                    }
                                }),
                            ))
                        },
                    ),
            )
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
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.error = None;
                                    cx.notify();
                                })),
                        ),
                )
            })
            .child(div().flex_1().min_h_0().w_full().child(body))
            .child(
                h_flex()
                    .h(px(28.))
                    .flex_shrink_0()
                    .px_3()
                    .gap_3()
                    .text_xs()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.loading {
                        "Opening…"
                    } else {
                        "Read only"
                    })
                    .child(div().flex_1().child(git_status))
                    .child(language)
                    .child("⌘R Refresh"),
            )
    }
}
