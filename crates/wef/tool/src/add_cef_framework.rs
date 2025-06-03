use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;

use crate::utils::{find_cef_root, print_error};

#[derive(Debug)]
pub(crate) struct AddCefFrameworkSettings {
    pub(crate) cef_root: Option<PathBuf>,
    pub(crate) app_path: PathBuf,
    pub(crate) release: bool,
    pub(crate) force: bool,
}

#[cfg(target_os = "macos")]
pub(crate) fn add_cef_framework(settings: &AddCefFrameworkSettings) -> Result<()> {
    let cef_root = find_cef_root(settings.cef_root.as_deref())?;

    println!(
        "Adding CEF framework into {}...",
        settings.app_path.display()
    );

    let contents_path = settings.app_path.join("Contents");
    if !contents_path.exists() {
        print_error(format_args!(
            "{} is not a valid Macos app.",
            settings.app_path.display(),
        ));
        anyhow::bail!("Frameworks not found");
    }

    // create frameworks directory
    let frameworks_path = contents_path.join("Frameworks");
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

    if !settings.force
        && frameworks_path
            .join("Chromium Embedded Framework.framework")
            .exists()
    {
        println!(
            "CEF framework already exists at {}. Use {} to overwrite.",
            "--force".bright_white(),
            frameworks_path.display()
        );
        return Ok(());
    }

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

#[cfg(target_os = "windows")]
pub(crate) fn add_cef_framework(settings: &AddCefFrameworkSettings) -> Result<()> {
    let cef_root = find_cef_root(settings.cef_root.as_deref())?;

    println!(
        "Adding CEF framework into {}...",
        settings.app_path.display()
    );

    let files = [
        "chrome_elf.dll",
        "d3dcompiler_47.dll",
        "dxcompiler.dll",
        "dxil.dll",
        "libcef.dll",
        "libEGL.dll",
        "libGLESv2.dll",
        "v8_context_snapshot.bin",
        "vk_swiftshader.dll",
        "vk_swiftshader_icd.json",
        "vulkan-1.dll",
    ];

    let resources = [
        "chrome_100_percent.pak",
        "chrome_200_percent.pak",
        "icudtl.dat",
        "resources.pak",
        "locales",
    ];

    if !settings.force
        && files
            .iter()
            .all(|filename| settings.app_path.join(filename).exists())
        && resources
            .iter()
            .all(|filename| settings.app_path.join(filename).exists())
    {
        println!(
            "CEF framework already exists in {}. Use {} to overwrite.",
            settings.app_path.display(),
            "--force".bright_white()
        );
        return Ok(());
    }

    for filename in files {
        let src_path = cef_root
            .join(if !settings.release {
                "Debug"
            } else {
                "Release"
            })
            .join(filename);
        let dst_path = settings.app_path.join(filename);
        std::fs::copy(src_path, dst_path).inspect_err(|err| {
            print_error(format_args!(
                "failed to copy {} to {}: {}",
                filename,
                settings.app_path.display(),
                err
            ));
        })?;
    }

    let resources_src_path = cef_root.join("Resources");
    fs_extra::dir::copy(
        &resources_src_path,
        &settings.app_path,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            copy_inside: false,
            content_only: true,
            ..Default::default()
        },
    )
    .inspect_err(|err| {
        print_error(format_args!(
            "failed to copy CEF Resources from {} to {}: {}",
            resources_src_path.display(),
            settings.app_path.display(),
            err
        ));
    })?;

    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn add_cef_framework(settings: &AddCefFrameworkSettings) -> Result<()> {
    let cef_root = find_cef_root(settings.cef_root.as_deref())?;

    println!(
        "Adding CEF framework into {}...",
        settings.app_path.display()
    );

    let files = [
        "libcef.so",
        "libEGL.so",
        "libGLESv2.so",
        "libvk_swiftshader.so",
        "libvulkan.so.1",
        "v8_context_snapshot.bin",
        "vk_swiftshader_icd.json",
    ];

    let resources = [
        "chrome_100_percent.pak",
        "chrome_200_percent.pak",
        "icudtl.dat",
        "resources.pak",
        "locales",
    ];

    if !settings.force
        && files
            .iter()
            .all(|filename| settings.app_path.join(filename).exists())
        && resources
            .iter()
            .all(|filename| settings.app_path.join(filename).exists())
    {
        println!(
            "CEF framework already exists in {}. Use {} to overwrite.",
            settings.app_path.display(),
            "--force".bright_white()
        );
        return Ok(());
    }

    for filename in files {
        let src_path = cef_root
            .join(if !settings.release {
                "Debug"
            } else {
                "Release"
            })
            .join(filename);
        let dst_path = settings.app_path.join(filename);
        std::fs::copy(src_path, dst_path).inspect_err(|err| {
            print_error(format_args!(
                "failed to copy {} to {}: {}",
                filename,
                settings.app_path.display(),
                err
            ));
        })?;
    }

    let resources_src_path = cef_root.join("Resources");
    fs_extra::dir::copy(
        &resources_src_path,
        &settings.app_path,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            copy_inside: false,
            content_only: true,
            ..Default::default()
        },
    )
    .inspect_err(|err| {
        print_error(format_args!(
            "failed to copy CEF Resources from {} to {}: {}",
            resources_src_path.display(),
            settings.app_path.display(),
            err
        ));
    })?;

    Ok(())
}
