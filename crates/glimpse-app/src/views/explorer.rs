use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use glimpse_core::DirectoryEntry;
use glimpse_services::workspace::list_directory;
use gpui_kit::{
    component::{
        ActiveTheme, FocusableExt,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
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
    parent: Option<usize>,
    subtree_end: usize,
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
        let mut view = Self {
            root: root.clone(),
            directories: HashMap::from([(root, entries)]),
            expanded: HashSet::new(),
            tasks: HashMap::new(),
            rows: Vec::new(),
            selected: None,
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
        };
        view.rebuild();
        view
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
        fn append(
            path: &PathBuf,
            depth: usize,
            parent: Option<usize>,
            this: &Explorer,
            rows: &mut Vec<Row>,
        ) {
            if let Some(children) = this.directories.get(path) {
                for entry in children {
                    let index = rows.len();
                    rows.push(Row {
                        entry: entry.clone(),
                        depth,
                        parent,
                        subtree_end: index + 1,
                    });
                    if this.expanded.contains(&entry.path) {
                        append(&entry.path, depth + 1, Some(index), this, rows);
                    }
                    rows[index].subtree_end = rows.len();
                }
            }
        }
        let selected = self
            .selected
            .and_then(|i| self.rows.get(i))
            .map(|r| r.entry.path.clone());
        let mut rows = Vec::new();
        append(&self.root, 0, None, self, &mut rows);
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

    fn render_row(&self, index: usize, sticky: bool, cx: &mut Context<Self>) -> AnyElement {
        let row = &self.rows[index];
        let marker = if self.tasks.contains_key(&row.entry.path) {
            "…"
        } else if self.expanded.contains(&row.entry.path) {
            "▾"
        } else {
            "▸"
        };
        let name = row
            .entry
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let menu_path = row.entry.path.clone();
        let menu_root = self.root.clone();
        div()
            .id((if sticky { "sticky-row" } else { "tree-row" }, index))
            .relative()
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, _, window, cx| {
                    view.selected = Some(index);
                    window.focus(&view.focus, cx);
                    cx.notify();
                }),
            )
            .h(px(ROW_HEIGHT))
            .w_full()
            .bg(if self.selected == Some(index) {
                cx.theme().accent
            } else {
                cx.theme().sidebar
            })
            .when(sticky, |row| row.occlude())
            .children((0..row.depth).map(|level| {
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(18. + level as f32 * 14.))
                    .w(px(1.))
                    .bg(cx.theme().border)
            }))
            .pl(px(8. + row.depth as f32 * 14.))
            .pr_2()
            .child(
                Button::new((if sticky { "sticky-entry" } else { "entry" }, index))
                    .custom(ButtonCustomVariant::new(cx))
                    .focus_ring(false)
                    .accessibility_label(format!(
                        "{}  {name}",
                        if row.entry.is_dir { marker } else { "·" }
                    ))
                    .child(
                        div().w_full().text_left().truncate().child(
                            gpui_kit::component::h_flex()
                                .gap_2()
                                .child(if row.entry.is_dir {
                                    div().child(marker).into_any_element()
                                } else {
                                    super::file_icons::file_icon(&row.entry.path).into_any_element()
                                })
                                .child(name),
                        ),
                    )
                    .w_full()
                    .h(px(ROW_HEIGHT))
                    .justify_start()
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.activate(index, window, cx);
                        if sticky {
                            view.scroll
                                .scroll_to_item_strict(index, ScrollStrategy::Top);
                        }
                    })),
            )
            .context_menu(move |menu, _, _| path_menu(menu, &menu_path, &menu_root))
            .into_any_element()
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
        let view = cx.entity().downgrade();
        gpui_kit::component::v_flex()
            .relative()
            .overflow_hidden()
            .size_full()
            .min_h_0()
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
                            .map(|index| view.render_row(index, false, cx))
                            .collect()
                    }),
                )
                .w_full()
                .flex_1()
                .min_h_0()
                .track_scroll(&self.scroll),
            )
            .child(
                canvas(
                    move |bounds, window, cx| {
                        // Layout the pinned rows after the list, using this frame's actual
                        // scroll position. They overlay the list without changing its height.
                        let mut pinned = view
                            .update(cx, |view, cx| {
                                let offset =
                                    -f32::from(view.scroll.0.borrow().base_handle.offset().y);
                                sticky_rows(&view.rows, offset, f32::from(bounds.size.height))
                                    .into_iter()
                                    .map(|(index, y)| (y, view.render_row(index, true, cx)))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        for (y, element) in &mut pinned {
                            element.prepaint_as_root(
                                bounds.origin + point(px(0.), px(*y)),
                                size(bounds.size.width, px(ROW_HEIGHT)).into(),
                                window,
                                cx,
                            );
                        }
                        pinned
                    },
                    |_, pinned, window, cx| {
                        // A departing child folder slides behind its pinned parent.
                        for (_, mut element) in pinned.into_iter().rev() {
                            element.paint(window, cx);
                        }
                    },
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
    }
}

/// Menu actions operate on the row's path, independent of the open document.
pub(super) fn path_menu(
    menu: PopupMenu,
    path: &std::path::Path,
    root: &std::path::Path,
) -> PopupMenu {
    let reveal = path.to_path_buf();
    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative = if relative.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        relative.to_string_lossy().into_owned()
    };
    let absolute = path.to_string_lossy().into_owned();
    menu.item(
        PopupMenuItem::new("Reveal in Finder").on_click(move |_, _, cx| cx.reveal_path(&reveal)),
    )
    .separator()
    .item(
        PopupMenuItem::new("Copy Relative Path").on_click(move |_, _, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(relative.clone()))
        }),
    )
    .item(
        PopupMenuItem::new("Copy Absolute Path").on_click(move |_, _, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(absolute.clone()))
        }),
    )
}

const ROW_HEIGHT: f32 = 32.;

/// Pin only real expanded ancestor rows that have crossed their tree slot.
/// A sibling subtree pushes the previous ancestor out, rather than inheriting it.
fn sticky_rows(rows: &[Row], offset: f32, height: f32) -> Vec<(usize, f32)> {
    let offset = offset.max(0.);
    let top = (offset / ROW_HEIGHT).floor() as usize;
    let mut pinned = Vec::new();
    let capacity = ((height / ROW_HEIGHT) as usize).saturating_sub(1);
    for slot in 0..capacity {
        let slot_y = slot as f32 * ROW_HEIGHT;
        let probe = top.saturating_add(((offset % ROW_HEIGHT + slot_y) / ROW_HEIGHT) as usize);
        let Some(row) = rows.get(probe) else {
            break;
        };
        let mut ancestors = Vec::new();
        let mut current = if row.entry.is_dir && row.subtree_end > probe + 1 {
            Some(probe)
        } else {
            row.parent
        };
        while let Some(index) = current {
            ancestors.push(index);
            current = rows[index].parent;
        }
        ancestors.reverse();
        if pinned
            .iter()
            .enumerate()
            .any(|(level, (index, _))| ancestors.get(level) != Some(index))
        {
            break;
        }
        let Some(&index) = ancestors.get(slot) else {
            break;
        };
        let natural_y = index as f32 * ROW_HEIGHT - offset;
        if natural_y >= slot_y {
            break;
        }
        let end_y = rows[index].subtree_end as f32 * ROW_HEIGHT - offset;
        if end_y <= slot_y {
            break;
        }
        pinned.push((index, slot_y.min(end_y - ROW_HEIGHT)));
    }
    pinned
}

#[cfg(test)]
mod tests {
    use super::{Row, sticky_rows};
    use glimpse_core::DirectoryEntry;

    fn row(name: &str, parent: Option<usize>, depth: usize, end: usize, is_dir: bool) -> Row {
        Row {
            entry: DirectoryEntry {
                path: name.into(),
                is_dir,
                is_symlink: false,
            },
            parent,
            depth,
            subtree_end: end,
        }
    }
    fn tree() -> Vec<Row> {
        vec![
            row("a", None, 0, 8, true),
            row("a/b", Some(0), 1, 6, true),
            row("a/b/c", Some(1), 2, 6, true),
            row("a/b/c/one", Some(2), 3, 4, false),
            row("a/b/c/two", Some(2), 3, 5, false),
            row("a/b/c/three", Some(2), 3, 6, false),
            row("a/z", Some(0), 1, 8, true),
            row("a/z/file", Some(6), 2, 8, false),
            row("other", None, 0, 10, true),
            row("other/file", Some(8), 1, 10, false),
        ]
    }
    #[test]
    fn ancestors_pin_only_after_scrolling_and_keep_tree_order() {
        let rows = tree();
        assert!(sticky_rows(&rows, 0., 320.).is_empty());
        assert_eq!(sticky_rows(&rows, 8., 320.), [(0, 0.), (1, 32.), (2, 64.)]);
        assert_eq!(sticky_rows(&rows, 96., 320.), [(0, 0.), (1, 32.), (2, 64.)]);
        assert!(sticky_rows(&rows, 96., 32.).is_empty());
    }
    #[test]
    fn sibling_branches_push_out_old_ancestors() {
        let rows = tree();
        assert_eq!(
            sticky_rows(&rows, 120., 320.),
            [(0, 0.), (1, 32.), (2, 40.)]
        );
        assert_eq!(sticky_rows(&rows, 176., 320.), [(0, 0.), (6, 32.)]);
        assert_eq!(sticky_rows(&rows, 240., 320.), [(0, -16.)]);
        assert!(sticky_rows(&rows, 256., 320.).is_empty());
        assert_eq!(sticky_rows(&rows, 264., 320.), [(8, 0.)]);
    }
    #[test]
    fn collapsed_or_empty_folders_do_not_stick() {
        let rows = vec![
            row("empty", None, 0, 1, true),
            row("next", None, 0, 2, false),
        ];
        assert!(sticky_rows(&rows, 8., 320.).is_empty());
        assert!(sticky_rows(&[], 100., 320.).is_empty());
    }
}
