mod loading;
mod render;

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
    root: Option<PathBuf>,
    explorer: Option<Entity<Explorer>>,
    changes: Option<Entity<Changes>>,
    subscriptions: Vec<Subscription>,
    sidebar: Sidebar,
    reader: Option<Reader>,
    error: Option<SharedString>,
    loading: bool,
    load_task: Option<Task<()>>,
    picker_task: Option<Task<()>>,
}

impl Workspace {
    pub fn new(path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let mut workspace = Self {
            focus,
            root: None,
            explorer: None,
            changes: None,
            subscriptions: Vec::new(),
            sidebar: Sidebar::Files,
            reader: None,
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
