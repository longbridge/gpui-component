#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod backoffice;
mod config;
pub mod ebus;
#[cfg(target_os = "windows")]
mod mutex;
mod ui;
pub mod xbus;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        mutex::Mutex::try_lock("x-todo-app", true)?;
    }
    backoffice::start()?;
    app::run();
    Ok(())
}
