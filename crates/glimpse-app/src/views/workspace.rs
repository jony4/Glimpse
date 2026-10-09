use std::path::PathBuf;

use glimpse_services::files::read_document;
use gpui_kit::{
    component::{ActiveTheme, Disableable, button::Button, h_flex, v_flex},
    prelude::FluentBuilder,
    *,
};

use super::{reader::Reader, welcome::welcome};
use crate::app::actions::OpenFile;

pub struct Workspace {
    focus: FocusHandle,
    reader: Option<Reader>,
    error: Option<SharedString>,
    loading: bool,
    // Retain the task for the lifetime of this workspace; opening another file cancels it.
    load_task: Option<Task<()>>,
}

impl Workspace {
    pub fn new(path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let mut workspace = Self {
            focus,
            reader: None,
            error: None,
            loading: false,
            load_task: None,
        };
        if let Some(path) = path {
            workspace.load(path, window, cx);
        }
        workspace
    }

    fn load(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.loading = true;
        self.error = None;
        let read = cx
            .background_executor()
            .spawn(async move { read_document(&path) });
        self.load_task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = read.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.loading = false;
                match result {
                    Ok(document) => {
                        window.set_window_title(&format!("{} — Glimpse", document.path.display()));
                        view.reader = Some(Reader::new(document, window, cx));
                    }
                    Err(error) => view.error = Some(format!("{error:#}").into()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn open_file(&mut self, _: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open file".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = selection.await;
            let _ = view.update_in(cx, |view, window, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            view.load(path, window, cx);
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => view.error = Some(error.to_string().into()),
                    Err(error) => view.error = Some(error.to_string().into()),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self
            .reader
            .as_ref()
            .map(|reader| reader.title.clone())
            .unwrap_or("Welcome".into());
        v_flex()
            .size_full()
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::open_file))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                h_flex()
                    .h(px(52.))
                    .flex_shrink_0()
                    .px_4()
                    .gap_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("open-file")
                            .label("Open…")
                            .disabled(self.loading)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.open_file(&OpenFile, window, cx)
                            })),
                    )
                    .child(div().flex_1().min_w_0().text_sm().truncate().child(title))
                    .when(
                        self.reader.as_ref().is_some_and(Reader::can_preview),
                        |bar| {
                            let label = self
                                .reader
                                .as_ref()
                                .map(Reader::toggle_label)
                                .unwrap_or("Preview");
                            bar.child(Button::new("toggle-preview").label(label).on_click(
                                cx.listener(|view, _, _, cx| {
                                    if let Some(reader) = &mut view.reader {
                                        reader.toggle_preview();
                                        cx.notify();
                                    }
                                }),
                            ))
                        },
                    ),
            )
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    div()
                        .p_3()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .child(match &self.reader {
                        Some(reader) => reader.render(),
                        None => welcome(cx).into_any_element(),
                    }),
            )
            .child(
                h_flex()
                    .h(px(28.))
                    .flex_shrink_0()
                    .px_4()
                    .text_xs()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.loading {
                        "Opening…"
                    } else {
                        "Read only"
                    }),
            )
    }
}
