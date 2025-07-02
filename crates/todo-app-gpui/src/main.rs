#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod backoffice;
mod config;
pub mod ebus;
mod ui;
pub mod xbus;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    backoffice::start()?;
    app::run();
    Ok(())
}
