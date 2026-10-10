use gpui_kit::{
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(super) struct NativeReader {
    path: PathBuf,
    folder: bool,
    shuffle: bool,
    task: Option<Task<()>>,
    cancelled: Arc<AtomicBool>,
    error: Option<String>,
}
impl NativeReader {
    pub fn new(path: PathBuf, folder: bool, shuffle: bool, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            path,
            folder,
            shuffle,
            task: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            error: None,
        };
        view.open(cx);
        view
    }
    fn open(&mut self, cx: &mut Context<Self>) {
        if self.task.is_some() {
            return;
        }
        self.error = None;
        let path = self.path.clone();
        let folder = self.folder;
        let shuffle = self.shuffle;
        let cancelled = self.cancelled.clone();
        let work = cx
            .background_executor()
            .spawn(async move { glim_services::preview::open(&path, folder, shuffle, cancelled) });
        self.task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |v, cx| {
                v.task = None;
                if let Err(e) = result {
                    v.error = Some(format!("{e:#}"));
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
}
impl Drop for NativeReader {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
impl Render for NativeReader {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_6()
            .child(
                div().text_lg().child(
                    self.path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.task.is_some() {
                        "Open in the Glim preview window. Close this tab to close the preview."
                    } else {
                        "Documents and media use a native preview window."
                    }),
            )
            .when_some(self.error.clone(), |view, error| {
                view.child(div().text_sm().text_color(cx.theme().danger).child(error))
            })
            .child(
                Button::new("open-native-preview")
                    .primary()
                    .label("Open Preview")
                    .disabled(self.task.is_some())
                    .on_click(cx.listener(|v, _, _, cx| v.open(cx))),
            )
    }
}
