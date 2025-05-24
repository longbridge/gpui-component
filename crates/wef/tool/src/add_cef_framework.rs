use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;

use crate::utils::{find_cef_root, print_error};

#[derive(Debug)]
pub(crate) struct AddCefFrameworkSettings {
    pub(crate) cef_root: Option<PathBuf>,
    pub(crate) app_path: PathBuf,
    pub(crate) release: bool,
}

pub(crate) fn add_cef_framework(settings: &AddCefFrameworkSettings) -> Result<()> {
    let cef_root = find_cef_root(settings.cef_root.as_deref())?;

    println!(
        "Adding CEF framework into {}...",
        settings.app_path.display()
    );

    let frameworks_path = settings.app_path.join("Contents").join("Frameworks");

    // create frameworks directory
    std::fs::create_dir_all(&frameworks_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create directory {}: {}",
            frameworks_path.display(),
            err
        ));
    })?;

    // copy CEF framework
    let cef_framework_path = cef_root
        .join(if !settings.release {
            "Debug"
        } else {
            "Release"
        })
        .join("Chromium Embedded Framework.framework");

    fs_extra::dir::copy(
        &cef_framework_path,
        &frameworks_path,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            copy_inside: false,
            content_only: false,
            ..Default::default()
        },
    )
    .inspect_err(|err| {
        print_error(format_args!(
            "failed to copy CEF framework from {} to {}: {}",
            cef_framework_path.display(),
            frameworks_path.display(),
            err
        ));
    })?;

    println!("{}", "Successfully!".green());
    Ok(())
}
