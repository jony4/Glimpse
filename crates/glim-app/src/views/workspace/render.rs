use super::{Sidebar, Workspace};
use crate::{
    app::actions::{AddFolder, CloseWindow, OpenFile, OpenFolder, Refresh, SaveFile},
    views::welcome::welcome,
};
use gpui_kit::{
    component::{
        ActiveTheme,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        h_flex,
        menu::{ContextMenuExt, PopupMenuItem},
        resizable::{h_resizable, resizable_panel},
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(mode) = cx
            .try_global::<crate::app::ReaderPreferences>()
            .and_then(|p| p.diff_side_by_side)
        {
            for reader in &mut self.tabs {
                reader.set_side_by_side(mode);
            }
        }
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
        let view = cx.entity().downgrade();
        let open: crate::views::reader::OpenLink =
            std::sync::Arc::new(move |path, anchor, window, cx| {
                let _ = view.update(cx, |v, cx| {
                    v.sidebar = Sidebar::Files;
                    v.open_path(path.clone(), window, cx);
                    if let Some(anchor) = anchor {
                        if let Some(i) = v
                            .active
                            .filter(|i| v.tabs[*i].path == path && v.tabs[*i].diff.is_none())
                        {
                            v.tabs[i].reveal_anchor(&anchor, cx);
                        } else {
                            v.pending_anchor = Some((path, anchor));
                        }
                    }
                    cx.notify();
                });
            });
        let content = match self.active.and_then(|i| self.tabs.get_mut(i)) {
            Some(reader) => reader.render(open, cx),
            None if !self.sidebar_visible => div()
                .relative()
                .size_full()
                .child(
                    // Anchor the empty state to the entire window, including the activity bar
                    // and title bar, so it shares the header search field's center line.
                    div()
                        .absolute()
                        .right_0()
                        .bottom_0()
                        .w(window.viewport_size().width)
                        .h(window.viewport_size().height)
                        .child(welcome(self.roots.is_empty(), cx)),
                )
                .into_any_element(),
            None => welcome(self.roots.is_empty(), cx).into_any_element(),
        };
        let gesture_view = cx.entity().downgrade();
        let viewer = v_flex()
            .size_full()
            .min_w_0()
            .overflow_hidden()
            .when(!self.tabs.is_empty(), |view| view.child(tab_bar))
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
                        .child(div().flex_1().min_w_0().truncate().child(path)),
                )
            })
            .when_some(
                self.active_reader()
                    .and_then(|reader| reader.history_summary()),
                |view, (subject, detail)| {
                    view.child(
                        v_flex()
                            .flex_shrink_0()
                            .px_4()
                            .py_2()
                            .gap_1()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .child(subject),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .truncate()
                                    .child(detail),
                            ),
                    )
                },
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(content)
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, window, _| {
                                let view = gesture_view.clone();
                                window.on_mouse_event(
                                    move |event: &ScrollWheelEvent, phase, window, cx| {
                                        if phase == DispatchPhase::Capture
                                            && bounds.contains(&event.position)
                                        {
                                            let _ = view.update(cx, |v, cx| {
                                                v.history_swipe(event, window, cx)
                                            });
                                        }
                                    },
                                );
                            },
                        )
                        .absolute()
                        .size_full(),
                    ),
            );
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
                        "Open a folder containing a Git repository."
                    },
                ))
                .child(
                    Button::new("empty-open-folder")
                        .primary()
                        .w_full()
                        .label("Open Folder")
                        .on_click(cx.listener(|v, _, w, cx| v.choose_path(true, w, cx))),
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
                    ),
            )
            .child(div().flex_1().min_h_0().w_full().child(panel_content))
            .id("workspace-sidebar");
        let sidebar = if self.sidebar == Sidebar::Files && self.explorer.is_none() {
            sidebar
                .context_menu(|menu, _, _| {
                    menu.item(PopupMenuItem::new("Add Folder to Workspace…").on_click(
                        |_, window, cx| {
                            window.dispatch_action(Box::new(AddFolder), cx);
                        },
                    ))
                })
                .into_any_element()
        } else {
            sidebar.into_any_element()
        };
        let activity = v_flex()
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
                    div()
                        .w(px(52.))
                        .h(px(48.))
                        .border_l_2()
                        .border_color(if self.sidebar_visible && self.sidebar == mode {
                            cx.theme().primary
                        } else {
                            gpui_kit::transparent_black()
                        })
                        .child(
                            Button::new(id)
                                .custom(ButtonCustomVariant::new(cx))
                                .size_full()
                                .rounded_none()
                                .accessibility_label(label)
                                .tooltip(label)
                                .child(svg().path(icon).size(px(25.)).text_color(
                                    if self.sidebar_visible && self.sidebar == mode {
                                        cx.theme().foreground
                                    } else {
                                        cx.theme().muted_foreground
                                    },
                                ))
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.sidebar_visible =
                                        view.sidebar != mode || !view.sidebar_visible;
                                    view.sidebar = mode;
                                    cx.notify();
                                })),
                        )
                }),
            );
        v_flex()
            .relative()
            .size_full()
            .track_focus(&self.focus)
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(header)
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Back),
                cx.listener(|v, _, w, cx| {
                    v.history_swipe = None;
                    v.navigate(false, w, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Forward),
                cx.listener(|v, _, w, cx| {
                    v.history_swipe = None;
                    v.navigate(true, w, cx);
                    cx.stop_propagation();
                }),
            )
            .on_action(cx.listener(|v, _: &AddFolder, w, cx| v.choose_add_folder(w, cx)))
            .on_action(cx.listener(|v, _: &OpenFile, w, cx| v.choose_path(false, w, cx)))
            .on_action(cx.listener(|v, _: &OpenFolder, w, cx| v.choose_path(true, w, cx)))
            .on_action(cx.listener(|v, _: &Refresh, w, cx| v.refresh(w, cx)))
            .on_action(cx.listener(|v, _: &SaveFile, w, cx| v.save_active(w, cx)))
            .on_action(cx.listener(|v, _: &CloseWindow, w, cx| {
                if let Some(i) = v.active {
                    v.close_tab(i, w, cx);
                } else {
                    v.request_close_window(w, cx);
                }
            }))
            .child(
                h_flex().flex_1().min_h_0().w_full().child(activity).child(
                    div().flex_1().min_w_0().h_full().child(
                        h_resizable("workspace-panels")
                            .child(
                                resizable_panel()
                                    .visible(self.sidebar_visible)
                                    .size(px(320.))
                                    .flex_none()
                                    .size_range(px(180.)..px(420.))
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
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    deferred(
                        v_flex()
                            .absolute()
                            .right(px(16.))
                            .bottom(px(16.))
                            .w(px(420.))
                            .max_w(relative(0.9))
                            .p_3()
                            .gap_2()
                            .rounded_lg()
                            .bg(cx.theme().background)
                            .border_1()
                            .border_color(cx.theme().border)
                            .shadow_lg()
                            .occlude()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().danger)
                                            .child("操作未完成"),
                                    )
                                    .child(
                                        Button::new("dismiss-error")
                                            .ghost()
                                            .label("×")
                                            .accessibility_label("关闭错误提示")
                                            .on_click(cx.listener(|v, _, _, cx| {
                                                v.error = None;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .id("workspace-error-message")
                                    .max_h(px(160.))
                                    .overflow_y_scroll()
                                    .text_sm()
                                    .whitespace_normal()
                                    .text_color(cx.theme().foreground)
                                    .child(error),
                            ),
                    )
                    .with_priority(2),
                )
            })
    }
}
