pub mod actions;
mod assets;
pub(crate) mod languages;

use anyhow::{Result, bail};
use gpui_kit::*;

use crate::views::workspace::Workspace;
use actions::{
    AddFolder, CloseWindow, DefaultApplications, NewWindow, OpenFile, OpenFolder, Quit, Refresh,
    SaveFile,
};

pub struct ReaderPreferences {
    pub word_wrap: bool,
    pub html_preview: bool,
    pub markdown_preview: bool,
    pub preview_save_task: Option<Task<()>>,
}
impl Global for ReaderPreferences {}

#[derive(Default)]
struct DefaultApplicationsTask(Option<Task<()>>);
impl Global for DefaultApplicationsTask {}

fn open_default_applications(cx: &mut App) {
    if cx.global::<DefaultApplicationsTask>().0.is_some() {
        return;
    }
    let work = cx.background_executor().spawn(async {
        glim_services::preview::open_associations(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        ))
    });
    let task = cx.spawn(async move |cx| {
        let result = work.await;
        cx.update(|cx| {
            cx.global_mut::<DefaultApplicationsTask>().0 = None;
            if let Err(error) = result
                && let Some((_, handle)) = cx.global::<OpenWorkspaces>().0.last().cloned()
            {
                let _ = handle.update(cx, |_, window, cx| {
                    drop(window.prompt(
                        PromptLevel::Warning,
                        "Default applications",
                        Some(&format!("{error:#}")),
                        &["OK"],
                        cx,
                    ));
                });
            }
        });
    });
    cx.global_mut::<DefaultApplicationsTask>().0 = Some(task);
}

pub fn run() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next();
    if arguments.next().is_some() {
        bail!("Usage: glim [file-or-folder]");
    }
    if path.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!(
            "Usage: glim [file-or-folder]\n\nOpen a folder, Git repository, or UTF-8 file in the viewer and basic text editor."
        );
        return Ok(());
    }

    let word_wrap = glim_services::preferences::word_wrap();
    let html_preview = glim_services::preferences::preview_mode("html");
    let markdown_preview = glim_services::preferences::preview_mode("markdown");
    let application = gpui_kit::application().with_assets(assets::Assets);
    let context = std::rc::Rc::new(std::cell::RefCell::new(None::<AsyncApp>));
    let pending = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let open_context = context.clone();
    let open_pending = pending.clone();
    application.on_open_urls(move |urls| {
        if let Some(app) = open_context.borrow().clone() {
            app.clone()
                .foreground_executor()
                .spawn(async move {
                    app.update(|cx| open_system_files(urls, cx));
                })
                .detach();
        } else {
            open_pending.borrow_mut().extend(urls);
        }
    });
    application.run(move |cx| {
        gpui_kit::init(cx);
        languages::init();
        cx.set_global(ReaderPreferences {
            word_wrap,
            html_preview,
            markdown_preview,
            preview_save_task: None,
        });
        cx.set_global(OpenWorkspaces::default());
        cx.set_global(DefaultApplicationsTask::default());
        *context.borrow_mut() = Some(cx.to_async());
        gpui_kit::component::Theme::set_scrollbar_mode(
            gpui_kit::component::scroll::ScrollbarMode::Hover,
            cx,
        );
        crate::views::explorer::Explorer::init(cx);
        cx.on_action(|_: &Quit, cx| request_quit(cx));
        cx.on_action(|_: &NewWindow, cx| open_workspace(None, cx));
        cx.on_action(|_: &DefaultApplications, cx| open_default_applications(cx));
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-shift-n", NewWindow, None),
            KeyBinding::new("cmd-o", OpenFile, None),
            KeyBinding::new("cmd-s", SaveFile, None),
            KeyBinding::new("cmd-shift-o", OpenFolder, None),
            KeyBinding::new("cmd-r", Refresh, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
        ]);
        cx.set_dock_menu(vec![MenuItem::action("New Window", NewWindow)]);
        cx.set_menus(vec![
            Menu {
                name: "Glim".into(),
                disabled: false,
                items: vec![
                    MenuItem::action("Default File Types…", DefaultApplications),
                    MenuItem::separator(),
                    MenuItem::action("Quit Glim", Quit),
                ],
            },
            Menu {
                name: "File".into(),
                disabled: false,
                items: vec![
                    MenuItem::action("New Window", NewWindow),
                    MenuItem::action("Open File…", OpenFile),
                    MenuItem::action("Open Folder…", OpenFolder),
                    MenuItem::action("Add Folder to Workspace…", AddFolder),
                    MenuItem::action("Save", SaveFile),
                    MenuItem::action("Refresh", Refresh),
                    MenuItem::action("Close Tab / Window", CloseWindow),
                ],
            },
        ]);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let requested = std::mem::take(&mut *pending.borrow_mut());
        if requested.is_empty() {
            open_workspace(path.map(Into::into), cx);
        } else {
            open_system_files(requested, cx);
        }
    });
    Ok(())
}

pub(crate) fn reveal_existing_preview(
    path: &std::path::Path,
    exclude: Option<EntityId>,
    cx: &mut App,
) -> bool {
    let Some(windows) = cx.try_global::<OpenWorkspaces>().map(|v| v.0.clone()) else {
        return false;
    };
    windows
        .into_iter()
        .filter(|(view, _)| Some(view.entity_id()) != exclude)
        .any(|(view, _)| {
            view.update(cx, |v, cx| v.reveal_native_path(path, cx))
                .unwrap_or(false)
        })
}

fn open_system_files(urls: Vec<String>, cx: &mut App) {
    for value in urls {
        if let Ok(url) = url::Url::parse(&value)
            && url.scheme() == "file"
            && let Ok(path) = url.to_file_path()
        {
            if reveal_existing_preview(&path, None, cx) {
                continue;
            }
            let windows = cx.global::<OpenWorkspaces>().0.clone();
            let reused = windows.iter().rev().any(|(view, handle)| {
                handle
                    .update(cx, |_, window, cx| {
                        view.update(cx, |v, cx| {
                            v.open_system_file_if_empty(path.clone(), window, cx)
                        })
                        .unwrap_or(false)
                    })
                    .unwrap_or(false)
            });
            if reused {
                cx.activate(true);
            } else {
                open_workspace(Some(path), cx);
            }
        }
    }
}

fn open_workspace(path: Option<std::path::PathBuf>, cx: &mut App) {
    // Use the usable screen rectangle directly. Maximized invokes macOS zoom,
    // which toggles an already full-size initial frame back to a smaller one.
    let bounds = cx
        .primary_display()
        .map(|display| display.visible_bounds())
        .unwrap_or_else(|| Bounds::centered(None, size(px(1100.), px(760.)), cx));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(800.), px(480.))),
        ..gpui_kit::component::TitleBar::window_options()
    };
    match gpui_kit::open_window(options, cx, |window, cx| {
        let workspace = cx.new(|cx| Workspace::new(path, window, cx));
        let windows = &mut cx.global_mut::<OpenWorkspaces>().0;
        windows.retain(|(view, _)| view.upgrade().is_some());
        windows.push((workspace.downgrade(), window.window_handle()));
        workspace
    }) {
        Ok(_) => cx.activate(true),
        Err(error) => eprintln!("Cannot open Glim window: {error:#}"),
    }
}

#[derive(Default)]
struct OpenWorkspaces(Vec<(WeakEntity<Workspace>, AnyWindowHandle)>);
impl Global for OpenWorkspaces {}

pub(crate) fn any_other_saving(exclude: EntityId, cx: &App) -> bool {
    cx.global::<OpenWorkspaces>().0.iter().any(|(v, _)| {
        v.upgrade()
            .is_some_and(|v| v.entity_id() != exclude && v.read(cx).is_saving())
    })
}
fn request_quit(cx: &mut App) {
    let windows = cx.global::<OpenWorkspaces>().0.clone();
    let target = windows
        .iter()
        .find(|(v, _)| v.upgrade().is_some_and(|v| v.read(cx).is_saving()))
        .or_else(|| {
            windows
                .iter()
                .find(|(v, _)| v.upgrade().is_some_and(|v| v.read(cx).has_unsaved()))
        });
    if let Some((view, handle)) = target {
        let _ = handle.update(cx, |_, window, cx| {
            let _ = view.update(cx, |v, cx| v.request_quit(window, cx));
        });
    } else {
        cx.quit();
    }
}

#[derive(Default)]
struct GitEditorLock(bool);
impl Global for GitEditorLock {}
pub(crate) fn git_editors_locked(cx: &App) -> bool {
    cx.try_global::<GitEditorLock>().is_some_and(|lock| lock.0)
}
pub(crate) fn lock_git_editors(exclude: EntityId, cx: &mut App) -> Result<()> {
    anyhow::ensure!(
        !git_editors_locked(cx),
        "Another Git operation is updating the working tree"
    );
    let windows = cx
        .try_global::<OpenWorkspaces>()
        .map(|w| w.0.clone())
        .unwrap_or_default();
    let others = windows
        .into_iter()
        .filter_map(|(v, _)| v.upgrade())
        .filter(|v| v.entity_id() != exclude)
        .collect::<Vec<_>>();
    anyhow::ensure!(
        others.iter().all(|v| !v.read(cx).has_unsaved()
            && !v.read(cx).is_saving()
            && !v.read(cx).has_running_git(cx)),
        "Another window has unsaved changes or an active operation. Save drafts and wait before updating the working tree."
    );
    cx.set_global(GitEditorLock(true));
    for view in others {
        view.update(cx, |v, cx| v.set_git_editing_locked(true, cx));
    }
    Ok(())
}
pub(crate) fn unlock_git_editors(exclude: EntityId, cx: &mut App) {
    cx.set_global(GitEditorLock(false));
    let windows = cx
        .try_global::<OpenWorkspaces>()
        .map(|w| w.0.clone())
        .unwrap_or_default();
    for (view, handle) in windows {
        if let Some(view) = view.upgrade()
            && view.entity_id() != exclude
        {
            let _ = handle.update(cx, |_, window, cx| {
                view.update(cx, |v, cx| v.unlock_git_and_refresh(window, cx));
            });
        }
    }
}
