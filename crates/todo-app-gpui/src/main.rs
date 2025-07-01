#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod backoffice;
pub mod ebus;
mod models;
mod ui;
pub mod xbus;

fn main() -> anyhow::Result<()> {
    backoffice::start()?;
    app::run();
    Ok(())
}
