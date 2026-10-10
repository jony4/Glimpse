mod repository;
mod tree;
use super::file_icons::file_icon;
use glim_core::{DiffScope, GitChange, Repository};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, Icon, Selectable, Sizable,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        menu::ContextMenuExt,
        resizable::{ResizableState, resizable_panel, v_resizable},
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

pub enum ChangeSelected {
    Open(PathBuf, GitChange),
    Refresh,
    Error(String),
    Git(PathBuf, glim_core::GitRequest),
    GitFinished,
    Commit(PathBuf, String),
}
#[derive(Clone)]
enum Row {
    Group(DiffScope),
    Directory(DiffScope, PathBuf, usize),
    File(usize, usize),
}

pub struct Changes {
    pub repositories: Vec<Repository>,
    pub selected_repo: usize,
    selected: Option<GitChange>,
    tree: bool,
    split_layout: Entity<ResizableState>,
    expanded_heights: [Pixels; 3],
    git: repository::GitState,
    repos_collapsed: bool,
    changes_collapsed: bool,
    loading: bool,
    _message_subscription: Subscription,
    collapsed: HashSet<(DiffScope, PathBuf)>,
    rows: Vec<Row>,
    scroll: UniformListScrollHandle,
    message: Entity<InputState>,
    drafts: HashMap<PathBuf, String>,
    busy: bool,
    external_busy: bool,
    notice: Option<SharedString>,
    task: Option<Task<()>>,
}
impl EventEmitter<ChangeSelected> for Changes {}
impl Changes {
    pub fn new(repositories: Vec<Repository>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let message =
            cx.new(|cx| InputState::new(window, cx).placeholder("Commit message · ⌘Enter"));
        let message_subscription =
            cx.subscribe_in(&message, window, |view, _, event, window, cx| {
                if matches!(
                    event,
                    InputEvent::PressEnter {
                        secondary: true,
                        ..
                    }
                ) {
                    view.commit(window, cx);
                }
            });
        let mut view = Self {
            repositories,
            selected_repo: 0,
            selected: None,
            tree: true,
            split_layout: cx.new(|_| ResizableState::default()),
            expanded_heights: [px(180.), px(300.), px(280.)],
            git: repository::GitState::new(window, cx),
            repos_collapsed: false,
            changes_collapsed: false,
            loading: false,
            _message_subscription: message_subscription,
            collapsed: HashSet::new(),
            rows: Vec::new(),
            scroll: UniformListScrollHandle::new(),
            message,
            drafts: HashMap::new(),
            busy: false,
            external_busy: false,
            notice: None,
            task: None,
        };
        view.rebuild();
        view.reload_git(cx);
        view
    }
    fn toggle_section(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let collapsed = match index {
            0 => &mut self.repos_collapsed,
            1 => &mut self.changes_collapsed,
            _ => &mut self.git.graph_collapsed,
        };
        let height = if *collapsed {
            self.expanded_heights[index]
        } else {
            if let Some(height) = self.split_layout.read(cx).sizes().get(index) {
                self.expanded_heights[index] = *height;
            }
            if index == 1 { px(38.) } else { px(36.) }
        };
        *collapsed = !*collapsed;
        self.split_layout.update(cx, |state, cx| {
            state.resize_panel(index, height, window, cx)
        });
        if index == 2 {
            if self.git.graph_collapsed {
                self.git.graph_task = None;
            } else {
                self.reload_graph(cx);
            }
        }
        cx.notify();
    }

    pub fn repository(&self) -> Option<&Repository> {
        self.repositories.get(self.selected_repo)
    }
    pub fn is_busy(&self) -> bool {
        self.busy || self.external_busy
    }
    pub fn set_external_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.external_busy = busy;
        cx.notify();
    }
    pub fn set_repositories(&mut self, repositories: Vec<Repository>, cx: &mut Context<Self>) {
        let root = self.repository().map(|r| r.root.clone());
        self.selected_repo = root
            .clone()
            .and_then(|root| repositories.iter().position(|r| r.root == root))
            .unwrap_or(0);
        let changed_root = root.as_ref() != repositories.get(self.selected_repo).map(|r| &r.root);
        self.repositories = repositories;
        self.rebuild();
        if changed_root {
            self.reset_git(cx);
        } else if !self.busy {
            self.reload_git(cx);
        }
        cx.notify();
    }
    fn rebuild(&mut self) {
        self.rows = self
            .repository()
            .map(|repo| tree::rows(&repo.changes, self.tree, &self.collapsed))
            .unwrap_or_default();
    }
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }
    fn stage_selection(
        &mut self,
        scope: DiffScope,
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_busy() || self.loading {
            return;
        }
        let Some(repo) = self.repository() else {
            return;
        };
        let root = repo.root.clone();
        let changes = repo
            .changes
            .iter()
            .filter(|c| {
                tree::group(c.scope) == scope && path.as_ref().is_none_or(|p| c.path.starts_with(p))
            })
            .cloned()
            .collect::<Vec<_>>();
        if changes.is_empty() {
            return;
        }
        self.busy = true;
        self.notice = None;
        let work = cx.background_executor().spawn(async move {
            glim_services::git::set_staged(&root, &changes, scope != DiffScope::Index)
        });
        self.task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |v, cx| {
                v.busy = false;
                if let Err(e) = result {
                    cx.emit(ChangeSelected::Error(format!("Git: {e:#}")));
                }
                cx.emit(ChangeSelected::Refresh);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn stage_button(
        &self,
        id: usize,
        scope: DiffScope,
        path: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) -> Button {
        let label = if scope == DiffScope::Index {
            "Unstage"
        } else {
            "Stage"
        };
        let target = path
            .as_ref()
            .map_or_else(|| "all changes".to_string(), |p| p.display().to_string());
        Button::new(("stage-selection", id))
            .ghost()
            .xsmall()
            .icon(Icon::new(if scope == DiffScope::Index {
                gpui_kit::assets::IconName::Minus
            } else {
                gpui_kit::assets::IconName::Plus
            }))
            .tooltip(format!("{label} {target}"))
            .accessibility_label(format!("{label} {target}"))
            .disabled(self.is_busy() || self.loading)
            .on_click(cx.listener(move |v, _, w, cx| {
                cx.stop_propagation();
                v.stage_selection(scope, path.clone(), w, cx);
            }))
    }

    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_busy() || self.loading {
            return;
        }
        let Some(root) = self.repository().map(|r| r.root.clone()) else {
            return;
        };
        let message = self.message.read(cx).value().to_string();
        if message.trim().is_empty() {
            cx.emit(ChangeSelected::Error("Enter a commit message.".into()));
            cx.notify();
            return;
        }
        self.busy = true;
        self.notice = None;
        let work = cx
            .background_executor()
            .spawn(async move { glim_services::git::commit(&root, &message) });
        self.task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = work.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.busy = false;
                match result {
                    Ok(()) => {
                        view.message.update(cx, |s, cx| s.set_value("", window, cx));
                        view.notice = Some("Commit created locally.".into());
                    }
                    Err(error) => cx.emit(ChangeSelected::Error(format!("Commit: {error:#}"))),
                }
                cx.emit(ChangeSelected::Refresh);
                cx.notify();
            });
        }));
        cx.notify();
    }
}
impl Render for Changes {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let repositories = v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .child(
                h_flex()
                    .id("repositories-heading")
                    .role(Role::Button)
                    .aria_label(if self.repos_collapsed {
                        "Expand repositories"
                    } else {
                        "Collapse repositories"
                    })
                    .h(px(36.))
                    .flex_shrink_0()
                    .px_2()
                    .gap_1()
                    .text_size(px(15.))
                    .font_weight(FontWeight::BOLD)
                    .cursor_pointer()
                    .child(disclosure(self.repos_collapsed, cx))
                    .child(div().flex_1().child("Repositories"))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::NORMAL)
                            .text_color(cx.theme().muted_foreground)
                            .child(self.repositories.len().to_string()),
                    )
                    .on_click(cx.listener(|v, _, window, cx| v.toggle_section(0, window, cx))),
            )
            .when(!self.repos_collapsed, |panel| {
                panel.child(
                    v_flex()
                        .id("repositories-list")
                        .px_2()
                        .pb_2()
                        .w_full()
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .children(self.repositories.iter().enumerate().map(|(i, r)| {
                            let menu_root = r.root.clone();
                            h_flex()
                                .id(("repo", i))
                                .role(Role::Button)
                                .aria_label(format!("Repository {}", r.root.display()))
                                .h(px(28.))
                                .flex_shrink_0()
                                .w_full()
                                .min_w_0()
                                .pl(px(22.))
                                .pr_2()
                                .gap_2()
                                .text_sm()
                                .rounded(px(4.))
                                .border_l_2()
                                .border_color(if i == self.selected_repo {
                                    cx.theme().primary
                                } else {
                                    cx.theme().transparent
                                })
                                .when(i == self.selected_repo, |row| row.bg(cx.theme().accent))
                                .cursor_pointer()
                                .child(
                                    Icon::new(gpui_kit::assets::IconName::FolderGit2)
                                        .size(px(15.))
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .child(
                                    div().flex_1().min_w_0().truncate().child(
                                        r.root
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .into_owned(),
                                    ),
                                )
                                .child(
                                    div()
                                        .max_w(px(110.))
                                        .truncate()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("⑂ {}", r.branch)),
                                )
                                .on_click(cx.listener(move |view, _, window, cx| {
                                    if view.is_busy() {
                                        return;
                                    }
                                    if let Some(root) = view.repository().map(|r| r.root.clone()) {
                                        view.drafts.insert(
                                            root,
                                            view.message.read(cx).value().to_string(),
                                        );
                                    }
                                    view.selected_repo = i;
                                    view.collapsed.clear();
                                    view.scroll = UniformListScrollHandle::new();
                                    view.selected = None;
                                    view.notice = None;
                                    let message = view
                                        .repository()
                                        .and_then(|r| view.drafts.get(&r.root))
                                        .cloned()
                                        .unwrap_or_default();
                                    view.message
                                        .update(cx, |s, cx| s.set_value(message, window, cx));
                                    view.rebuild();
                                    view.reset_git(cx);
                                    cx.notify();
                                }))
                                .context_menu(move |menu, _, _| {
                                    super::explorer::path_menu(menu, &menu_root, &menu_root)
                                })
                        }))
                        .when(self.repositories.is_empty(), |list| {
                            list.child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(if self.loading {
                                        "Finding repositories…"
                                    } else {
                                        "No Git repositories found."
                                    }),
                            )
                        }),
                )
            });
        let changes = v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .child(
                h_flex()
                    .h(px(38.))
                    .flex_shrink_0()
                    .px_2()
                    .gap_1()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .id("changes-heading")
                            .role(Role::Button)
                            .aria_label(if self.changes_collapsed {
                                "Expand changes"
                            } else {
                                "Collapse changes"
                            })
                            .flex_1()
                            .h_full()
                            .gap_1()
                            .text_size(px(15.))
                            .font_weight(FontWeight::BOLD)
                            .cursor_pointer()
                            .child(disclosure(self.changes_collapsed, cx))
                            .child("Changes")
                            .on_click(
                                cx.listener(|v, _, window, cx| v.toggle_section(1, window, cx)),
                            ),
                    )
                    .child(
                        Button::new("changes-tree")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(gpui_kit::assets::IconName::ListTree))
                            .tooltip("Tree")
                            .accessibility_label("Tree")
                            .selected(self.tree)
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.tree = true;
                                v.rebuild();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("changes-list")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(gpui_kit::assets::IconName::List))
                            .tooltip("List")
                            .accessibility_label("List")
                            .selected(!self.tree)
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.tree = false;
                                v.rebuild();
                                cx.notify();
                            })),
                    ),
            )
            .when(!self.changes_collapsed, |panel| {
                panel
                    .child(self.git_controls(cx))
                    .child(
                        div().pl(px(26.)).pr_3().pb_3().child(
                            Input::new(&self.message)
                                .small()
                                .bordered(false)
                                .focus_bordered(false)
                                .disabled(self.is_busy() || self.repositories.is_empty()),
                        ),
                    )
                    .when_some(self.notice.clone(), |v, message| {
                        v.child(div().px_3().pb_1().text_xs().child(message))
                    })
            })
            .when(!self.changes_collapsed, |panel| {
                panel.child(
                    uniform_list(
                        "changes",
                        self.rows.len(),
                        cx.processor(|view, range: std::ops::Range<usize>, _, cx| {
                            range
                                .map(|i| match view.rows[i].clone() {
                                    Row::Group(scope) => h_flex()
                                        .w_full()
                                        .id(("change-group", i))
                                        .role(Role::Button)
                                        .aria_label(tree::label(scope))
                                        .h(px(28.))
                                        .pl(px(22.))
                                        .pr_2()
                                        .gap_1()
                                        .text_sm()
                                        .font_weight(FontWeight::NORMAL)
                                        .bg(cx.theme().secondary)
                                        .cursor_pointer()
                                        .child(disclosure(
                                            view.collapsed.contains(&(scope, PathBuf::new())),
                                            cx,
                                        ))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .truncate()
                                                .child(tree::label(scope)),
                                        )
                                        .child(view.stage_button(i, scope, None, cx))
                                        .child(
                                            div()
                                                .min_w(px(18.))
                                                .h(px(18.))
                                                .px_1()
                                                .rounded_full()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_xs()
                                                .bg(cx.theme().primary)
                                                .text_color(cx.theme().primary_foreground)
                                                .child(
                                                    view.repository()
                                                        .map_or(0, |r| {
                                                            r.changes
                                                                .iter()
                                                                .filter(|c| {
                                                                    tree::group(c.scope) == scope
                                                                })
                                                                .count()
                                                        })
                                                        .to_string(),
                                                ),
                                        )
                                        .on_click(cx.listener(move |v, _, _, cx| {
                                            let key = (scope, PathBuf::new());
                                            if !v.collapsed.remove(&key) {
                                                v.collapsed.insert(key);
                                            }
                                            v.rebuild();
                                            cx.notify();
                                        }))
                                        .into_any_element(),
                                    Row::Directory(scope, path, depth) => {
                                        let menu_root = view.repository().unwrap().root.clone();
                                        let menu_path = menu_root.join(&path);
                                        let collapsed =
                                            view.collapsed.contains(&(scope, path.clone()));
                                        let label = path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .into_owned();
                                        h_flex()
                                            .id(("git-dir", i))
                                            .role(Role::Button)
                                            .aria_label(format!(
                                                "{} {label}",
                                                if collapsed { "▸" } else { "▾" }
                                            ))
                                            .h(px(28.))
                                            .w_full()
                                            .min_w_0()
                                            .pl(px(34. + depth as f32 * 16.))
                                            .pr_2()
                                            .text_sm()
                                            .gap_1()
                                            .cursor_pointer()
                                            .child(disclosure(collapsed, cx))
                                            .child(div().flex_1().min_w_0().truncate().child(label))
                                            .child(view.stage_button(
                                                i,
                                                scope,
                                                Some(path.clone()),
                                                cx,
                                            ))
                                            .on_click(cx.listener(move |v, _, _, cx| {
                                                let key = (scope, path.clone());
                                                if !v.collapsed.remove(&key) {
                                                    v.collapsed.insert(key);
                                                }
                                                v.rebuild();
                                                cx.notify();
                                            }))
                                            .context_menu(move |menu, _, _| {
                                                super::explorer::path_menu(
                                                    menu, &menu_path, &menu_root,
                                                )
                                            })
                                            .into_any_element()
                                    }
                                    Row::File(index, depth) => {
                                        let repo = view.repository().unwrap();
                                        let change = repo.changes[index].clone();
                                        let root = repo.root.clone();
                                        let menu_root = root.clone();
                                        let menu_path = root.join(&change.path);
                                        let stage_button = view.stage_button(
                                            i,
                                            tree::group(change.scope),
                                            Some(change.path.clone()),
                                            cx,
                                        );
                                        let label = if view.tree {
                                            change
                                                .path
                                                .file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .into_owned()
                                        } else {
                                            change.path.to_string_lossy().into_owned()
                                        };
                                        h_flex()
                                            .w_full()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .h(px(28.))
                                            .pl(px(34. + depth as f32 * 16.))
                                            .pr_2()
                                            .when(view.selected.as_ref() == Some(&change), |v| {
                                                v.bg(cx.theme().accent)
                                            })
                                            .child(
                                                h_flex()
                                                    .id(("change", i))
                                                    .role(Role::Button)
                                                    .aria_label(format!(
                                                        "{} · {}",
                                                        change.path.display(),
                                                        change.scope.label()
                                                    ))
                                                    .h(px(28.))
                                                    .flex_1()
                                                    .min_w_0()
                                                    .overflow_hidden()
                                                    .text_sm()
                                                    .cursor_pointer()
                                                    .gap_2()
                                                    .child(file_icon(&change.path))
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .truncate()
                                                            .child(label),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_shrink_0()
                                                            .w(px(14.))
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(change.status.to_string()),
                                                    )
                                                    .on_click(cx.listener(move |v, _, _, cx| {
                                                        v.selected = Some(change.clone());
                                                        cx.emit(ChangeSelected::Open(
                                                            root.clone(),
                                                            change.clone(),
                                                        ));
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(stage_button)
                                            .context_menu(move |menu, _, _| {
                                                super::explorer::path_menu(
                                                    menu, &menu_path, &menu_root,
                                                )
                                            })
                                            .into_any_element()
                                    }
                                })
                                .collect()
                        }),
                    )
                    .flex_1()
                    .w_full()
                    .min_h_0()
                    .track_scroll(&self.scroll),
                )
            });
        let graph = self.graph_panel(cx);
        v_resizable("source-control-sections")
            .with_state(&self.split_layout)
            .child(
                resizable_panel()
                    .size(px(180.))
                    .flex_none()
                    .size_range(px(36.)..px(10000.))
                    .child(repositories),
            )
            .child(
                resizable_panel()
                    .size_range(px(38.)..px(10000.))
                    .child(changes),
            )
            .child(
                resizable_panel()
                    .size(px(36.))
                    .flex_none()
                    .size_range(px(36.)..px(10000.))
                    .child(graph),
            )
    }
}

fn disclosure(collapsed: bool, cx: &App) -> Icon {
    Icon::new(if collapsed {
        gpui_kit::assets::IconName::ChevronRight
    } else {
        gpui_kit::assets::IconName::ChevronDown
    })
    .size(px(14.))
    .flex_shrink_0()
    .text_color(cx.theme().muted_foreground)
}
