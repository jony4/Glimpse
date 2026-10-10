mod editing;
mod gestures;
mod git;
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
    history_swipe: Option<gestures::HistorySwipe>,
    search: Entity<InputState>,
    sidebar_visible: bool,
    word_wrap: bool,
    preference_task: Option<Task<()>>,
    repository_task: Option<Task<()>>,
    results: Vec<PathBuf>,
    pending_anchor: Option<(PathBuf, String)>,
    search_task: Option<Task<()>>,
    _search_subscription: Subscription,
    watch: Option<glim_services::watch::WorkspaceWatch>,
    watch_task: Option<Task<()>>,
    watch_setup: Option<Task<()>>,
    refresh_task: Option<Task<()>>,
    root: Option<PathBuf>,
    roots: Vec<PathBuf>,
    explorer: Option<Entity<Explorer>>,
    changes: Option<Entity<Changes>>,
    subscriptions: Vec<Subscription>,
    sidebar: Sidebar,
    tabs: Vec<Reader>,
    active: Option<usize>,
    tab_scroll: ScrollHandle,
    error: Option<SharedString>,
    last_scan_warning: Option<String>,
    loading: bool,
    load_task: Option<Task<()>>,
    picker_task: Option<Task<()>>,
    save_task: Option<Task<()>>,
    saving: Option<EntityId>,
    git_busy: bool,
    git_edit_locked: bool,
    autosave: std::collections::HashMap<EntityId, (Subscription, Option<Task<()>>)>,
    save_queue: std::collections::VecDeque<EntityId>,
    confirm_task: Option<Task<()>>,
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
        let weak = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            weak.update(cx, |v, cx| {
                if v.has_unsaved() || v.is_saving() {
                    v.request_close_window(window, cx);
                    false
                } else {
                    true
                }
            })
            .unwrap_or(true)
        });
        let mut workspace = Self {
            history: Vec::new(),
            history_cursor: None,
            navigating: false,
            history_swipe: None,
            sidebar_visible: false,
            word_wrap: cx
                .try_global::<crate::app::ReaderPreferences>()
                .is_some_and(|p| p.word_wrap),
            preference_task: None,
            repository_task: None,
            search,
            results: Vec::new(),
            pending_anchor: None,
            search_task: None,
            _search_subscription: subscription,
            watch: None,
            watch_task: None,
            watch_setup: None,
            refresh_task: None,
            focus,
            root: None,
            roots: Vec::new(),
            explorer: None,
            changes: None,
            subscriptions: Vec::new(),
            sidebar: Sidebar::Files,
            tabs: Vec::new(),
            active: None,
            tab_scroll: ScrollHandle::new(),
            error: None,
            last_scan_warning: None,
            loading: false,
            load_task: None,
            picker_task: None,
            save_task: None,
            saving: None,
            git_busy: false,
            git_edit_locked: crate::app::git_editors_locked(cx),
            autosave: Default::default(),
            save_queue: Default::default(),
            confirm_task: None,
        };
        if let Some(path) = path {
            workspace.open_path(path, window, cx);
        }
        workspace
    }
}
