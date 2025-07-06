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

 rust_i18n::i18n!("locales", fallback = "en");
 
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::sync::OnceLock; 
        static MUTEX: OnceLock<mutex::Mutex> = OnceLock::new();
        let mutex = mutex::Mutex::try_lock("x-todo-app", true)?;
        MUTEX.set(mutex).ok();
    }
    backoffice::start()?;
    app::run();
    Ok(())
}
