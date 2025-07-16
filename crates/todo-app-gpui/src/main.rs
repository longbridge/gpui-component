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

use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use mimalloc::MiMalloc;
use tracing::level_filters::LevelFilter;
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
    {
        let log_file = std::env::current_exe()?
            .parent()
            .unwrap()
            .join("todo_app_gpui.log");
        let file = std::fs::File::create(&log_file)?;
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive(LevelFilter::WARN.into())
                    .add_directive("todo_app_gpui=debug".parse()?),
            )
            .with_writer(file)
            .with_ansi(true)
            .with_line_number(true)
            .with_file(true)
            .with_target(true)
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        use std::sync::OnceLock;
        use tracing_appender::non_blocking::WorkerGuard;
        const GUARD: OnceLock<WorkerGuard> = OnceLock::new();
        let (non_blocking, _guard) = tracing_appender::non_blocking(LogWriterForGui);
        GUARD.set(_guard).ok();
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive(LevelFilter::WARN.into())
                    .add_directive("todo_app_gpui=debug".parse()?),
            )
            .with_writer(non_blocking)
            .with_ansi(true)
            .with_line_number(true)
            .with_file(true)
            .with_target(true)
            .init();
    }

    Ok(())
}

#[derive(Debug)]
pub struct LogRecord(pub String);

struct LogWriterForGui;

impl std::io::Write for LogWriterForGui {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        CrossRuntimeBridge::global().emit(LogRecord(
            String::from_utf8_lossy(buf).into_owned().to_string(),
        ));
        let buf_len = buf.len();
        Ok(buf_len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
