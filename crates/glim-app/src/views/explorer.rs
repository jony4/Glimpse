use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use glim_core::DirectoryEntry;
use glim_services::workspace::list_directory;
use gpui_kit::{
    base::TestSupportExt,
    component::{
        ActiveTheme, FocusableExt, Icon,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    },
    prelude::FluentBuilder,
    *,
};

pub enum ExplorerEvent {
    OpenFile(PathBuf),
    PlayFolder(PathBuf, bool),
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
    roots: Vec<PathBuf>,
    directories: HashMap<PathBuf, Vec<DirectoryEntry>>,
    expanded: HashSet<PathBuf>,
    tasks: HashMap<PathBuf, Task<()>>,
    media_presence: HashMap<PathBuf, bool>,
    media_tasks: HashMap<PathBuf, Task<()>>,
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
            roots: vec![root.clone()],
            directories: HashMap::from([(root.clone(), entries)]),
            expanded: HashSet::from([root]),
            tasks: HashMap::new(),
            media_presence: HashMap::new(),
            media_tasks: HashMap::new(),
            rows: Vec::new(),
            selected: None,
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
        };
        view.rebuild();
        view
    }

    pub fn add_folder(
        &mut self,
        root: PathBuf,
        entries: Vec<DirectoryEntry>,
        cx: &mut Context<Self>,
    ) {
        if !self.roots.iter().any(|existing| root.starts_with(existing)) {
            self.roots.retain(|existing| !existing.starts_with(&root));
            self.roots.push(root.clone());
            self.expanded.insert(root.clone());
            self.directories.insert(root, entries);
            self.rebuild();
            cx.notify();
        }
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.media_tasks.clear();
        self.media_presence.clear();
        let mut paths = self.expanded.iter().cloned().collect::<Vec<_>>();
        paths.extend(self.roots.iter().cloned());
        paths.sort();
        paths.dedup();
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
        for root in &self.roots {
            let index = rows.len();
            rows.push(Row {
                entry: DirectoryEntry {
                    path: root.clone(),
                    is_dir: true,
                    is_symlink: false,
                },
                depth: 0,
                parent: None,
                subtree_end: index + 1,
            });
            if self.expanded.contains(root) {
                append(root, 1, Some(index), self, &mut rows);
            }
            rows[index].subtree_end = rows.len();
        }
        self.selected = selected.and_then(|path| rows.iter().position(|r| r.entry.path == path));
        self.rows = rows;
    }

    fn check_folder_media(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !cfg!(target_os = "macos") {
            return;
        }
        if self.media_presence.contains_key(&path) || self.media_tasks.contains_key(&path) {
            return;
        }
        let read_path = path.clone();
        let read = cx.background_executor().spawn(async move {
            glim_services::preview::folder_has_media(&read_path).unwrap_or(false)
        });
        let key = path.clone();
        let task = cx.spawn(async move |view, cx| {
            let present = read.await;
            let _ = view.update(cx, |v, cx| {
                v.media_tasks.remove(&path);
                v.media_presence.insert(path, present);
                cx.notify();
            });
        });
        self.media_tasks.insert(key, task);
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

    fn activate_path(
        &mut self,
        path: &std::path::Path,
        sticky: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.rows.iter().position(|row| row.entry.path == path) else {
            return;
        };
        let depth = self.rows[index].depth;
        self.activate(index, window, cx);
        if sticky {
            self.scroll
                .scroll_to_item_strict_with_offset(index, ScrollStrategy::Top, depth);
        }
    }

    fn render_row(&self, index: usize, sticky: bool, cx: &mut Context<Self>) -> AnyElement {
        let row = &self.rows[index];
        let expanded = self.expanded.contains(&row.entry.path);
        let name = row
            .entry
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let row_path = row.entry.path.clone();
        let right_click_path = row_path.clone();
        let menu_path = row.entry.path.clone();
        let menu_is_dir = row.entry.is_dir;
        let menu_view = cx.entity().downgrade();
        let hover_path = menu_path.clone();

        let menu_root = self
            .roots
            .iter()
            .filter(|root| row.entry.path.starts_with(root))
            .max_by_key(|root| root.components().count())
            .cloned()
            .unwrap_or_default();
        let entry = div()
            .id(entry_id(
                &row.entry.path,
                if sticky { "sticky-row" } else { "tree-row" },
            ))
            .relative()
            .on_hover(cx.listener(move |view, hovered: &bool, _, cx| {
                if *hovered && menu_is_dir {
                    view.check_folder_media(hover_path.clone(), cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, _, window, cx| {
                    if menu_is_dir {
                        view.check_folder_media(right_click_path.clone(), cx);
                    }
                    view.selected = view
                        .rows
                        .iter()
                        .position(|r| r.entry.path == right_click_path);
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
                    .left(px(26. + level as f32 * 14.))
                    .w(px(1.))
                    .bg(cx.theme().border)
            }))
            .pl(px(8. + row.depth as f32 * 14.))
            .pr_2()
            .child(
                Button::new(entry_id(
                    &row.entry.path,
                    if sticky { "sticky-entry" } else { "entry" },
                ))
                .custom(ButtonCustomVariant::new(cx))
                .focus_ring(false)
                .accessibility_label(format!(
                    "{}  {name}",
                    if !row.entry.is_dir {
                        "File"
                    } else if self.tasks.contains_key(&row.entry.path) {
                        "Loading folder"
                    } else if expanded {
                        "Expanded folder"
                    } else {
                        "Collapsed folder"
                    }
                ))
                .child(
                    div().w_full().text_left().truncate().child(
                        gpui_kit::component::h_flex()
                            .w_full()
                            .min_w_0()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(16.))
                                    .h(px(16.))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(if row.entry.is_dir {
                                        // Keep the same icon box during loading; replacing
                                        // a font glyph with an ellipsis makes it flicker.
                                        Icon::new(if expanded {
                                            gpui_kit::assets::IconName::ChevronDown
                                        } else {
                                            gpui_kit::assets::IconName::ChevronRight
                                        })
                                        .size(px(14.))
                                        .flex_shrink_0()
                                        .text_color(cx.theme().muted_foreground)
                                        .into_any_element()
                                    } else {
                                        super::file_icons::file_icon(&row.entry.path)
                                            .into_any_element()
                                    }),
                            )
                            .child(
                                div()
                                    .id(entry_id(
                                        &row.entry.path,
                                        if sticky { "sticky-label" } else { "label" },
                                    ))
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .child(name)
                                    .test_support(),
                            ),
                    ),
                )
                .w_full()
                .h(px(ROW_HEIGHT))
                .justify_start()
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.activate_path(&row_path, sticky, window, cx);
                })),
            )
            .context_menu(move |mut menu, _, cx| {
                let has_media = menu_view.upgrade().is_some_and(|view| {
                    view.read(cx).media_presence.get(&menu_path).copied() == Some(true)
                });
                if menu_is_dir && has_media {
                    for (label, shuffle) in [("Play in Order", false), ("Shuffle Play", true)] {
                        let path = menu_path.clone();
                        let view = menu_view.clone();
                        menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                            let _ = view.update(cx, |_, cx| {
                                cx.emit(ExplorerEvent::PlayFolder(path.clone(), shuffle))
                            });
                        }));
                    }
                    menu = menu.separator();
                } else if !menu_is_dir && glim_services::preview::supports(&menu_path) {
                    let path = menu_path.clone();
                    let view = menu_view.clone();
                    menu = menu
                        .item(PopupMenuItem::new("Open Preview / Play").on_click(
                            move |_, _, cx| {
                                let _ = view.update(cx, |_, cx| {
                                    cx.emit(ExplorerEvent::OpenFile(path.clone()))
                                });
                            },
                        ))
                        .separator();
                }
                path_menu(menu, &menu_path, &menu_root)
            });
        div()
            .w_full()
            .child(entry)
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
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
                        let (mut pinned, blank_top) = view
                            .update(cx, |view, cx| {
                                let offset =
                                    -f32::from(view.scroll.0.borrow().base_handle.offset().y);
                                let blank_top =
                                    (view.rows.len() as f32 * ROW_HEIGHT - offset).max(0.);
                                let pinned =
                                    sticky_rows(&view.rows, offset, f32::from(bounds.size.height))
                                        .into_iter()
                                        .map(|(index, y)| (y, view.render_row(index, true, cx)))
                                        .collect::<Vec<_>>();
                                (pinned, blank_top)
                            })
                            .unwrap_or_default();
                        // A separate hit area below the final row: never an ancestor
                        // of file/folder menus, so both menus cannot open together.
                        let mut blank = if blank_top < f32::from(bounds.size.height) {
                            let mut element = div()
                                .id("files-blank-space")
                                .w(bounds.size.width)
                                .h(bounds.size.height - px(blank_top))
                                .context_menu(|menu, _, _| {
                                    menu.item(
                                        PopupMenuItem::new("Add Folder to Workspace…").on_click(
                                            |_, window, cx| {
                                                window.dispatch_action(
                                                    Box::new(crate::app::actions::AddFolder),
                                                    cx,
                                                );
                                            },
                                        ),
                                    )
                                })
                                .into_any_element();
                            element.prepaint_as_root(
                                bounds.origin + point(px(0.), px(blank_top)),
                                size(bounds.size.width, bounds.size.height - px(blank_top)).into(),
                                window,
                                cx,
                            );
                            Some(element)
                        } else {
                            None
                        };
                        for (y, element) in &mut pinned {
                            element.prepaint_as_root(
                                bounds.origin + point(px(0.), px(*y)),
                                size(bounds.size.width, px(ROW_HEIGHT)).into(),
                                window,
                                cx,
                            );
                        }
                        (pinned, blank.take())
                    },
                    |_, (pinned, blank), window, cx| {
                        if let Some(mut blank) = blank {
                            blank.paint(window, cx);
                        }
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
        PopupMenuItem::new(if cfg!(target_os = "macos") {
            "Reveal in Finder"
        } else {
            "Reveal in File Explorer"
        })
        .on_click(move |_, _, cx| cx.reveal_path(&reveal)),
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

fn entry_id(path: &std::path::Path, part: &'static str) -> ElementId {
    ElementId::NamedChild(
        std::sync::Arc::new(ElementId::Path(path.into())),
        part.into(),
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
    use glim_core::DirectoryEntry;

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

#[cfg(test)]
mod interaction_tests {
    use super::{Explorer, entry_id};
    use glim_core::DirectoryEntry;
    use gpui_kit::{
        AppContext, Bounds, Point, TestAppContext, WindowBounds, WindowOptions, px, size,
        test::TestWindowExt,
    };

    #[gpui_kit::test]
    fn loading_arrow_and_expanded_arrow_keep_label_in_place(cx: &mut TestAppContext) {
        let path = std::path::PathBuf::from("/fixture/folder");
        cx.update(gpui_kit::init);
        let (handle, view) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(400.), px(500.)),
                    })),
                    ..Default::default()
                },
                cx,
                |_, cx| {
                    cx.new(|cx| {
                        Explorer::new(
                            "/fixture".into(),
                            vec![DirectoryEntry {
                                path: path.clone(),
                                is_dir: true,
                                is_symlink: false,
                            }],
                            cx,
                        )
                    })
                },
            )
            .unwrap()
        });
        let before = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window.find(entry_id(&path, "label")).bounds()
            })
            .unwrap();
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.tasks.insert(
                    path.clone(),
                    cx.spawn(async |_, _| std::future::pending::<()>().await),
                );
                v.expanded.insert(path.clone());
                cx.notify();
            })
        });
        let loading = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window.find(entry_id(&path, "label")).bounds()
            })
            .unwrap();
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.tasks.clear();
                cx.notify();
            })
        });
        let expanded = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window.find(entry_id(&path, "label")).bounds()
            })
            .unwrap();
        assert_eq!(
            before, loading,
            "loading marker must not shift the filename"
        );
        assert_eq!(
            before, expanded,
            "disclosure state must not shift the filename"
        );
    }
    #[gpui_kit::test]
    fn clicking_sticky_folder_keeps_it_below_pinned_parent(cx: &mut TestAppContext) {
        let parent = std::path::PathBuf::from("/fixture/parent");
        let branch = parent.join("branch");
        let file = |path| DirectoryEntry {
            path,
            is_dir: false,
            is_symlink: false,
        };
        cx.update(gpui_kit::init);
        let (handle, view) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(400.), px(500.)),
                    })),
                    ..Default::default()
                },
                cx,
                |_, cx| {
                    cx.new(|cx| {
                        let mut view = Explorer::new(
                            "/fixture".into(),
                            vec![DirectoryEntry {
                                path: parent.clone(),
                                is_dir: true,
                                is_symlink: false,
                            }],
                            cx,
                        );
                        let mut siblings = (0..30)
                            .map(|i| file(parent.join(format!("before-{i}"))))
                            .collect::<Vec<_>>();
                        siblings.push(DirectoryEntry {
                            path: branch.clone(),
                            is_dir: true,
                            is_symlink: false,
                        });
                        siblings.extend((0..100).map(|i| file(parent.join(format!("after-{i}")))));
                        view.directories.insert(parent.clone(), siblings);
                        view.directories.insert(
                            branch.clone(),
                            (0..10)
                                .map(|i| file(branch.join(format!("file-{i}"))))
                                .collect(),
                        );
                        view.expanded.extend([parent.clone(), branch.clone()]);
                        view.rebuild();
                        view
                    })
                },
            )
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.scroll
                    .0
                    .borrow()
                    .base_handle
                    .set_offset(gpui_kit::point(px(0.), px(-1088.)));
                cx.notify();
            })
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let before = window.find(entry_id(&branch, "sticky-label")).bounds();
            window.click(entry_id(&branch, "sticky-entry"), cx);
            window.render_frame(cx);
            let after = window.find(entry_id(&branch, "label")).bounds();
            assert_eq!(
                before.origin, after.origin,
                "collapse must preserve the clicked folder's visual slot"
            );
        })
        .unwrap();
        assert!(!cx.update(|cx| view.read(cx).expanded.contains(&branch)));
    }

    #[gpui_kit::test]
    fn repeated_cached_expansion_keeps_clicked_label_fixed(cx: &mut TestAppContext) {
        let parent = std::path::PathBuf::from("/fixture/folder");
        cx.update(gpui_kit::init);
        let (handle, view) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(400.), px(500.)),
                    })),
                    ..Default::default()
                },
                cx,
                |_, cx| {
                    cx.new(|cx| {
                        let mut view = Explorer::new(
                            "/fixture".into(),
                            vec![DirectoryEntry {
                                path: parent.clone(),
                                is_dir: true,
                                is_symlink: false,
                            }],
                            cx,
                        );
                        view.directories.insert(
                            parent.clone(),
                            (0..30)
                                .map(|i| DirectoryEntry {
                                    path: parent.join(format!("file-{i}")),
                                    is_dir: false,
                                    is_symlink: false,
                                })
                                .collect(),
                        );
                        view
                    })
                },
            )
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let original = window.find(entry_id(&parent, "label")).bounds();
            for _ in 0..20 {
                window.click(entry_id(&parent, "entry"), cx);
                window.render_frame(cx);
                assert_eq!(window.find(entry_id(&parent, "label")).bounds(), original);
            }
        })
        .unwrap();
        assert!(!cx.update(|cx| view.read(cx).expanded.contains(&parent)));
    }
}
