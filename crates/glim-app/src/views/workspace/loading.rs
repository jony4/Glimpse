use super::Workspace;
use crate::views::{
    changes::{ChangeSelected, Changes},
    explorer::{Explorer, ExplorerEvent},
    reader::Reader,
};
use glim_core::{DiffDocument, Document, GitChange};
use glim_services::{
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
    Page(glim_services::paged::TextPage),
    Bytes(glim_services::binary::BytePreview),
    Commit(glim_services::binary::BytePreview),
    Image(glim_services::media::ImageDocument),
    Diff(DiffDocument, PathBuf),
    Unavailable(PathBuf, String),
}
impl Workspace {
    pub(super) fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.results.clear();
        self.search_task = None;
        self.pending_anchor = None;
        if let Some(index) = self
            .tabs
            .iter()
            .position(|t| t.path == path && t.diff.is_none() && !t.historical)
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
                        view.receive_folder(folder, window, cx);
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
    pub(super) fn install_folder(
        &mut self,
        snapshot: FolderSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.subscriptions.clear();
        self.explorer = None;
        self.changes = None;
        self.roots.clear();
        self.root = None;
        self.add_folder(snapshot, window, cx);
    }

    pub(super) fn add_folder(
        &mut self,
        snapshot: FolderSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar = super::Sidebar::Files;
        self.sidebar_visible = true;
        if self
            .roots
            .iter()
            .any(|root| snapshot.root.starts_with(root))
        {
            return;
        }
        self.roots.retain(|root| !root.starts_with(&snapshot.root));
        self.roots.push(snapshot.root.clone());
        self.root = self.roots.first().cloned();
        self.refresh_task = None;
        self.error = None;
        self.search_task = None;
        self.watch_setup = None;
        self.watch_task = None;
        self.watch = None;
        self.repository_task = None;
        if let Some(explorer) = &self.explorer {
            explorer.update(cx, |v, cx| {
                v.add_folder(snapshot.root, snapshot.entries, cx)
            });
        } else {
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
        }
        if self.changes.is_none() {
            let changes = cx.new(|cx| Changes::new(Vec::new(), window, cx));
            self.subscriptions.push(cx.subscribe_in(
                &changes,
                window,
                |view, _, event, window, cx| match event {
                    ChangeSelected::Open(root, change) => {
                        view.open_diff(root.clone(), change.clone(), window, cx)
                    }
                    ChangeSelected::Refresh => view.refresh(window, cx),
                    ChangeSelected::Error(message) => {
                        view.error = Some(message.clone().into());
                        cx.notify();
                    }
                    ChangeSelected::Git(root, request) => {
                        view.run_git(root.clone(), request.clone(), cx)
                    }
                    ChangeSelected::GitFinished => view.finish_git(window, cx),
                    ChangeSelected::Commit(root, oid) => {
                        view.open_commit(root.clone(), oid.clone(), window, cx)
                    }
                },
            ));
            changes.update(cx, |v, cx| {
                v.set_loading(true, cx);
                v.set_external_busy(self.git_edit_locked, cx);
            });
            self.changes = Some(changes);
        }
        let roots = self.roots.clone();
        let parents = self
            .tabs
            .iter()
            .filter_map(|r| r.path.parent().map(ToOwned::to_owned))
            .collect::<Vec<_>>();
        let work = cx.background_executor().spawn(async move {
            let root = &roots[0];
            let mut watch = glim_services::watch::WorkspaceWatch::new(root).ok();
            if let Some(watch) = &mut watch {
                for path in roots.iter().skip(1).chain(parents.iter()) {
                    let _ = watch.add_path(path);
                }
            }
            let repositories = glim_services::workspace::repositories_for_roots(&roots);
            if let (Some(watch), Ok(repositories)) = (&mut watch, &repositories) {
                let _ = watch.add_repositories(root, repositories);
            }
            (roots, repositories, watch)
        });
        self.repository_task = Some(cx.spawn_in(window, async move |view, cx| {
            let (roots, repositories, watch) = work.await;
            let _ = view.update_in(cx, |view, window, cx| {
                if view.roots != roots {
                    return;
                }
                view.repository_task = None;
                if let Some(changes) = &view.changes {
                    changes.update(cx, |v, cx| {
                        v.set_loading(false, cx);
                        match repositories {
                            Ok(repositories) => v.set_repositories(repositories, cx),
                            Err(error) => view.error = Some(format!("Git: {error:#}").into()),
                        }
                    });
                }
                view.watch = watch;
                view.start_watching(window, cx);
                cx.notify();
            });
        }));
    }
    pub(super) fn make_reader(content: Content, window: &mut Window, cx: &mut App) -> Reader {
        match content {
            Content::File(d) => Reader::new(d, window, cx),
            Content::Page(d) => Reader::from_page(d, cx),
            Content::Bytes(d) => Reader::from_bytes(d, window, cx),
            Content::Commit(d) => Reader::from_commit(d.path, d.text, window, cx),
            Content::Image(d) => Reader::from_image(d, window, cx),
            Content::Diff(d, r) => Reader::from_diff(d, &r, window, cx),
            Content::Unavailable(p, e) => Reader::unavailable(p, e, window, cx),
        }
    }
    fn install_content(&mut self, content: Content, window: &mut Window, cx: &mut Context<Self>) {
        let mut reader = Self::make_reader(content, window, cx);
        reader.set_editing_locked(self.git_edit_locked, cx);
        reader.set_word_wrap(self.word_wrap, window, cx);
        if let Some(i) = self.tabs.iter().position(|t| {
            t.path == reader.path
                && t.historical == reader.historical
                && t.diff.as_ref().map(|d| d.scope) == reader.diff.as_ref().map(|d| d.scope)
        }) {
            self.tabs[i] = reader;
            self.active = Some(i);
        } else {
            self.tabs.push(reader);
            self.active = Some(self.tabs.len() - 1);
            self.tab_scroll.scroll_to_item(self.tabs.len() - 1);
        }
        if let Some((path, anchor)) = self.pending_anchor.take()
            && let Some(i) = self.active
            && self.tabs[i].path == path
        {
            self.tabs[i].reveal_anchor(&anchor, cx);
        }
        self.observe_autosave(window, cx);
        self.record_history();
        window.set_window_title("Glim");
        if self.root.is_none() {
            self.watch_file_parent(window, cx);
        }
    }
    pub(super) fn open_commit(
        &mut self,
        root: PathBuf,
        oid: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = root.join(format!("Commit {oid}"));
        if let Some(index) = self
            .tabs
            .iter()
            .position(|r| r.historical && r.path == path)
        {
            self.activate_tab(index, window, cx);
            return;
        }
        self.loading = true;
        let work = cx.background_executor().spawn(async move {
            glim_services::git::management::commit_details(&root, &oid)
                .map(|text| glim_services::binary::BytePreview { path, text })
        });
        self.load_task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = work.await;
            let _ = view.update_in(cx, |v, window, cx| {
                v.loading = false;
                match result {
                    Ok(document) => v.install_content(Content::Commit(document), window, cx),
                    Err(e) => v.error = Some(format!("Commit: {e:#}").into()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    pub(super) fn open_diff(
        &mut self,
        root: PathBuf,
        change: GitChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_anchor = None;
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
    pub(super) fn choose_add_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some("Add folders to workspace".into()),
        });
        self.picker_task = Some(cx.spawn_in(window, async move |view, cx| {
            let paths = match selection.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return,
                other => {
                    let _ = view.update(cx, |v, cx| {
                        v.error = Some(format!("Cannot choose folders: {other:?}").into());
                        cx.notify();
                    });
                    return;
                }
            };
            for path in paths {
                let folder = cx
                    .background_executor()
                    .spawn(async move { open_folder(&path) })
                    .await;
                let _ = view.update_in(cx, |v, window, cx| {
                    match folder {
                        Ok(folder) => v.add_folder(folder, window, cx),
                        Err(error) => v.error = Some(format!("{error:#}").into()),
                    }
                    cx.notify();
                });
            }
        }));
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
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("safetensors"))
    {
        return glim_services::safetensors::read_preview(path).map(Content::Bytes);
    }
    let result = if glim_services::media::supports(path) {
        glim_services::media::read_image(path).map(Content::Image)
    } else {
        (|| {
            let first = glim_services::paged::read_page(path, 0, false)?;
            if first.total > glim_services::files::MAX_DOCUMENT_BYTES
                || first.text.lines().any(|line| line.len() > 16 * 1024)
            {
                Ok(Content::Page(first))
            } else {
                read_document(path).map(|d| {
                    if d.text.lines().any(|line| line.len() > 16 * 1024)
                        || d.text.bytes().filter(|b| *b == b'\n').take(100_001).count() > 100_000
                    {
                        Content::Page(first)
                    } else {
                        Content::File(d)
                    }
                })
            }
        })()
    };
    if result.is_err()
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
    {
        glim_services::binary::read_preview(path).map(Content::Bytes)
    } else {
        result
    }
}

#[cfg(test)]
mod format_tests {
    use super::{Content, read_content};
    #[test]
    fn dotfiles_are_routed_by_content_not_missing_extension() {
        let dir = tempfile::tempdir().unwrap();
        for name in [".env", ".npmrc", ".gitignore", ".config.local"] {
            let path = dir.path().join(name);
            std::fs::write(&path, "EXAMPLE=value\n").unwrap();
            assert!(matches!(read_content(&path).unwrap(), Content::File(_)));
        }
        let binary = dir.path().join(".DS_Store");
        std::fs::write(&binary, [0, 1, 2, 255]).unwrap();
        assert!(matches!(read_content(&binary).unwrap(), Content::Bytes(_)));
    }
}
