//! Kit-enabled runner for `nsh package --target gpui`.
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    gpui_kit_nsh::host::packaged()
}
