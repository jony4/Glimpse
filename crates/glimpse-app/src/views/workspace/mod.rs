mod clone;
mod header;
mod loading;
mod render;
mod tabs;
mod updates;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use std::path::PathBuf;

use super::{changes::Changes, explorer::Explorer, reader::Reader};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sidebar {
    Files,
    Changes,
}

pub struct Workspace {
    focus: FocusHandle,
    history: Vec<header::Target>,
    history_cursor: Option<usize>,
    navigating: bool,
    search: Entity<InputState>,
    clone_url: Entity<InputState>,
    results: Vec<PathBuf>,
    search_task: Option<Task<()>>,
    _search_subscription: Subscription,
    watch: Option<glimpse_services::watch::WorkspaceWatch>,
    watch_task: Option<Task<()>>,
    watch_setup: Option<Task<()>>,
    refresh_task: Option<Task<()>>,
    root: Option<PathBuf>,
    explorer: Option<Entity<Explorer>>,
    changes: Option<Entity<Changes>>,
    subscriptions: Vec<Subscription>,
    sidebar: Sidebar,
    tabs: Vec<Reader>,
    active: Option<usize>,
    tab_scroll: ScrollHandle,
    error: Option<SharedString>,
    loading: bool,
    load_task: Option<Task<()>>,
    picker_task: Option<Task<()>>,
}

impl Workspace {
    pub fn new(path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search files in workspace…"));
        let subscription = cx.subscribe_in(&search, window, |view, _, event, window, cx| {
            if matches!(event, InputEvent::Change) {
                view.search_files(window, cx);
            }
        });
        let mut workspace = Self {
            history: Vec::new(),
            history_cursor: None,
            navigating: false,
            clone_url: cx
                .new(|cx| InputState::new(window, cx).placeholder("https://github.com/owner/repo")),
            search,
            results: Vec::new(),
            search_task: None,
            _search_subscription: subscription,
            watch: None,
            watch_task: None,
            watch_setup: None,
            refresh_task: None,
            focus,
            root: None,
            explorer: None,
            changes: None,
            subscriptions: Vec::new(),
            sidebar: Sidebar::Files,
            tabs: Vec::new(),
            active: None,
            tab_scroll: ScrollHandle::new(),
            error: None,
            loading: false,
            load_task: None,
            picker_task: None,
        };
        if let Some(path) = path {
            workspace.open_path(path, window, cx);
        }
        workspace
    }
}
