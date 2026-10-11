use super::*;
use glim_core::{GitOperation, GitRequest, GitSnapshot, GraphRow};
use glim_services::git::management;
use gpui_kit::component::WindowExt;

#[derive(Clone)]
enum GraphDisplayRow {
    Commit(usize),
    File(usize, String, management::CommitFile),
    Loading(usize),
    Empty(usize),
}

#[derive(Clone)]
pub(super) enum BranchForm {
    Create(Option<String>),
    Rename,
    Track(String),
}
pub(super) struct GitState {
    pub snapshot: Option<GitSnapshot>,
    pub snapshot_task: Option<Task<()>>,
    pub picker: bool,
    pub form: Option<BranchForm>,
    pub input: Entity<InputState>,
    pub _input_subscription: Subscription,
    pub confirm_task: Option<Task<()>>,
    pub operation: Option<(PathBuf, &'static str)>,
    pub graph_collapsed: bool,
    pub graph_all: bool,
    pub graph_limit: usize,
    graph_more: bool,
    graph_refresh_pending: bool,
    pub graph_rows: Vec<GraphRow>,
    expanded_commits: HashSet<String>,
    commit_files: HashMap<String, Vec<management::CommitFile>>,
    file_tasks: HashMap<String, Task<()>>,
    pub selected_commit: Option<String>,
    pub graph_task: Option<Task<()>>,
    pub graph_scroll: UniformListScrollHandle,
}
impl GitState {
    pub fn new(window: &mut Window, cx: &mut Context<Changes>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Branch name / filter"));
        let subscription = cx.subscribe_in(&input, window, |v, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) && v.git.form.is_some() {
                v.submit_branch(window, cx);
            }
            cx.notify();
        });
        Self {
            snapshot: None,
            snapshot_task: None,
            picker: false,
            form: None,
            input,
            _input_subscription: subscription,
            confirm_task: None,
            operation: None,
            graph_collapsed: true,
            graph_all: true,
            graph_limit: 200,
            graph_more: false,
            graph_refresh_pending: false,
            graph_rows: Vec::new(),
            expanded_commits: HashSet::new(),
            commit_files: HashMap::new(),
            file_tasks: HashMap::new(),
            selected_commit: None,
            graph_task: None,
            graph_scroll: UniformListScrollHandle::new(),
        }
    }
}
struct BranchDialog {
    owner: WeakEntity<Changes>,
    _subscription: Subscription,
}
impl Render for BranchDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.owner
            .update(cx, |view, cx| view.git_controls(cx))
            .unwrap_or_else(|_| div().into_any_element())
    }
}

impl Changes {
    pub(super) fn reload_git(&mut self, cx: &mut Context<Self>) {
        self.git.snapshot_task = None;
        let Some(root) = self.repository().map(|r| r.root.clone()) else {
            self.git.snapshot = None;
            self.git.graph_rows.clear();
            return;
        };
        let work = cx
            .background_executor()
            .spawn(async move { management::snapshot(&root) });
        self.git.snapshot_task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |v, cx| {
                v.git.snapshot_task = None;
                match result {
                    Ok(snapshot) => {
                        let history_changed = v.git.snapshot.as_ref().is_none_or(|previous| {
                            previous.graph_revision != snapshot.graph_revision
                        });
                        v.git.snapshot = Some(snapshot);
                        if history_changed && !v.git.graph_collapsed {
                            v.reload_graph(cx);
                        }
                    }
                    Err(error) => {
                        v.git.snapshot = None;
                        cx.emit(ChangeSelected::Error(format!("Git: {error:#}")));
                    }
                }
                cx.notify();
            });
        }));
    }
    pub(super) fn reset_git(&mut self, cx: &mut Context<Self>) {
        self.git.snapshot = None;
        self.git.picker = false;
        self.git.form = None;
        self.git.graph_rows.clear();
        self.git.expanded_commits.clear();
        self.git.commit_files.clear();
        self.git.file_tasks.clear();
        self.git.selected_commit = None;
        self.git.graph_task = None;
        self.git.graph_limit = 200;
        self.git.graph_more = false;
        self.git.graph_refresh_pending = false;
        self.git.graph_scroll = UniformListScrollHandle::new();
        self.reload_git(cx);
    }
    pub(super) fn request_operation(
        &mut self,
        operation: GitOperation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_busy()
            || self.loading
            || self.git.confirm_task.is_some()
            || self.git.snapshot_task.is_some()
        {
            return;
        }
        let Some(root) = self.repository().map(|r| r.root.clone()) else {
            return;
        };
        let Some(snapshot) = &self.git.snapshot else {
            return;
        };
        let request = GitRequest {
            expected_head: snapshot.head.clone(),
            expected_branch: snapshot.branch.clone(),
            operation,
        };
        if let Some(explanation) = request.operation.confirmation() {
            let detail = format!(
                "{}\n\nRepository: {}\nCurrent branch: {}\nTarget: {}",
                explanation,
                root.display(),
                snapshot.branch.as_deref().unwrap_or("Detached HEAD"),
                request.operation.target().unwrap_or("Current repository")
            );
            let answer = window.prompt(
                PromptLevel::Warning,
                request.operation.label(),
                Some(&detail),
                &["Cancel", "Continue"],
                cx,
            );
            self.git.confirm_task = Some(cx.spawn_in(window, async move |view, cx| {
                let accepted = answer.await == Ok(1);
                let _ = view.update(cx, |v, cx| {
                    v.git.confirm_task = None;
                    if accepted && v.repository().is_some_and(|r| r.root == root) {
                        cx.emit(ChangeSelected::Git(root, request));
                    }
                    cx.notify();
                });
            }));
        } else {
            cx.emit(ChangeSelected::Git(root, request));
        }
    }
    pub fn run_operation(&mut self, root: PathBuf, request: GitRequest, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.notice = None;
        self.git.operation = Some((root.clone(), request.operation.label()));
        self.git.snapshot_task = None;
        self.git.graph_task = None;
        let label = request.operation.label();
        let work = cx
            .background_executor()
            .spawn(async move { management::execute(&root, &request) });
        self.task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |v, cx| {
                v.busy = false;
                v.git.operation = None;
                match result {
                    Ok(()) => {
                        v.notice = Some(format!("{label} completed.").into());
                        v.git.form = None;
                        v.git.picker = false;
                    }
                    Err(error) => cx.emit(ChangeSelected::Error(format!("{label}: {error:#}"))),
                }
                v.reload_git(cx);
                cx.emit(ChangeSelected::GitFinished);
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn open_branch_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.git.picker {
            return;
        }
        self.git.picker = true;
        self.git.form = None;
        self.git
            .input
            .update(cx, |s, cx| s.set_value("", window, cx));
        let owner = cx.entity();
        let weak = owner.downgrade();
        let content = cx.new(|cx| {
            let subscription = cx.observe_in(&owner, window, |_, owner, window, cx| {
                if !owner.read(cx).git.picker {
                    window.close_dialog(cx);
                }
                cx.notify();
            });
            BranchDialog {
                owner: weak.clone(),
                _subscription: subscription,
            }
        });
        let title = self
            .repository()
            .and_then(|r| r.root.file_name())
            .map(|name| format!("Branches · {}", name.to_string_lossy()))
            .unwrap_or_else(|| "Branches".into());
        window.open_dialog(cx, move |dialog, _, _| {
            let weak = weak.clone();
            dialog
                .title(title.clone())
                .width(px(520.))
                .child(content.clone())
                .on_close(move |_, _, cx| {
                    let _ = weak.update(cx, |v, cx| {
                        v.git.picker = false;
                        v.git.form = None;
                        cx.notify();
                    });
                })
        });
        let input = self.git.input.clone();
        window.defer(cx, move |window, cx| {
            input.update(cx, |s, cx| s.focus(window, cx))
        });
        cx.notify();
    }

    fn branch_form(
        &mut self,
        form: BranchForm,
        value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.git.form = Some(form);
        self.git.picker = true;
        self.git.input.update(cx, |s, cx| {
            s.set_value(value, window, cx);
            s.focus(window, cx);
        });
        cx.notify();
    }
    fn submit_branch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.git.input.read(cx).value().trim().to_owned();
        if name.is_empty() {
            return;
        }
        let operation = match self.git.form.clone() {
            Some(BranchForm::Create(start)) => GitOperation::Create { name, start },
            Some(BranchForm::Rename) => GitOperation::Rename(name),
            Some(BranchForm::Track(reference)) => GitOperation::Track { reference, name },
            None => return,
        };
        self.request_operation(operation, window, cx);
    }
    pub(super) fn repository_actions(&self, cx: &mut Context<Self>) -> AnyElement {
        use gpui_kit::assets::IconName;
        use gpui_kit::component::menu::DropdownMenu;
        let blocked = self.is_busy()
            || self.loading
            || self.git.snapshot_task.is_some()
            || self.git.snapshot.is_none()
            || self.git.confirm_task.is_some();
        let state = self.git.snapshot.clone().unwrap_or_default();
        let root = self.repository().map(|r| r.root.clone());
        let view = cx.entity().downgrade();
        h_flex()
            .gap_1()
            .flex_shrink_0()
            .child(
                Button::new("repo-branches")
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(IconName::GitBranch))
                    .tooltip("Branches…")
                    .accessibility_label("Branches")
                    .disabled(blocked)
                    .on_click(cx.listener(|v, _, w, cx| {
                        cx.stop_propagation();
                        v.open_branch_dialog(w, cx);
                    })),
            )
            .child(
                Button::new("repo-sync")
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(IconName::RefreshCw))
                    .tooltip(format!("Sync Changes · ↓{} ↑{}", state.behind, state.ahead))
                    .accessibility_label("Sync Changes")
                    .disabled(blocked || state.upstream.is_none())
                    .on_click(cx.listener(|v, _, w, cx| {
                        cx.stop_propagation();
                        v.request_operation(GitOperation::Sync, w, cx);
                    })),
            )
            .child(
                Button::new("repo-more")
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(IconName::Ellipsis))
                    .tooltip("Repository actions…")
                    .accessibility_label("Repository actions")
                    .disabled(blocked)
                    .dropdown_menu(move |menu, _, cx| {
                        view.update(cx, |v, cx| {
                            v.repository_menu(menu, root.clone().expect("Selected repository"), cx)
                        })
                        .expect("Repository menu owner is alive")
                    }),
            )
            .into_any_element()
    }

    pub(super) fn repository_menu(
        &self,
        mut menu: gpui_kit::component::menu::PopupMenu,
        target: PathBuf,
        cx: &mut Context<Self>,
    ) -> gpui_kit::component::menu::PopupMenu {
        use gpui_kit::component::menu::PopupMenuItem;
        let selected = self.repository().is_some_and(|r| r.root == target);
        let known = selected && self.git.snapshot.is_some();
        let state = if known {
            self.git.snapshot.clone().unwrap_or_default()
        } else {
            GitSnapshot::default()
        };
        let blocked = self.is_busy() || self.loading || self.git.confirm_task.is_some();
        let root = Some(target.clone());
        let view = cx.entity().downgrade();
        let branch_view = view.clone();
        menu = menu.item(PopupMenuItem::new("Branches…").disabled(blocked).on_click(
            move |_, w, cx| {
                let _ = branch_view.update(cx, |v, cx| {
                    if let Some(index) = v.repositories.iter().position(|r| r.root == target) {
                        if v.is_busy() {
                            return;
                        }
                        v.select_repository(index, w, cx);
                        v.open_branch_dialog(w, cx);
                    }
                });
            },
        ));
        let mut actions = vec![
            (
                "Fetch".to_string(),
                GitOperation::Fetch,
                known && state.remotes.is_empty(),
            ),
            (
                "Pull".to_string(),
                GitOperation::Pull,
                known && state.upstream.is_none(),
            ),
            (
                "Push".to_string(),
                GitOperation::Push,
                known && state.upstream.is_none(),
            ),
            ("Stash…".to_string(), GitOperation::Stash, false),
        ];
        if state.upstream.is_none() {
            for remote in &state.remotes {
                actions.push((
                    format!("Publish Branch → {remote}"),
                    GitOperation::Publish(remote.clone()),
                    state.branch.is_none() || state.head.is_none(),
                ));
            }
        }
        if state.merging {
            actions.push(("Abort Merge…".into(), GitOperation::AbortMerge, false));
        }
        for (oid, subject) in &state.stashes {
            actions.push((
                format!("Apply Stash · {subject}"),
                GitOperation::ApplyStash(oid.clone()),
                false,
            ));
        }
        actions.push((
            "Sync Changes".into(),
            GitOperation::Sync,
            known && state.upstream.is_none(),
        ));
        for (label, operation, disabled) in actions {
            let view = view.clone();
            let root = root.clone();
            menu = menu.item(
                PopupMenuItem::new(label)
                    .disabled(blocked || disabled)
                    .on_click(move |_, w, cx| {
                        let _ = view.update(cx, |v, cx| {
                            if let Some(root) = &root {
                                v.request_repository_operation(
                                    root.clone(),
                                    operation.clone(),
                                    w,
                                    cx,
                                );
                            }
                        });
                    }),
            );
        }
        for (label, all) in [
            ("Graph: All Branches", true),
            ("Graph: Current Branch", false),
        ] {
            let view = view.clone();
            let root = root.clone();
            menu = menu.item(PopupMenuItem::new(label).disabled(blocked).on_click(
                move |_, w, cx| {
                    let _ = view.update(cx, |v, cx| {
                        if v.is_busy() {
                            return;
                        }
                        let Some(index) = v
                            .repositories
                            .iter()
                            .position(|r| Some(&r.root) == root.as_ref())
                        else {
                            return;
                        };
                        v.select_repository(index, w, cx);
                        v.git.graph_all = all;
                        v.git.graph_limit = 200;
                        v.git.graph_more = false;
                        v.git.graph_rows.clear();
                        v.git.graph_scroll = UniformListScrollHandle::new();
                        v.reload_graph(cx);
                        cx.notify();
                    });
                },
            ));
        }
        menu
    }

    fn request_repository_operation(
        &mut self,
        root: PathBuf,
        operation: GitOperation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_busy() || self.git.confirm_task.is_some() {
            return;
        }
        let Some(index) = self.repositories.iter().position(|r| r.root == root) else {
            return;
        };
        self.select_repository(index, window, cx);
        if self.git.snapshot.is_some() && self.git.snapshot_task.is_none() {
            self.request_operation(operation, window, cx);
            return;
        }
        let query_root = root.clone();
        let work = cx
            .background_executor()
            .spawn(async move { management::snapshot(&query_root) });
        self.git.snapshot_task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = work.await;
            let _ = view.update_in(cx, |v, window, cx| {
                if v.repository().map(|r| &r.root) != Some(&root) {
                    return;
                }
                v.git.snapshot_task = None;
                match result {
                    Ok(snapshot) => {
                        v.git.snapshot = Some(snapshot);
                        v.request_operation(operation, window, cx);
                    }
                    Err(error) => cx.emit(ChangeSelected::Error(format!("Git: {error:#}"))),
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn git_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let blocked = self.is_busy()
            || self.loading
            || self.git.snapshot_task.is_some()
            || self.git.snapshot.is_none()
            || self.git.confirm_task.is_some();
        let state = self.git.snapshot.clone().unwrap_or_default();
        let query = self.git.input.read(cx).value().to_lowercase();
        let controls = v_flex()
            .id("git-controls")
            .max_h(px(320.))
            .overflow_y_scroll()
            .w_full()
            .flex_shrink_0()
            .px_2()
            .gap_1()
            .when(self.git.picker, |panel| {
                panel
                    .child(Input::new(&self.git.input).small().disabled(blocked))
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("git-create")
                                    .ghost()
                                    .xsmall()
                                    .label("Create Branch…")
                                    .disabled(blocked)
                                    .on_click(cx.listener(|v, _, w, cx| {
                                        v.branch_form(
                                            BranchForm::Create(None),
                                            String::new(),
                                            w,
                                            cx,
                                        )
                                    })),
                            )
                            .child(
                                Button::new("git-rename")
                                    .ghost()
                                    .xsmall()
                                    .label("Rename…")
                                    .disabled(blocked || state.branch.is_none())
                                    .on_click(cx.listener(|v, _, w, cx| {
                                        let name = v
                                            .git
                                            .snapshot
                                            .as_ref()
                                            .and_then(|s| s.branch.clone())
                                            .unwrap_or_default();
                                        v.branch_form(BranchForm::Rename, name, w, cx);
                                    })),
                            ),
                    )
                    .when_some(self.git.form.as_ref(), |panel, form| {
                        let label = match form {
                            BranchForm::Create(_) => "Create & Checkout",
                            BranchForm::Rename => "Rename Branch",
                            BranchForm::Track(_) => "Checkout Tracking Branch",
                        };
                        panel.child(
                            Button::new("git-submit-branch")
                                .ghost()
                                .small()
                                .label(label)
                                .disabled(blocked || query.trim().is_empty())
                                .on_click(cx.listener(|v, _, w, cx| v.submit_branch(w, cx))),
                        )
                    })
                    .when(self.git.form.is_none(), |panel| {
                        panel.child(
                            v_flex()
                                .id("git-branches")
                                .max_h(px(180.))
                                .overflow_y_scroll()
                                .children(
                                    state
                                        .branches
                                        .iter()
                                        .filter(|b| b.name.to_lowercase().contains(&query))
                                        .enumerate()
                                        .map(|(i, b)| {
                                            let branch = b.clone();
                                            let menu_branch = b.clone();
                                            let view = cx.entity().downgrade();
                                            Button::new(("git-checkout", i))
                                                .ghost()
                                                .small()
                                                .w_full()
                                                .justify_start()
                                                .label(format!(
                                                    "{}{}{}",
                                                    if b.current { "✓ " } else { "" },
                                                    if b.remote { "Remote · " } else { "" },
                                                    b.name
                                                ))
                                                .tooltip(format!(
                                                    "{}{}",
                                                    b.reference,
                                                    if b.upstream.is_empty() {
                                                        String::new()
                                                    } else {
                                                        format!(" → {}", b.upstream)
                                                    }
                                                ))
                                                .disabled(blocked || b.current)
                                                .on_click(cx.listener(move |v, _, w, cx| {
                                                    if branch.remote {
                                                        let name = branch
                                                            .name
                                                            .split_once('/')
                                                            .map_or(
                                                                branch.name.as_str(),
                                                                |(_, n)| n,
                                                            )
                                                            .to_owned();
                                                        v.branch_form(
                                                            BranchForm::Track(
                                                                branch.reference.clone(),
                                                            ),
                                                            name,
                                                            w,
                                                            cx,
                                                        );
                                                    } else {
                                                        v.request_operation(
                                                            GitOperation::Switch(
                                                                branch.name.clone(),
                                                            ),
                                                            w,
                                                            cx,
                                                        );
                                                    }
                                                }))
                                                .context_menu(move |menu, _, _| {
                                                    use gpui_kit::component::menu::PopupMenuItem;
                                                    let merge_view = view.clone();
                                                    let reference = menu_branch.reference.clone();
                                                    let create_view = view.clone();
                                                    let start = menu_branch.reference.clone();
                                                    let delete_view = view.clone();
                                                    let name = menu_branch.name.clone();
                                                    menu.item(
                                                        PopupMenuItem::new("Create Branch From…")
                                                            .disabled(blocked)
                                                            .on_click(move |_, w, cx| {
                                                                let _ = create_view.update(
                                                                    cx,
                                                                    |v, cx| {
                                                                        v.branch_form(
                                                                            BranchForm::Create(
                                                                                Some(start.clone()),
                                                                            ),
                                                                            String::new(),
                                                                            w,
                                                                            cx,
                                                                        )
                                                                    },
                                                                );
                                                            }),
                                                    )
                                                    .item(
                                                        PopupMenuItem::new(
                                                            "Merge into Current Branch…",
                                                        )
                                                        .disabled(blocked || menu_branch.current)
                                                        .on_click(move |_, w, cx| {
                                                            let _ =
                                                                merge_view.update(cx, |v, cx| {
                                                                    v.request_operation(
                                                                        GitOperation::Merge(
                                                                            reference.clone(),
                                                                        ),
                                                                        w,
                                                                        cx,
                                                                    )
                                                                });
                                                        }),
                                                    )
                                                    .item(
                                                        PopupMenuItem::new("Delete Branch…")
                                                            .disabled(
                                                                blocked
                                                                    || menu_branch.remote
                                                                    || menu_branch.current,
                                                            )
                                                            .on_click(move |_, w, cx| {
                                                                let _ = delete_view.update(
                                                                    cx,
                                                                    |v, cx| {
                                                                        v.request_operation(
                                                                            GitOperation::Delete(
                                                                                name.clone(),
                                                                            ),
                                                                            w,
                                                                            cx,
                                                                        )
                                                                    },
                                                                );
                                                            }),
                                                    )
                                                })
                                        }),
                                ),
                        )
                    })
            });
        controls.into_any_element()
    }
    pub(super) fn reload_graph(&mut self, cx: &mut Context<Self>) {
        if self.git.graph_collapsed {
            return;
        }
        if self.git.graph_task.is_some() {
            self.git.graph_refresh_pending = true;
            return;
        }
        self.git.graph_refresh_pending = false;
        let Some(root) = self.repository().map(|r| r.root.clone()) else {
            return;
        };
        self.git.graph_more = false;
        let limit = self.git.graph_limit;
        let all = self.git.graph_all;
        let work = cx
            .background_executor()
            .spawn(async move { management::graph(&root, limit, all) });
        self.git.graph_task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |v, cx| {
                v.git.graph_task = None;
                if v.git.graph_all != all || v.git.graph_limit != limit {
                    v.reload_graph(cx);
                    return;
                }
                match result {
                    Ok(rows) => {
                        v.git.graph_more = limit < 1000
                            && rows.iter().filter(|row| row.commit.is_some()).count() >= limit;
                        if v.git.graph_rows != rows {
                            v.git.graph_rows = rows;
                        }
                    }
                    Err(e) => cx.emit(ChangeSelected::Error(format!("Graph: {e:#}"))),
                }
                if v.git.graph_refresh_pending {
                    v.reload_graph(cx);
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn toggle_commit(&mut self, oid: String, cx: &mut Context<Self>) {
        self.git.selected_commit = Some(oid.clone());
        if self.git.expanded_commits.remove(&oid) {
            cx.notify();
            return;
        }
        self.git.expanded_commits.insert(oid.clone());
        if !self.git.commit_files.contains_key(&oid) && !self.git.file_tasks.contains_key(&oid) {
            let Some(root) = self.repository().map(|r| r.root.clone()) else {
                return;
            };
            let task_oid = oid.clone();
            let task_root = root.clone();
            let work = cx
                .background_executor()
                .spawn(async move { management::commit_files(&task_root, &task_oid) });
            let key = oid.clone();
            let task = cx.spawn(async move |view, cx| {
                let result = work.await;
                let _ = view.update(cx, |v, cx| {
                    if v.repository().map(|r| &r.root) != Some(&root) {
                        return;
                    }
                    v.git.file_tasks.remove(&oid);
                    match result {
                        Ok(files) => {
                            v.git.commit_files.insert(oid.clone(), files);
                        }
                        Err(error) => {
                            v.git.expanded_commits.remove(&oid);
                            cx.emit(ChangeSelected::Error(format!("Commit files: {error:#}")));
                        }
                    }
                    cx.notify();
                });
            });
            self.git.file_tasks.insert(key, task);
        }
        cx.notify();
    }

    fn graph_display_rows(&self) -> Vec<GraphDisplayRow> {
        let mut rows = Vec::new();
        for (index, row) in self.git.graph_rows.iter().enumerate() {
            rows.push(GraphDisplayRow::Commit(index));
            if let Some(oid) = row
                .commit
                .as_ref()
                .filter(|oid| self.git.expanded_commits.contains(*oid))
            {
                match self.git.commit_files.get(oid) {
                    Some(files) if files.is_empty() => rows.push(GraphDisplayRow::Empty(index)),
                    Some(files) => rows.extend(
                        files
                            .iter()
                            .cloned()
                            .map(|file| GraphDisplayRow::File(index, oid.clone(), file)),
                    ),
                    None => rows.push(GraphDisplayRow::Loading(index)),
                }
            }
        }
        rows
    }

    fn show_commit(&mut self, oid: String, cx: &mut Context<Self>) {
        self.git.selected_commit = Some(oid.clone());
        cx.notify();
        if let Some(repo) = self.repository() {
            cx.emit(ChangeSelected::Commit(repo.root.clone(), oid));
        }
    }
    pub(super) fn graph_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let display_rows = self.graph_display_rows();
        let lane_width = self
            .git
            .graph_rows
            .iter()
            .map(|r| r.layout.width)
            .max()
            .unwrap_or(1);
        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .child(
                h_flex()
                    .h(px(36.))
                    .flex_shrink_0()
                    .px_2()
                    .gap_1()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .id("git-graph-toggle")
                            .role(Role::Button)
                            .aria_label(if self.git.graph_collapsed {
                                "Expand graph"
                            } else {
                                "Collapse graph"
                            })
                            .flex_1()
                            .h_full()
                            .gap_1()
                            .text_size(px(15.))
                            .font_weight(FontWeight::BOLD)
                            .cursor_pointer()
                            .child(disclosure(self.git.graph_collapsed, cx))
                            .child("Graph")
                            .on_click(cx.listener(|v, _, w, cx| v.toggle_section(2, w, cx))),
                    )
                    .when(!self.git.graph_collapsed, |bar| {
                        bar.child(
                            Button::new("git-graph-refresh")
                                .ghost()
                                .xsmall()
                                .label("↻")
                                .tooltip("Refresh Graph")
                                .disabled(self.git.graph_task.is_some())
                                .on_click(cx.listener(|v, _, _, cx| v.reload_graph(cx))),
                        )
                    }),
            )
            .when(!self.git.graph_collapsed, |panel| {
                panel
                    .when(
                        self.git.graph_task.is_some() && self.git.graph_rows.is_empty(),
                        |panel| panel.child(div().px_3().text_xs().child("Loading…")),
                    )
                    .when(
                        self.git.graph_rows.is_empty() && self.git.graph_task.is_none(),
                        |panel| panel.child(div().px_3().text_sm().child("No commits yet.")),
                    )
                    .child(
                        uniform_list(
                            "git-graph-rows",
                            display_rows.len(),
                            cx.processor(move |v, range: std::ops::Range<usize>, _, cx| {
                                if v.git.graph_more
                                    && v.git.graph_task.is_none()
                                    && range.end.saturating_add(8) >= display_rows.len()
                                {
                                    // Queue after painting; never run Git from the row renderer.
                                    v.git.graph_more = false;
                                    let view = cx.entity().downgrade();
                                    let root = v.repository().map(|r| r.root.clone());
                                    let scope = v.git.graph_all;
                                    cx.defer(move |cx| {
                                        let _ = view.update(cx, |v, cx| {
                                            if !v.git.graph_collapsed
                                                && v.git.graph_task.is_none()
                                                && v.repository().map(|r| &r.root) == root.as_ref()
                                                && v.git.graph_all == scope
                                            {
                                                v.git.graph_limit =
                                                    (v.git.graph_limit + 200).min(1000);
                                                v.reload_graph(cx);
                                            }
                                        });
                                    });
                                }
                                range
                                    .map(|i| {
                                        let index = match &display_rows[i] {
                                            GraphDisplayRow::Commit(index) => *index,
                                            GraphDisplayRow::File(index, oid, file) => {
                                                let oid = oid.clone();
                                                let path = file.path.clone();
                                                let name =
                                                    if let Some(previous) = &file.previous_path {
                                                        format!(
                                                            "{} → {}",
                                                            previous.display(),
                                                            file.path.display()
                                                        )
                                                    } else {
                                                        file.path.display().to_string()
                                                    };
                                                return h_flex()
                                                    .id(("commit-file", i))
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(move |v, _, _, cx| {
                                                        if let Some(repo) = v.repository() {
                                                            cx.emit(ChangeSelected::CommitFile(
                                                                repo.root.clone(),
                                                                oid.clone(),
                                                                path.clone(),
                                                            ));
                                                        }
                                                    }))
                                                    .h(px(40.))
                                                    .px_2()
                                                    .gap_2()
                                                    .child(div().w(px(14.)).flex_shrink_0())
                                                    .child(graph_lane(
                                                        &v.git.graph_rows[*index].layout,
                                                        true,
                                                        lane_width,
                                                        cx,
                                                    ))
                                                    .child(file_icon(&file.path))
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .truncate()
                                                            .text_sm()
                                                            .child(name),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(file.status.to_string()),
                                                    )
                                                    .into_any_element();
                                            }
                                            GraphDisplayRow::Loading(index)
                                            | GraphDisplayRow::Empty(index) => {
                                                return h_flex()
                                                    .h(px(40.))
                                                    .px_2()
                                                    .gap_2()
                                                    .child(div().w(px(14.)).flex_shrink_0())
                                                    .child(graph_lane(
                                                        &v.git.graph_rows[*index].layout,
                                                        true,
                                                        lane_width,
                                                        cx,
                                                    ))
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(
                                                                if matches!(
                                                                    &display_rows[i],
                                                                    GraphDisplayRow::Loading(_)
                                                                ) {
                                                                    "Loading files…"
                                                                } else {
                                                                    "No changed files"
                                                                },
                                                            ),
                                                    )
                                                    .into_any_element();
                                            }
                                        };
                                        let row = &v.git.graph_rows[index];
                                        let oid = row.commit.clone();
                                        let copy = row.commit.clone();
                                        h_flex()
                                            .id(("git-graph-row", i))
                                            .h(px(40.))
                                            .px_2()
                                            .gap_2()
                                            .w_full()
                                            .min_w_0()
                                            .cursor_pointer()
                                            .when(
                                                row.commit.is_some()
                                                    && row.commit == v.git.selected_commit,
                                                |row| row.bg(cx.theme().foreground.opacity(0.06)),
                                            )
                                            .child(div().w(px(14.)).flex_shrink_0().when_some(
                                                row.commit.as_ref(),
                                                |item, oid| {
                                                    item.child(disclosure(
                                                        !v.git.expanded_commits.contains(oid),
                                                        cx,
                                                    ))
                                                },
                                            ))
                                            .child(graph_lane(&row.layout, false, lane_width, cx))
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .child(div().truncate().text_sm().child(
                                                        format!(
                                                            "{} {}",
                                                            row.subject, row.references
                                                        ),
                                                    ))
                                                    .child(
                                                        div()
                                                            .truncate()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(format!(
                                                                "{} · {}",
                                                                row.author, row.age
                                                            )),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(row.short_id.clone()),
                                            )
                                            .on_click(cx.listener(
                                                move |v, event: &ClickEvent, _, cx| {
                                                    if let Some(oid) = &oid {
                                                        if event.click_count() == 2 {
                                                            v.show_commit(oid.clone(), cx);
                                                        } else {
                                                            v.toggle_commit(oid.clone(), cx);
                                                        }
                                                    }
                                                },
                                            ))
                                            .context_menu(move |menu, _, _| {
                                                let copy = copy.clone();
                                                menu.item(
                                                    gpui_kit::component::menu::PopupMenuItem::new(
                                                        "Copy Commit ID",
                                                    )
                                                    .disabled(copy.is_none())
                                                    .on_click(move |_, _, cx| {
                                                        if let Some(oid) = &copy {
                                                            cx.write_to_clipboard(
                                                                ClipboardItem::new_string(
                                                                    oid.clone(),
                                                                ),
                                                            );
                                                        }
                                                    }),
                                                )
                                            })
                                            .into_any_element()
                                    })
                                    .collect()
                            }),
                        )
                        .track_scroll(&self.git.graph_scroll)
                        .flex_1()
                        .min_h_0()
                        .w_full(),
                    )
            })
            .into_any_element()
    }
}

// Parent-derived edges share identical lane coordinates at adjacent row boundaries.
fn graph_lane(
    layout: &glim_core::GraphLayout,
    continuation: bool,
    lanes: usize,
    cx: &App,
) -> AnyElement {
    let layout = layout.clone();
    let colors = [
        cx.theme().blue,
        cx.theme().green,
        cx.theme().magenta,
        cx.theme().cyan,
    ];
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let top = bounds.origin.y;
            let middle = top + bounds.size.height / 2.;
            let bottom = bounds.bottom_right().y;
            let x = |lane: usize| bounds.origin.x + px(lane as f32 * 14. + 7.);
            let mut line = |from, to, color: usize| {
                let mut path = PathBuilder::stroke(px(1.5));
                path.move_to(from);
                path.line_to(to);
                if let Ok(path) = path.build() {
                    window.paint_path(path, colors[color % colors.len()]);
                }
            };
            if continuation {
                for (lane, color) in layout.after.iter().enumerate() {
                    line(point(x(lane), top), point(x(lane), bottom), *color);
                }
                return;
            }
            for (from, to, color) in &layout.through {
                line(point(x(*from), top), point(x(*from), middle), *color);
                line(point(x(*from), middle), point(x(*to), bottom), *color);
            }
            if layout.incoming {
                line(
                    point(x(layout.node), top),
                    point(x(layout.node), middle),
                    layout.color,
                );
            }
            for (to, color) in &layout.parents {
                line(point(x(layout.node), middle), point(x(*to), bottom), *color);
            }
            window.paint_quad(
                fill(
                    Bounds::new(
                        point(x(layout.node) - px(3.), middle - px(3.)),
                        size(px(6.), px(6.)),
                    ),
                    colors[layout.color % colors.len()],
                )
                .corner_radii(px(3.)),
            );
        },
    )
    .w(px(lanes.max(1) as f32 * 14.))
    .flex_shrink_0()
    .h(px(40.))
    .into_any_element()
}
