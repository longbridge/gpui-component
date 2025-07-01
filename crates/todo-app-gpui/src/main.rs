#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::time::Duration;

use crate::backoffice::mcp::{GetServerInstance, McpRegistry};
mod app;
mod backoffice;
pub mod ebus;
mod models;
mod ui;
pub mod xbus;

fn main() -> anyhow::Result<()> {
    // #[cfg(debug_assertions)]
    // let _guard = ftlog::builder()
    //     .max_log_level(log::LevelFilter::Info)
    //     .try_init()
    //     .map_err(|err| anyhow::anyhow!("{}", err))?;
    // #[cfg(not(debug_assertions))]
    // let _guard = ftlog::builder()
    //     .max_log_level(log::LevelFilter::Info)
    //     .try_init()
    //     .map_err(|err| anyhow::anyhow!("{}", err))?;
    //let _sys = actix::System::new();

    backoffice::start()?;
    app::run();
    Ok(())
}
