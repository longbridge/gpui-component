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
use tracing_subscriber::EnvFilter;

rust_i18n::i18n!("locales", fallback = "en");

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_log()?;
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

fn init_log() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("todo-app-gpui=trace".parse()?),
        )
        .with_max_level(tracing::Level::WARN)
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_line_number(true)
        .with_file(true)
        .with_target(true)
        .init();

    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("todo-app-gpui=info".parse()?))
        .with_max_level(tracing::Level::WARN)
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_line_number(true)
        .with_file(true)
        .with_target(true)
        .init();
    Ok(())
}
