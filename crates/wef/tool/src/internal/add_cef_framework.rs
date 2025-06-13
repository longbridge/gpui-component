use std::path::Path;

use anyhow::{Context, Result};

pub(crate) fn add_cef_framework(
    cef_root: &Path,
    app_path: &Path,
    release: bool,
    force: bool,
) -> Result<()> {
    match std::env::consts::OS {
        "macos" => add_cef_framework_macos(cef_root, app_path, release, force),
        "windows" => add_cef_framework_windows(cef_root, app_path, release, force),
        "linux" => add_cef_framework_linux(cef_root, app_path, release, force),
        _ => {
            anyhow::bail!("Unsupported platform: {}", std::env::consts::OS);
        }
    }
}

fn add_cef_framework_macos(
    cef_root: &Path,
    app_path: &Path,
    release: bool,
    force: bool,
) -> Result<()> {
    let contents_path = app_path.join("Contents");
    anyhow::ensure!(
        contents_path.exists(),
        "{} is not a valid MacOS app.",
        app_path.display()
    );

    // create frameworks directory
    let frameworks_path = contents_path.join("Frameworks");
    std::fs::create_dir_all(&frameworks_path).with_context(|| {
        format!(
            "create frameworks directory at {}",
            frameworks_path.display()
        )
    })?;

    // copy CEF framework
    let cef_framework_path = cef_root
        .join(if !release { "Debug" } else { "Release" })
        .join("Chromium Embedded Framework.framework");

    if !force
        && frameworks_path
            .join("Chromium Embedded Framework.framework")
            .exists()
    {
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
    .with_context(|| {
        format!(
            "copy CEF framework from {} to {}",
            cef_framework_path.display(),
            frameworks_path.display()
        )
    })?;

    Ok(())
}

fn add_cef_framework_windows(
    cef_root: &Path,
    app_path: &Path,
    release: bool,
    force: bool,
) -> Result<()> {
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

    if !force
        && files
            .iter()
            .all(|filename| app_path.join(filename).exists())
        && resources
            .iter()
            .all(|filename| app_path.join(filename).exists())
    {
        return Ok(());
    }

    for filename in files {
        let src_path = cef_root
            .join(if !release { "Debug" } else { "Release" })
            .join(filename);
        let dst_path = app_path.join(filename);
        std::fs::copy(src_path, dst_path)
            .with_context(|| format!("copy {} to {}", filename, app_path.display()))?;
    }

    let resources_src_path = cef_root.join("Resources");
    fs_extra::dir::copy(
        &resources_src_path,
        app_path,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            copy_inside: false,
            content_only: true,
            ..Default::default()
        },
    )
    .with_context(|| {
        format!(
            "copy CEF Resources from {} to {}",
            resources_src_path.display(),
            app_path.display()
        )
    })?;

    Ok(())
}

fn add_cef_framework_linux(
    cef_root: &Path,
    app_path: &Path,
    release: bool,
    force: bool,
) -> Result<()> {
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

    if !force
        && files
            .iter()
            .all(|filename| app_path.join(filename).exists())
        && resources
            .iter()
            .all(|filename| app_path.join(filename).exists())
    {
        return Ok(());
    }

    for filename in files {
        let src_path = cef_root
            .join(if !release { "Debug" } else { "Release" })
            .join(filename);
        let dst_path = app_path.join(filename);
        std::fs::copy(src_path, dst_path)
            .with_context(|| format!("copy {} to {}", filename, app_path.display()))?;
    }

    let resources_src_path = cef_root.join("Resources");
    fs_extra::dir::copy(
        &resources_src_path,
        app_path,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            copy_inside: false,
            content_only: true,
            ..Default::default()
        },
    )
    .with_context(|| {
        format!(
            "copy CEF Resources from {} to {}",
            resources_src_path.display(),
            app_path.display()
        )
    })?;

    Ok(())
}
