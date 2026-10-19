//! NSH's command line with the Kit UI set available to its existing tooling.

use std::{path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    // This is the process entry point, before NSH starts worker threads.
    // An explicit override keeps NSH's normal runtime selection available.
    if std::env::var_os("NSH_RUNTIME").is_none()
        && let Some(directory) = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(PathBuf::from))
        && directory.join("gpui").is_dir()
    {
        unsafe { std::env::set_var("NSH_RUNTIME", directory) };
    }
    nsh_cli::main_with(vec![nsh_ui_mobile::registry(), gpui_kit_nsh::ui_set()])
}
