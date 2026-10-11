#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod views;

fn main() -> anyhow::Result<()> {
    app::run()
}
