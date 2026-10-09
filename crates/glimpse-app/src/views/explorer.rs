use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use glimpse_core::DirectoryEntry;
use glimpse_services::workspace::list_directory;
use gpui_kit::{
    component::{
        ActiveTheme,
        button::{Button, ButtonCustomVariant, ButtonVariants},
    },
    prelude::FluentBuilder,
    *,
};

pub enum ExplorerEvent {
    OpenFile(PathBuf),
    Error(String),
}

gpui_kit::actions!(explorer, [Up, Down, Expand, Collapse, OpenSelected]);

#[derive(Clone)]
struct Row {
    entry: DirectoryEntry,
    depth: usize,
}

/// Only expanded directories are loaded, and only visible rows are rendered.
pub struct Explorer {
    root: PathBuf,
    directories: HashMap<PathBuf, Vec<DirectoryEntry>>,
    expanded: HashSet<PathBuf>,
    tasks: HashMap<PathBuf, Task<()>>,
    rows: Vec<Row>,
    selected: Option<usize>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
}

impl EventEmitter<ExplorerEvent> for Explorer {}

impl Explorer {
    pub fn init(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("up", Up, Some("Explorer")),
            KeyBinding::new("down", Down, Some("Explorer")),
            KeyBinding::new("right", Expand, Some("Explorer")),
            KeyBinding::new("left", Collapse, Some("Explorer")),
            KeyBinding::new("enter", OpenSelected, Some("Explorer")),
        ]);
    }

    pub fn new(root: PathBuf, entries: Vec<DirectoryEntry>, cx: &mut Context<Self>) -> Self {
        let rows = entries
            .iter()
            .cloned()
            .map(|entry| Row { entry, depth: 0 })
            .collect();
        Self {
            root: root.clone(),
            directories: HashMap::from([(root, entries)]),
            expanded: HashSet::new(),
            tasks: HashMap::new(),
            rows,
            selected: None,
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
        }
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let mut paths = self.expanded.iter().cloned().collect::<Vec<_>>();
        paths.push(self.root.clone());
        for path in paths {
            let read_path = path.clone();
            let read = cx
                .background_executor()
                .spawn(async move { list_directory(&read_path) });
            let key = path.clone();
            let task = cx.spawn(async move |view, cx| {
                let result = read.await;
                let _ = view.update(cx, |view, cx| {
                    if let Ok(entries) = result {
                        view.directories.insert(path.clone(), entries);
                    } else {
                        view.directories.remove(&path);
                    }
                    view.tasks.remove(&path);
                    view.rebuild();
                    cx.notify();
                });
            });
            self.tasks.insert(key, task);
        }
    }

    fn rebuild(&mut self) {
        fn append(path: &PathBuf, depth: usize, this: &Explorer, rows: &mut Vec<Row>) {
            if let Some(children) = this.directories.get(path) {
                for entry in children {
                    rows.push(Row {
                        entry: entry.clone(),
                        depth,
                    });
                    if this.expanded.contains(&entry.path) {
                        append(&entry.path, depth + 1, this, rows);
                    }
                }
            }
        }
        let selected = self
            .selected
            .and_then(|i| self.rows.get(i))
            .map(|r| r.entry.path.clone());
        let mut rows = Vec::new();
        append(&self.root, 0, self, &mut rows);
        self.selected = selected.and_then(|path| rows.iter().position(|r| r.entry.path == path));
        self.rows = rows;
    }

    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.rows.get(index).cloned() else {
            return;
        };
        self.selected = Some(index);
        window.focus(&self.focus, cx);
        if !row.entry.is_dir {
            cx.emit(ExplorerEvent::OpenFile(row.entry.path));
        } else if !self.expanded.remove(&row.entry.path) {
            self.expanded.insert(row.entry.path.clone());
            if !self.directories.contains_key(&row.entry.path)
                && !self.tasks.contains_key(&row.entry.path)
            {
                let path = row.entry.path;
                let read_path = path.clone();
                let read = cx
                    .background_executor()
                    .spawn(async move { list_directory(&read_path) });
                let key = path.clone();
                let task = cx.spawn(async move |view, cx| {
                    let result = read.await;
                    let _ = view.update(cx, |view, cx| {
                        view.tasks.remove(&path);
                        match result {
                            Ok(entries) => {
                                view.directories.insert(path, entries);
                            }
                            Err(error) => {
                                view.expanded.remove(&path);
                                cx.emit(ExplorerEvent::Error(format!("{error:#}")));
                            }
                        }
                        view.rebuild();
                        cx.notify();
                    });
                });
                self.tasks.insert(key, task);
            }
        }
        self.rebuild();
        cx.notify();
    }

    fn move_selection(&mut self, down: bool, cx: &mut Context<Self>) {
        if self.rows.is_empty() {
            return;
        }
        let selected = match self.selected {
            Some(i) if down => (i + 1).min(self.rows.len() - 1),
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.selected = Some(selected);
        self.scroll
            .scroll_to_item(selected, ScrollStrategy::Nearest);
        cx.notify();
    }
}

impl Render for Explorer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .key_context("Explorer")
            .track_focus(&self.focus)
            .on_action(cx.listener(|view, _: &Up, _, cx| view.move_selection(false, cx)))
            .on_action(cx.listener(|view, _: &Down, _, cx| view.move_selection(true, cx)))
            .on_action(cx.listener(|view, _: &OpenSelected, window, cx| {
                if let Some(index) = view.selected {
                    view.activate(index, window, cx);
                }
            }))
            .on_action(cx.listener(|view, _: &Expand, window, cx| {
                if let Some(index) = view.selected
                    && let Some(row) = view.rows.get(index)
                    && row.entry.is_dir
                    && !view.expanded.contains(&row.entry.path)
                {
                    view.activate(index, window, cx);
                }
            }))
            .on_action(cx.listener(|view, _: &Collapse, window, cx| {
                if let Some(index) = view.selected
                    && let Some(row) = view.rows.get(index)
                {
                    if view.expanded.contains(&row.entry.path) {
                        view.activate(index, window, cx);
                    } else if let Some(parent) = row.entry.path.parent() {
                        view.selected = view.rows.iter().position(|r| r.entry.path == parent);
                        cx.notify();
                    }
                }
            }))
            .when(self.rows.is_empty(), |view| {
                view.child(
                    div()
                        .p_4()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("This folder is empty."),
                )
            })
            .child(
                uniform_list(
                    "files",
                    self.rows.len(),
                    cx.processor(|view, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|index| {
                                let row = &view.rows[index];
                                let marker = if view.tasks.contains_key(&row.entry.path) {
                                    "…"
                                } else if row.entry.is_dir {
                                    if view.expanded.contains(&row.entry.path) {
                                        "▾"
                                    } else {
                                        "▸"
                                    }
                                } else if row.entry.is_symlink {
                                    "↗"
                                } else {
                                    "·"
                                };
                                let name = row
                                    .entry
                                    .path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy();
                                div()
                                    .h(px(32.))
                                    .pl(px(8. + row.depth as f32 * 14.))
                                    .pr_2()
                                    .when(view.selected == Some(index), |row| {
                                        row.bg(cx.theme().accent)
                                    })
                                    .child(
                                        Button::new(("entry", index))
                                            .custom(ButtonCustomVariant::new(cx))
                                            .accessibility_label(format!("{marker}  {name}"))
                                            .child(
                                                div().w_full().text_left().truncate().child(
                                                    gpui_kit::component::h_flex()
                                                        .gap_2()
                                                        .child(if row.entry.is_dir {
                                                            div().child(marker).into_any_element()
                                                        } else {
                                                            super::file_icons::file_icon(
                                                                &row.entry.path,
                                                            )
                                                            .into_any_element()
                                                        })
                                                        .child(name.into_owned()),
                                                ),
                                            )
                                            .w_full()
                                            .h(px(32.))
                                            .justify_start()
                                            .on_click(cx.listener(move |view, _, window, cx| {
                                                view.activate(index, window, cx)
                                            })),
                                    )
                            })
                            .collect()
                    }),
                )
                .size_full()
                .track_scroll(&self.scroll),
            )
    }
}
