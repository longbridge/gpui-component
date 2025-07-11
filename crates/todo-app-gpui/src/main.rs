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
use std::sync::OnceLock;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
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
    const GUARD: OnceLock<WorkerGuard> = OnceLock::new();
    let logs = std::env::current_exe()?.parent().unwrap().join("logs");
    println!("日志目录: {}", logs.display());
    std::fs::create_dir_all(&logs).ok();

    // let file_appender = tracing_appender::rolling::never(logs, "todo_app_gpui.log");
    // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let file = std::fs::File::create(logs.join("todo_app_gpui.log"))?;
    //GUARD.set(_guard).ok();
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(LevelFilter::WARN.into())
                .add_directive("todo_app_gpui=debug".parse()?),
        )
        //.with_writer(file)
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_line_number(true)
        .with_file(true)
        .with_target(true)
        .init();

    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(LevelFilter::WARN.into())
                .add_directive("todo_app_gpui=info".parse()?),
        )
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_line_number(true)
        .with_file(true)
        .with_target(true)
        .init();
    Ok(())
}
