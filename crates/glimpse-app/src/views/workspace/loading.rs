use std::path::PathBuf;

use anyhow::Result;
use glimpse_core::{DiffDocument, Document, GitChange};
use glimpse_services::{
    files::read_document,
    git::read_diff,
    workspace::{FolderSnapshot, open_folder},
};
use gpui_kit::*;

use super::{Sidebar, Workspace};
use crate::views::{
    changes::{ChangeSelected, Changes},
    explorer::{Explorer, ExplorerEvent},
    reader::Reader,
};

enum Opened {
    Folder(FolderSnapshot),
    File(Document),
}
enum Content {
    File(Document),
    Diff(DiffDocument, PathBuf),
}

impl Workspace {
    pub(super) fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.loading = true;
        self.error = None;
        let read = cx.background_executor().spawn(async move {
            if path.is_dir() {
                open_folder(&path).map(Opened::Folder)
            } else {
                read_document(&path).map(Opened::File)
            }
        });
        self.load_task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = read.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.loading = false;
                match result {
                    Ok(Opened::Folder(folder)) => {
                        view.reader = None;
                        view.sidebar = Sidebar::Files;
                        view.install_folder(folder, window, cx);
                    }
                    Ok(Opened::File(document)) => {
                        view.install_content(Content::File(document), window, cx)
                    }
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
        window.set_window_title(&format!("{} — Glimpse", snapshot.root.display()));
        self.root = Some(snapshot.root.clone());
        self.error = snapshot.git_warning.map(Into::into);
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
        self.changes = snapshot.repository.map(|repository| {
            let changes = cx.new(|_| Changes::new(repository));
            self.subscriptions.push(cx.subscribe_in(
                &changes,
                window,
                |view, _, event: &ChangeSelected, window, cx| {
                    view.open_diff(event.0.clone(), window, cx);
                },
            ));
            changes
        });
        if self.changes.is_none() {
            self.sidebar = Sidebar::Files;
        }
    }

    fn install_content(&mut self, content: Content, window: &mut Window, cx: &mut Context<Self>) {
        let reader = match content {
            Content::File(document) => Reader::new(document, window, cx),
            Content::Diff(document, root) => Reader::from_diff(document, &root, window, cx),
        };
        window.set_window_title(&format!("{} — Glimpse", reader.title));
        self.reader = Some(reader);
    }

    pub(super) fn open_diff(
        &mut self,
        change: GitChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(repository_root) = self
            .changes
            .as_ref()
            .map(|view| view.read(cx).repository.root.clone())
        else {
            return;
        };
        self.loading = true;
        self.error = None;
        let read = cx.background_executor().spawn(async move {
            read_diff(&repository_root, &change).map(|diff| Content::Diff(diff, repository_root))
        });
        self.load_task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = read.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.loading = false;
                match result {
                    Ok(content) => view.install_content(content, window, cx),
                    Err(error) => view.error = Some(format!("{error:#}").into()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let document = self
            .reader
            .as_ref()
            .map(|reader| (reader.path.clone(), reader.diff.clone()));
        let repository_root = self
            .changes
            .as_ref()
            .map(|view| view.read(cx).repository.root.clone());
        if root.is_none() && document.is_none() {
            return;
        }
        self.loading = true;
        self.error = None;
        let read = cx.background_executor().spawn(async move {
            let folder = root.as_deref().map(open_folder).transpose();
            let content: Result<Option<Content>> = document
                .map(|(path, change)| {
                    if let (Some(change), Some(root)) = (change, repository_root) {
                        read_diff(&root, &change).map(|diff| Content::Diff(diff, root))
                    } else {
                        read_document(&path).map(Content::File)
                    }
                })
                .transpose();
            (folder, content)
        });
        self.load_task = Some(cx.spawn_in(window, async move |view, cx| {
            let (folder, content) = read.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.loading = false;
                match folder {
                    Ok(Some(folder)) => view.install_folder(folder, window, cx),
                    Ok(None) => {}
                    Err(error) => view.error = Some(format!("{error:#}").into()),
                }
                match content {
                    Ok(Some(content)) => view.install_content(content, window, cx),
                    Ok(None) => {}
                    Err(error) => view.error = Some(format!("{error:#}").into()),
                }
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
                    Ok(Err(error)) => view.error = Some(error.to_string().into()),
                    Err(error) => view.error = Some(error.to_string().into()),
                }
                cx.notify();
            });
        }));
    }
}
