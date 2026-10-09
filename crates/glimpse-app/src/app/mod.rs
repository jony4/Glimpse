pub mod actions;
mod assets;

use anyhow::{Result, bail};
use gpui_kit::*;

use crate::views::workspace::Workspace;
use actions::{CloseWindow, OpenFile, OpenFolder, Quit, Refresh};

pub fn run() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next();
    if arguments.next().is_some() {
        bail!("Usage: glimpse [file-or-folder]");
    }
    if path.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!(
            "Usage: glimpse [file-or-folder]\n\nOpen a folder, Git repository, or UTF-8 file in the read-only viewer."
        );
        return Ok(());
    }

    gpui_kit::application()
        .with_assets(assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            crate::views::explorer::Explorer::init(cx);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-o", OpenFile, None),
                KeyBinding::new("cmd-shift-o", OpenFolder, None),
                KeyBinding::new("cmd-r", Refresh, None),
                KeyBinding::new("cmd-w", CloseWindow, None),
            ]);
            cx.set_menus(vec![
                Menu {
                    name: "Glimpse".into(),
                    disabled: false,
                    items: vec![MenuItem::action("Quit Glimpse", Quit)],
                },
                Menu {
                    name: "File".into(),
                    disabled: false,
                    items: vec![
                        MenuItem::action("Open File…", OpenFile),
                        MenuItem::action("Open Folder…", OpenFolder),
                        MenuItem::action("Refresh", Refresh),
                        MenuItem::action("Close Window", CloseWindow),
                    ],
                },
            ]);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1100.), px(760.)),
                    cx,
                ))),
                window_min_size: Some(size(px(800.), px(480.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Glimpse".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            match gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| Workspace::new(path.map(Into::into), window, cx))
            }) {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    eprintln!("Cannot open Glimpse window: {error:#}");
                    cx.quit();
                }
            }
        });
    Ok(())
}
