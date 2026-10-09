pub mod actions;

use anyhow::{Result, bail};
use gpui_kit::*;

use crate::views::workspace::Workspace;
use actions::{OpenFile, Quit};

pub fn run() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next();
    if arguments.next().is_some() {
        bail!("Usage: glimpse [file]");
    }
    if path.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!("Usage: glimpse [file]\n\nOpen a UTF-8 file in the read-only viewer.");
        return Ok(());
    }

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-o", OpenFile, None),
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
                    items: vec![MenuItem::action("Open…", OpenFile)],
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
                window_min_size: Some(size(px(640.), px(420.))),
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
