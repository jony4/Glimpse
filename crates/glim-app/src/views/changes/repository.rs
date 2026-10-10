use super::*;
use glim_core::{GitOperation, GitRequest, GitSnapshot, GraphRow};
use glim_services::git::management;

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
    pub graph_rows: Vec<GraphRow>,
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
            graph_rows: Vec::new(),
            selected_commit: None,
            graph_task: None,
            graph_scroll: UniformListScrollHandle::new(),
        }
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
                    Ok(snapshot) => v.git.snapshot = Some(snapshot),
                    Err(error) => {
                        v.git.snapshot = None;
                        cx.emit(ChangeSelected::Error(format!("Git: {error:#}")));
                    }
                }
                cx.notify();
            });
        }));
        if !self.git.graph_collapsed {
            self.reload_graph(cx);
        }
    }
    pub(super) fn reset_git(&mut self, cx: &mut Context<Self>) {
        self.git.snapshot = None;
        self.git.picker = false;
        self.git.form = None;
        self.git.graph_rows.clear();
        self.git.selected_commit = None;
        self.git.graph_task = None;
        self.git.graph_limit = 200;
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
    pub(super) fn git_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let blocked = self.is_busy()
            || self.loading
            || self.git.snapshot_task.is_some()
            || self.git.snapshot.is_none()
            || self.git.confirm_task.is_some();
        let state = self.git.snapshot.clone().unwrap_or_default();
        let label = state
            .branch
            .clone()
            .or_else(|| {
                state
                    .head
                    .as_ref()
                    .map(|id| format!("Detached {}", &id[..8.min(id.len())]))
            })
            .unwrap_or_else(|| "Branch".into());
        let query = self.git.input.read(cx).value().to_lowercase();
        let controls = v_flex()
            .id("git-controls")
            .max_h(px(240.))
            .overflow_y_scroll()
            .w_full()
            .flex_shrink_0()
            .px_2()
            .gap_1()
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("git-branch-picker")
                            .ghost()
                            .small()
                            .label(format!("⑂ {label}"))
                            .tooltip("Checkout Branch…")
                            .disabled(blocked)
                            .on_click(cx.listener(|v, _, w, cx| {
                                v.git.picker = !v.git.picker;
                                v.git.form = None;
                                v.git.input.update(cx, |s, cx| s.set_value("", w, cx));
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("↓{} ↑{}", state.behind, state.ahead)),
                    ),
            )
            .child(
                h_flex().gap_1().children(
                    [
                        (GitOperation::Fetch, "Fetch"),
                        (GitOperation::Pull, "Pull"),
                        (GitOperation::Push, "Push"),
                        (GitOperation::Sync, "Sync Changes"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(i, (op, label))| {
                        let needs_upstream = !matches!(op, GitOperation::Fetch);
                        Button::new(("git-network", i))
                            .ghost()
                            .xsmall()
                            .label(label)
                            .tooltip(if needs_upstream && state.upstream.is_none() {
                                "Publish this branch to configure an upstream"
                            } else {
                                label
                            })
                            .disabled(
                                blocked
                                    || (needs_upstream && state.upstream.is_none())
                                    || state.remotes.is_empty(),
                            )
                            .on_click(cx.listener(move |v, _, w, cx| {
                                v.request_operation(op.clone(), w, cx)
                            }))
                    }),
                ),
            )
            .when(state.upstream.is_none(), |panel| {
                panel.child(h_flex().flex_wrap().gap_1().children(
                    state.remotes.iter().enumerate().map(|(i, remote)| {
                        let remote = remote.clone();
                        Button::new(("git-publish", i))
                            .ghost()
                            .xsmall()
                            .label(format!("Publish Branch → {remote}"))
                            .disabled(blocked || state.branch.is_none() || state.head.is_none())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                v.request_operation(GitOperation::Publish(remote.clone()), w, cx)
                            }))
                    }),
                ))
            })
            .when_some(self.git.operation.as_ref(), |panel, (_, label)| {
                panel.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{label}…")),
                )
            })
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
                            )
                            .child(
                                Button::new("git-close-picker")
                                    .ghost()
                                    .xsmall()
                                    .label("×")
                                    .tooltip("Close branch picker")
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.git.picker = false;
                                        v.git.form = None;
                                        cx.notify();
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
            })
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("git-stash")
                            .ghost()
                            .xsmall()
                            .label("Stash…")
                            .tooltip("Stash tracked and untracked changes")
                            .disabled(blocked)
                            .on_click(cx.listener(|v, _, w, cx| {
                                v.request_operation(GitOperation::Stash, w, cx)
                            })),
                    )
                    .when(state.merging, |bar| {
                        bar.child(
                            Button::new("git-abort-merge")
                                .ghost()
                                .xsmall()
                                .label("Abort Merge…")
                                .disabled(blocked)
                                .on_click(cx.listener(|v, _, w, cx| {
                                    v.request_operation(GitOperation::AbortMerge, w, cx)
                                })),
                        )
                    }),
            )
            .when(self.git.picker && !state.stashes.is_empty(), |panel| {
                panel.child(
                    v_flex()
                        .id("git-stashes")
                        .max_h(px(100.))
                        .overflow_y_scroll()
                        .children(state.stashes.iter().enumerate().map(|(i, (oid, subject))| {
                            let oid = oid.clone();
                            Button::new(("git-apply-stash", i))
                                .ghost()
                                .xsmall()
                                .label(format!("Apply Stash · {subject}"))
                                .disabled(blocked)
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    v.request_operation(
                                        GitOperation::ApplyStash(oid.clone()),
                                        w,
                                        cx,
                                    )
                                }))
                        })),
                )
            });
        controls.into_any_element()
    }
    pub(super) fn reload_graph(&mut self, cx: &mut Context<Self>) {
        self.git.graph_task = None;
        if self.git.graph_collapsed {
            return;
        }
        let Some(root) = self.repository().map(|r| r.root.clone()) else {
            return;
        };
        let limit = self.git.graph_limit;
        let all = self.git.graph_all;
        let work = cx
            .background_executor()
            .spawn(async move { management::graph(&root, limit, all) });
        self.git.graph_task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |v, cx| {
                v.git.graph_task = None;
                match result {
                    Ok(rows) => v.git.graph_rows = rows,
                    Err(e) => cx.emit(ChangeSelected::Error(format!("Graph: {e:#}"))),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn show_commit(&mut self, oid: String, cx: &mut Context<Self>) {
        self.git.selected_commit = Some(oid.clone());
        cx.notify();
        if let Some(repo) = self.repository() {
            cx.emit(ChangeSelected::Commit(repo.root.clone(), oid));
        }
    }
    pub(super) fn graph_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
                        Button::new("git-graph-toggle")
                            .ghost()
                            .small()
                            .flex_1()
                            .justify_start()
                            .icon(disclosure(self.git.graph_collapsed, cx))
                            .label("Graph")
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
                    .child(
                        h_flex()
                            .px_2()
                            .gap_1()
                            .child(
                                Button::new("git-graph-scope")
                                    .ghost()
                                    .xsmall()
                                    .label(if self.git.graph_all {
                                        "All Branches"
                                    } else {
                                        "Current Branch"
                                    })
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.git.graph_all = !v.git.graph_all;
                                        v.git.graph_limit = 200;
                                        v.reload_graph(cx);
                                    })),
                            )
                            .child(
                                Button::new("git-graph-more")
                                    .ghost()
                                    .xsmall()
                                    .label("Load More")
                                    .disabled(
                                        self.git.graph_task.is_some()
                                            || self.git.graph_limit >= 1000,
                                    )
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.git.graph_limit = (v.git.graph_limit + 200).min(1000);
                                        v.reload_graph(cx);
                                    })),
                            ),
                    )
                    .when(self.git.graph_task.is_some(), |panel| {
                        panel.child(div().px_3().text_xs().child("Loading…"))
                    })
                    .when(
                        self.git.graph_rows.is_empty() && self.git.graph_task.is_none(),
                        |panel| panel.child(div().px_3().text_sm().child("No commits yet.")),
                    )
                    .child(
                        uniform_list(
                            "git-graph-rows",
                            self.git.graph_rows.len(),
                            cx.processor(|v, range: std::ops::Range<usize>, _, cx| {
                                range
                                    .map(|i| {
                                        let row = &v.git.graph_rows[i];
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
                                            .child(graph_lane(&row.graph, cx))
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
                                            .on_click(cx.listener(move |v, _, _, cx| {
                                                if let Some(oid) = &oid {
                                                    v.show_commit(oid.clone(), cx);
                                                }
                                            }))
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

// Git computes topology; paint its lane geometry with theme colors, not font glyphs.
fn graph_lane(prefix: &str, cx: &App) -> AnyElement {
    let prefix = prefix.to_owned();
    let width = px((prefix.len().max(2) * 7) as f32);
    let colors = [
        cx.theme().blue,
        cx.theme().green,
        cx.theme().magenta,
        cx.theme().cyan,
    ];
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            for (column, ch) in prefix.chars().enumerate() {
                let x = bounds.origin.x + px(column as f32 * 7. + 3.5);
                let top = bounds.origin.y;
                let bottom = bounds.bottom_right().y;
                let middle = top + bounds.size.height / 2.;
                let color = colors[(column / 2) % colors.len()];
                let endpoints = match ch {
                    '|' | '*' => Some((point(x, top), point(x, bottom))),
                    '/' => Some((point(x + px(7.), top), point(x - px(7.), bottom))),
                    '\\' => Some((point(x - px(7.), top), point(x + px(7.), bottom))),
                    '_' | '-' => Some((point(x - px(7.), middle), point(x + px(7.), middle))),
                    _ => None,
                };
                if let Some((start, end)) = endpoints {
                    let mut path = PathBuilder::stroke(px(1.5));
                    path.move_to(start);
                    path.line_to(end);
                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                }
                if ch == '*' {
                    window.paint_quad(
                        fill(
                            Bounds::new(point(x - px(3.), middle - px(3.)), size(px(6.), px(6.))),
                            color,
                        )
                        .corner_radii(px(3.)),
                    );
                }
            }
        },
    )
    .w(width)
    .h(px(40.))
    .into_any_element()
}
