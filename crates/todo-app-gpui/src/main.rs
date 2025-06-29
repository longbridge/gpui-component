#![cfg_attr(all(target_os = "windows"), windows_subsystem = "windows")]
mod app;
mod backoffice;
mod models;
mod ui;
pub mod xbus;

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt()
    //     .with_env_filter(
    //         tracing_subscriber::EnvFilter::from_default_env()
    //             .add_directive(tracing::Level::ERROR.into()),
    //     )
    //     .with_writer(std::io::stderr)
    //     .with_ansi(true)
    //     .with_line_number(true)
    //     .with_file(true)
    //     .init();
    app::run();
}
