mod tree;
use super::file_icons::file_icon;
use glimpse_core::{DiffScope, GitChange, Repository};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, Selectable,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        h_flex,
        input::{Input, InputState},
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
    collapsed: HashSet<(DiffScope, PathBuf)>,
    rows: Vec<Row>,
    scroll: UniformListScrollHandle,
    message: Entity<InputState>,
    drafts: HashMap<PathBuf, String>,
    busy: bool,
    notice: Option<SharedString>,
    task: Option<Task<()>>,
}
impl EventEmitter<ChangeSelected> for Changes {}
impl Changes {
    pub fn new(repositories: Vec<Repository>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let message =
            cx.new(|cx| InputState::new(window, cx).placeholder("Message for staged changes"));
        let mut view = Self {
            repositories,
            selected_repo: 0,
            selected: None,
            tree: true,
            collapsed: HashSet::new(),
            rows: Vec::new(),
            scroll: UniformListScrollHandle::new(),
            message,
            drafts: HashMap::new(),
            busy: false,
            notice: None,
            task: None,
        };
        view.rebuild();
        view
    }
    pub fn repository(&self) -> Option<&Repository> {
        self.repositories.get(self.selected_repo)
    }
    pub fn is_busy(&self) -> bool {
        self.busy
    }
    pub fn set_repositories(&mut self, repositories: Vec<Repository>, cx: &mut Context<Self>) {
        let root = self.repository().map(|r| r.root.clone());
        self.selected_repo = root
            .and_then(|root| repositories.iter().position(|r| r.root == root))
            .unwrap_or(0);
        self.repositories = repositories;
        self.rebuild();
        cx.notify();
    }
    fn rebuild(&mut self) {
        self.rows = self
            .repository()
            .map(|repo| tree::rows(&repo.changes, self.tree, &self.collapsed))
            .unwrap_or_default();
    }
    fn mutate(&mut self, change: Option<GitChange>, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(root) = self.repository().map(|r| r.root.clone()) else {
            return;
        };
        let message = self.message.read(cx).value().to_string();
        let committing = change.is_none();
        if committing && message.trim().is_empty() {
            self.notice = Some("Enter a commit message.".into());
            cx.notify();
            return;
        }
        self.busy = true;
        self.notice = None;
        let work = cx.background_executor().spawn(async move {
            if let Some(change) = change {
                glimpse_services::git::stage(&root, &change, change.scope != DiffScope::Index)
            } else {
                glimpse_services::git::commit(&root, &message)
            }
        });
        self.task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = work.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.busy = false;
                match result {
                    Ok(()) => {
                        if committing {
                            view.message.update(cx, |s, cx| s.set_value("", window, cx));
                            view.notice = Some("Commit created locally.".into());
                        }
                    }
                    Err(error) => view.notice = Some(format!("{error:#}").into()),
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
        let staged = self
            .repository()
            .is_some_and(|r| r.changes.iter().any(|c| c.scope == DiffScope::Index));
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Repositories"),
            )
            .children(self.repositories.iter().enumerate().map(|(i, r)| {
                Button::new(("repo", i))
                    .custom(ButtonCustomVariant::new(cx))
                    .selected(i == self.selected_repo)
                    .disabled(self.busy)
                    .accessibility_label(format!("Repository {}", r.root.display()))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                div().flex_1().truncate().child(
                                    r.root
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .into_owned(),
                                ),
                            )
                            .child(div().text_xs().child(format!("⑂ {}", r.branch))),
                    )
                    .w_full()
                    .on_click(cx.listener(move |view, _, window, cx| {
                        if let Some(root) = view.repository().map(|r| r.root.clone()) {
                            view.drafts
                                .insert(root, view.message.read(cx).value().to_string());
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
                        cx.notify();
                    }))
            }))
            .when(self.repositories.is_empty(), |v| {
                v.child(div().p_3().text_sm().child("No Git repositories found."))
            })
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_1()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Changes"),
                    )
                    .child(
                        Button::new("changes-tree")
                            .ghost()
                            .label("Tree")
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
                            .label("List")
                            .selected(!self.tree)
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.tree = false;
                                v.rebuild();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .px_3()
                    .pb_2()
                    .child(Input::new(&self.message).disabled(self.busy)),
            )
            .child(
                div().px_3().pb_2().child(
                    Button::new("commit")
                        .primary()
                        .w_full()
                        .label(if self.busy {
                            "Working…"
                        } else {
                            "Commit staged changes"
                        })
                        .disabled(self.busy || !staged)
                        .on_click(cx.listener(|v, _, w, cx| v.mutate(None, w, cx))),
                ),
            )
            .when_some(self.notice.clone(), |v, message| {
                v.child(div().px_3().pb_2().text_xs().child(message))
            })
            .child(
                uniform_list(
                    "changes",
                    self.rows.len(),
                    cx.processor(|view, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|i| match view.rows[i].clone() {
                                Row::Group(scope) => div()
                                    .h(px(28.))
                                    .px_3()
                                    .py_1()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!(
                                        "{}  {}",
                                        scope.label(),
                                        view.repository().map_or(0, |r| r
                                            .changes
                                            .iter()
                                            .filter(|c| c.scope == scope)
                                            .count())
                                    ))
                                    .into_any_element(),
                                Row::Directory(scope, path, depth) => {
                                    let label = format!(
                                        "{} {}",
                                        if view.collapsed.contains(&(scope, path.clone())) {
                                            "▸"
                                        } else {
                                            "▾"
                                        },
                                        path.file_name().unwrap_or_default().to_string_lossy()
                                    );
                                    h_flex()
                                        .id(("git-dir", i))
                                        .role(Role::Button)
                                        .aria_label(label.clone())
                                        .h(px(28.))
                                        .w_full()
                                        .min_w_0()
                                        .pl(px(12. + depth as f32 * 16.))
                                        .pr_2()
                                        .text_sm()
                                        .cursor_pointer()
                                        .child(div().min_w_0().truncate().child(label))
                                        .on_click(cx.listener(move |v, _, _, cx| {
                                            let key = (scope, path.clone());
                                            if !v.collapsed.remove(&key) {
                                                v.collapsed.insert(key);
                                            }
                                            v.rebuild();
                                            cx.notify();
                                        }))
                                        .into_any_element()
                                }
                                Row::File(index, depth) => {
                                    let repo = view.repository().unwrap();
                                    let change = repo.changes[index].clone();
                                    let root = repo.root.clone();
                                    let action = change.clone();
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
                                        .pl(px(12. + depth as f32 * 16.))
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
                                        .child(
                                            Button::new(("stage", i))
                                                .h(px(24.))
                                                .w(px(24.))
                                                .px_0()
                                                .flex_shrink_0()
                                                .ghost()
                                                .label(if action.scope == DiffScope::Index {
                                                    "−"
                                                } else {
                                                    "+"
                                                })
                                                .accessibility_label(
                                                    if action.scope == DiffScope::Index {
                                                        "Unstage file"
                                                    } else {
                                                        "Stage file"
                                                    },
                                                )
                                                .disabled(view.busy)
                                                .on_click(cx.listener(move |v, _, w, cx| {
                                                    v.mutate(Some(action.clone()), w, cx)
                                                })),
                                        )
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
    }
}
