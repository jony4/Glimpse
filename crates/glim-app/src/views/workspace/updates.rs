use super::{Workspace, loading::Content};
use glim_services::{git::read_diff, watch::WorkspaceWatch, workspace::repositories_for_roots};
use gpui_kit::*;
use std::time::Duration;

impl Workspace {
    pub(super) fn start_watching(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.watch_task = Some(cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                if view
                    .update_in(cx, |view, window, cx| {
                        if !view.loading
                            && view.refresh_task.is_none()
                            && !view.changes.as_ref().is_some_and(|c| c.read(cx).is_busy())
                            && view
                                .watch
                                .as_ref()
                                .is_some_and(WorkspaceWatch::take_changed)
                        {
                            view.refresh_with_scan_notice(false, window, cx);
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }
    pub(super) fn watch_file_parent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parents = self
            .tabs
            .iter()
            .filter_map(|r| r.path.parent().map(ToOwned::to_owned))
            .collect::<std::collections::BTreeSet<_>>();
        let work = cx.background_executor().spawn(async move {
            let mut parents = parents.into_iter();
            let parent = parents
                .next()
                .ok_or_else(|| anyhow::anyhow!("No open files"))?;
            let mut watch = WorkspaceWatch::new(&parent)?;
            for parent in parents {
                watch.add_path(&parent)?;
            }
            Ok::<_, anyhow::Error>(watch)
        });
        self.watch_setup = Some(cx.spawn_in(window, async move |view, cx| {
            let result = work.await;
            let _ = view.update_in(cx, |view, window, cx| {
                if let Ok(watch) = result {
                    view.watch = Some(watch);
                    view.start_watching(window, cx);
                }
            });
        }));
    }
    pub(super) fn report_scan_warning(&mut self, warning: Option<String>, show_notice: bool) {
        if let Some(warning) = warning
            && show_notice
            && self.last_scan_warning.as_ref() != Some(&warning)
            && self.error.is_none()
        {
            // Discovery warnings never replace failures from explicit operations.
            self.error = Some(warning.clone().into());
            self.last_scan_warning = Some(warning);
        }
    }

    pub(super) fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_with_scan_notice(true, window, cx);
    }

    fn refresh_with_scan_notice(
        &mut self,
        show_notice: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.refresh_task.is_some() || self.repository_task.is_some() || self.is_saving() {
            return;
        }
        let roots = self.roots.clone();
        let targets = self
            .tabs
            .iter()
            .filter(|r| !r.historical && r.snapshot != "native-preview")
            .map(|r| (r.path.clone(), r.diff.clone(), r.repository_root.clone()))
            .collect::<Vec<_>>();
        if let Some(explorer) = &self.explorer {
            explorer.update(cx, |v, cx| v.refresh(cx));
        }
        let read = cx.background_executor().spawn(async move {
            let repos = repositories_for_roots(&roots);
            let contents = targets
                .into_iter()
                .map(|(path, diff, root)| {
                    let scope = diff.as_ref().map(|d| d.scope);
                    let content = if let (Some(change), Some(root)) = (diff, root) {
                        read_diff(&root, &change).map(|d| Content::Diff(d, root))
                    } else {
                        super::loading::read_content(&path)
                    };
                    (
                        path.clone(),
                        scope,
                        content.unwrap_or_else(|e| Content::Unavailable(path, format!("{e:#}"))),
                    )
                })
                .collect::<Vec<_>>();
            (repos, contents)
        });
        self.refresh_task = Some(cx.spawn_in(window, async move |view, cx| {
            let (repos, contents) = read.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.refresh_task = None;
                view.report_scan_warning(repos.warning(), show_notice);
                if let Some(changes) = &view.changes {
                    changes.update(cx, |v, cx| v.set_repositories(repos.repositories, cx));
                }
                for (path, scope, content) in contents {
                    let Some(index) = view.tabs.iter().position(|r| {
                        !r.historical && r.path == path && r.diff.as_ref().map(|d| d.scope) == scope
                    }) else {
                        continue;
                    };
                    // Never replace a draft, even if this refresh started before typing.
                    if view.tabs[index].is_dirty() || view.is_saving() {
                        continue;
                    }
                    let unchanged = match &content {
                        Content::File(d) => view.tabs[index].snapshot == d.text,
                        Content::Image(d) => view.tabs[index].snapshot == d.fingerprint,
                        Content::Diff(d, _) => view.tabs[index].snapshot == d.patch,
                        Content::Bytes(d) | Content::Commit(d, _) => {
                            view.tabs[index].snapshot == d.text
                        }
                        Content::Page(d) => view.tabs[index].snapshot == d.fingerprint(),
                        Content::Native(..) => view.tabs[index].snapshot == "native-preview",
                        Content::Unavailable(_, _) => false,
                    };
                    if unchanged && !view.tabs[index].is_unavailable() {
                        continue;
                    }
                    let mut reader = Self::make_reader(content, window, cx);
                    reader.inherit_view(&view.tabs[index], cx);
                    reader.set_word_wrap(view.word_wrap, window, cx);
                    reader.set_editing_locked(view.git_edit_locked, cx);
                    // Keep diff identity even when a file disappears during refresh.
                    reader.diff = view.tabs[index].diff.clone();
                    reader.repository_root = view.tabs[index].repository_root.clone();
                    view.tabs[index] = reader;
                }
                view.observe_autosave(window, cx);
                cx.notify();
            });
        }));
    }
}
