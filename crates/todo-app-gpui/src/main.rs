#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod app;
mod backoffice;
mod models;
mod ui;
pub mod xbus;

#[actix::main]
async fn main() -> anyhow::Result<()> {
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
    backoffice::start();
    log::info!("Starting Todo App GPUI...");
    app::run();
    Ok(())
}
