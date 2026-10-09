use super::{Sidebar, Workspace};
use crate::views::{
    changes::{ChangeSelected, Changes},
    explorer::{Explorer, ExplorerEvent},
    reader::Reader,
};
use glimpse_core::{DiffDocument, Document, GitChange};
use glimpse_services::{
    files::read_document,
    git::read_diff,
    workspace::{FolderSnapshot, open_folder},
};
use gpui_kit::*;
use std::path::PathBuf;

enum Opened {
    Folder(FolderSnapshot),
    File(PathBuf, anyhow::Result<Content>),
}
pub(super) enum Content {
    File(Document),
    Image(glimpse_services::media::ImageDocument),
    Diff(DiffDocument, PathBuf),
    Unavailable(PathBuf, String),
}
impl Workspace {
    pub(super) fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.results.clear();
        if let Some(index) = self
            .tabs
            .iter()
            .position(|t| t.path == path && t.diff.is_none())
        {
            self.activate_tab(index, window, cx);
            return;
        }
        self.loading = true;
        self.error = None;
        let read = cx.background_executor().spawn(async move {
            if path.is_dir() {
                open_folder(&path).map(Opened::Folder)
            } else {
                let result = read_content(&path);
                Ok(Opened::File(path, result))
            }
        });
        self.load_task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = read.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.loading = false;
                match result {
                    Ok(Opened::Folder(folder)) => {
                        view.tabs.clear();
                        view.active = None;
                        view.history.clear();
                        view.history_cursor = None;
                        view.sidebar = Sidebar::Files;
                        view.install_folder(folder, window, cx);
                    }
                    Ok(Opened::File(_, Ok(document))) => view.install_content(document, window, cx),
                    Ok(Opened::File(path, Err(error))) => view.install_content(
                        Content::Unavailable(path, format!("{error:#}")),
                        window,
                        cx,
                    ),
                    Err(error) => view.error = Some(format!("{error:#}").into()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn install_folder(
        &mut self,
        snapshot: FolderSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.subscriptions.clear();
        self.refresh_task = None;
        window.set_window_title("Glimpse");
        self.root = Some(snapshot.root.clone());
        self.error = snapshot.git_warning.map(Into::into);
        self.watch = snapshot.watch;
        self.start_watching(window, cx);
        let explorer = cx.new(|cx| Explorer::new(snapshot.root, snapshot.entries, cx));
        self.subscriptions.push(cx.subscribe_in(
            &explorer,
            window,
            |view, _, event, window, cx| match event {
                ExplorerEvent::OpenFile(path) => view.open_path(path.clone(), window, cx),
                ExplorerEvent::Error(error) => {
                    view.error = Some(error.clone().into());
                    cx.notify();
                }
            },
        ));
        self.explorer = Some(explorer);
        let changes = cx.new(|cx| Changes::new(snapshot.repositories, window, cx));
        self.subscriptions.push(
            cx.subscribe_in(&changes, window, |view, _, event, window, cx| match event {
                ChangeSelected::Open(root, change) => {
                    view.open_diff(root.clone(), change.clone(), window, cx)
                }
                ChangeSelected::Refresh => view.refresh(window, cx),
            }),
        );
        self.changes = Some(changes);
    }
    pub(super) fn make_reader(content: Content, window: &mut Window, cx: &mut App) -> Reader {
        match content {
            Content::File(d) => Reader::new(d, window, cx),
            Content::Image(d) => Reader::from_image(d, window, cx),
            Content::Diff(d, r) => Reader::from_diff(d, &r, window, cx),
            Content::Unavailable(p, e) => Reader::unavailable(p, e, window, cx),
        }
    }
    fn install_content(&mut self, content: Content, window: &mut Window, cx: &mut Context<Self>) {
        let reader = Self::make_reader(content, window, cx);
        if let Some(i) = self.tabs.iter().position(|t| {
            t.path == reader.path
                && t.diff.as_ref().map(|d| d.scope) == reader.diff.as_ref().map(|d| d.scope)
        }) {
            self.tabs[i] = reader;
            self.active = Some(i);
        } else {
            self.tabs.push(reader);
            self.active = Some(self.tabs.len() - 1);
            self.tab_scroll.scroll_to_item(self.tabs.len() - 1);
        }
        self.record_history();
        window.set_window_title("Glimpse");
        if self.root.is_none() {
            self.watch_file_parent(window, cx);
        }
    }
    pub(super) fn open_diff(
        &mut self,
        root: PathBuf,
        change: GitChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(i) = self.tabs.iter().position(|t| {
            t.path == root.join(&change.path)
                && t.diff.as_ref().is_some_and(|d| d.scope == change.scope)
        }) {
            self.activate_tab(i, window, cx);
            return;
        }
        self.loading = true;
        self.error = None;
        let path = root.join(&change.path);
        let read = cx.background_executor().spawn(async move {
            read_diff(&root, &change)
                .map(|d| Content::Diff(d, root))
                .unwrap_or_else(|e| Content::Unavailable(path, format!("{e:#}")))
        });
        self.load_task = Some(cx.spawn_in(window, async move |view, cx| {
            let content = read.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.loading = false;
                view.install_content(content, window, cx);
                cx.notify();
            });
        }));
        cx.notify();
    }
    pub(super) fn choose_path(
        &mut self,
        folder: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: !folder,
            directories: folder,
            multiple: false,
            prompt: Some(if folder { "Open folder" } else { "Open file" }.into()),
        });
        self.picker_task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = selection.await;
            let _ = view.update_in(cx, |view, window, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            view.open_path(path, window, cx);
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(e)) => view.error = Some(e.to_string().into()),
                    Err(e) => view.error = Some(e.to_string().into()),
                }
                cx.notify();
            });
        }));
    }
}

pub(super) fn read_content(path: &std::path::Path) -> anyhow::Result<Content> {
    if glimpse_services::media::supports(path) {
        glimpse_services::media::read_image(path).map(Content::Image)
    } else {
        read_document(path).map(Content::File)
    }
}
