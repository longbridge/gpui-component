#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod app;
mod ui;
mod models;

fn main() {
    app::run();
}
