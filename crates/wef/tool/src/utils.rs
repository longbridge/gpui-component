use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::Result;
use colored::Colorize;

pub(crate) fn find_cef_root(cef_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(cef_root) = cef_root {
        println!("Using CEF_ROOT: {}", cef_root.display());
        Ok(cef_root.to_path_buf())
    } else if let Ok(cef_root) = std::env::var("CEF_ROOT") {
        println!("Using CEF_ROOT: {}", cef_root);
        Ok(PathBuf::from(cef_root))
    } else {
        let err = anyhow::anyhow!("CEF_ROOT environment variable not set");
        print_error(&err);
        Err(err)
    }
}

pub(crate) fn print_error(err: impl Display) {
    eprintln!("{}: {}", "Error".red(), err);
}
