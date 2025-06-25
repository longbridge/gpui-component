use std::{
    ffi::OsStr,
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use askama::Template;
use cargo_metadata::{Metadata, MetadataCommand};
use icns::IconFamily;
use image::GenericImageView;

use crate::internal::{InfoPlist, add_cef_framework, add_helper};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryKind {
    Bin,
    Example,
}

#[derive(Debug)]
struct BinaryInfo {
    metadata: serde_json::Value,
    package_name: String,
    package_path: PathBuf,
    target_name: String,
    kind: BinaryKind,
    version: String,
}

fn execute_path(
    metadata: &Metadata,
    target_dir: &Path,
    package: Option<&str>,
    bin: Option<&str>,
    example: Option<&str>,
) -> Result<(PathBuf, BinaryInfo)> {
    let packages = if let Some(package_name) = package {
        vec![
            metadata
                .workspace_packages()
                .into_iter()
                .find(|package| package.name.as_str() == package_name)
                .ok_or_else(|| {
                    anyhow::anyhow!("No package `{}` found in the workspace", package_name)
                })?,
        ]
    } else if metadata.workspace_default_members.is_available() {
        metadata.workspace_default_packages()
    } else {
        metadata.workspace_packages()
    };

    let (package, target, binary_kind) = if let Some(bin_name) = bin {
        packages
            .iter()
            .find_map(|package| {
                package
                    .targets
                    .iter()
                    .find(|target| target.is_bin() && target.name == bin_name)
                    .map(|target| (*package, target, BinaryKind::Bin))
            })
            .ok_or_else(|| anyhow::anyhow!("no bin target named `{}`", bin_name))?
    } else if let Some(example_name) = example {
        packages
            .iter()
            .find_map(|package| {
                package
                    .targets
                    .iter()
                    .find(|target| target.is_example() && target.name == example_name)
                    .map(|target| (*package, target, BinaryKind::Example))
            })
            .ok_or_else(|| anyhow::anyhow!("no example target named `{}`", example_name))?
    } else {
        let mut bin_targets = packages
            .iter()
            .flat_map(|package| {
                package
                    .targets
                    .iter()
                    .filter_map(|target| target.is_bin().then_some((*package, target)))
            })
            .collect::<Vec<_>>();
        anyhow::ensure!(!bin_targets.is_empty(), "a bin target must be available");
        anyhow::ensure!(bin_targets.len() == 1, "could not determine which binary");
        let (package, target) = bin_targets.remove(0);
        (package, target, BinaryKind::Bin)
    };

    let exec_path = match std::env::consts::OS {
        "macos" => Ok(target_dir.join(&target.name)),
        "windows" => Ok(target_dir.join(&target.name).with_extension("exe")),
        "linux" => Ok(target_dir.join(&target.name)),
        _ => Err(anyhow::anyhow!(
            "Unsupported platform: {}",
            std::env::consts::OS
        )),
    };
    Ok((
        exec_path?,
        BinaryInfo {
            metadata: package.metadata.clone(),
            package_name: package.name.to_string(),
            package_path: package
                .manifest_path
                .parent()
                .unwrap()
                .to_path_buf()
                .into_std_path_buf(),
            target_name: target.name.clone(),
            kind: binary_kind,
            version: package.version.to_string(),
        },
    ))
}

fn find_bundle_settings(
    binary_info: &BinaryInfo,
    bundle_type: Option<&str>,
) -> Result<Option<InfoPlist>> {
    let config = match binary_info.kind {
        BinaryKind::Bin => {
            let mut config = binary_info
                .metadata
                .pointer(&format!("/bundle/bin/{}", binary_info.target_name));
            if config.is_none() && binary_info.target_name == binary_info.package_name {
                config = binary_info.metadata.pointer("/bundle");
            }
            config
        }
        BinaryKind::Example => binary_info
            .metadata
            .pointer(&format!("/bundle/example/{}", binary_info.target_name)),
    };

    let config = if let Some(bundle_type) = bundle_type {
        Some(config.and_then(|c| c.get(bundle_type)).ok_or_else(|| {
            anyhow::anyhow!("No bundle settings found for type `{}`", bundle_type)
        })?)
    } else {
        config
    };

    config
        .map(|config| {
            let mut config: InfoPlist = serde_json::from_value(config.clone())?;
            if config.bundle_short_version.is_none() {
                config.bundle_short_version = Some(binary_info.version.clone());
            }
            Ok::<_, anyhow::Error>(config)
        })
        .transpose()
        .context("parse bundle settings")
}

fn create_plist(binary_info: &BinaryInfo, bundle_type: Option<&str>) -> Result<InfoPlist> {
    let plist = find_bundle_settings(binary_info, bundle_type)?.unwrap_or_else(|| {
        println!("Bundle settings is not found, fallback to default settings");
        let mut plist = InfoPlist::new(
            &binary_info.target_name,
            format!("io.github.wef.{}", &binary_info.target_name),
        );
        plist.bundle_short_version = Some(binary_info.version.clone());
        plist
    });
    Ok(plist)
}

fn create_icns_file(
    package_path: &Path,
    resources_dir: &Path,
    plist: &mut InfoPlist,
) -> Result<()> {
    if plist.icons.is_empty() {
        return Ok(());
    }

    for icon_path in &plist.icons {
        let icon_path = package_path.join(icon_path);
        if icon_path.extension() == Some(OsStr::new("icns")) {
            std::fs::create_dir(resources_dir)?;

            let target_path = resources_dir.join(&plist.name).with_extension("icns");
            std::fs::copy(&icon_path, &target_path).with_context(|| {
                format!("copy {} to {}", icon_path.display(), target_path.display())
            })?;
            plist.icon = Some(format!("{}.icns", plist.name));
            return Ok(());
        }
    }

    let mut family = IconFamily::new();

    fn make_icns_image(img: image::DynamicImage) -> Result<icns::Image> {
        let pixel_format = match img.color() {
            image::ColorType::Rgba8 => icns::PixelFormat::RGBA,
            image::ColorType::Rgb8 => icns::PixelFormat::RGB,
            image::ColorType::La8 => icns::PixelFormat::GrayAlpha,
            image::ColorType::L8 => icns::PixelFormat::Gray,
            _ => {
                anyhow::bail!("Unsupported image color type: {:?}", img.color());
            }
        };
        Ok(icns::Image::from_data(
            pixel_format,
            img.width(),
            img.height(),
            img.into_bytes(),
        )?)
    }

    fn add_icon_to_family(
        icon: image::DynamicImage,
        density: u32,
        family: &mut icns::IconFamily,
    ) -> Result<()> {
        // Try to add this image to the icon family.  Ignore images whose sizes
        // don't map to any ICNS icon type; print warnings and skip images that
        // fail to encode.
        match icns::IconType::from_pixel_size_and_density(icon.width(), icon.height(), density) {
            Some(icon_type) => {
                if !family.has_icon_with_type(icon_type) {
                    let icon = make_icns_image(icon)?;
                    family.add_icon_with_type(&icon, icon_type)?;
                }
                Ok(())
            }
            None => anyhow::bail!("No matching IconType"),
        }
    }

    fn is_retina(path: &Path) -> bool {
        path.file_stem()
            .and_then(OsStr::to_str)
            .map(|stem| stem.ends_with("@2x"))
            .unwrap_or(false)
    }

    let mut images_to_resize: Vec<(image::DynamicImage, u32, u32)> = vec![];
    for icon_path in &plist.icons {
        let icon_path = package_path.join(icon_path);
        let icon = image::open(&icon_path)
            .with_context(|| format!("load image {}", icon_path.display()))?;
        let density = if is_retina(&icon_path) { 2 } else { 1 };
        let (w, h) = icon.dimensions();
        let orig_size = w.min(h);
        let next_size_down = 2f32.powf((orig_size as f32).log2().floor()) as u32;
        if orig_size > next_size_down {
            images_to_resize.push((icon, next_size_down, density));
        } else {
            add_icon_to_family(icon, density, &mut family)?;
        }
    }

    for (icon, next_size_down, density) in images_to_resize {
        let icon = icon.resize_exact(next_size_down, next_size_down, image::imageops::Lanczos3);
        add_icon_to_family(icon, density, &mut family)?;
    }

    if !family.is_empty() {
        std::fs::create_dir_all(resources_dir)?;
        let icns_path = resources_dir.join(&plist.name).with_extension("icns");
        let icns_file = BufWriter::new(File::create(&icns_path)?);
        family
            .write(icns_file)
            .with_context(|| format!("write icns file {}", icns_path.display()))?;
        plist.icon = Some(format!("{}.icns", plist.name));
        return Ok(());
    }

    anyhow::bail!("No usable icon files found.")
}

fn bundle_macos_app(
    exec_path: &Path,
    binary_info: BinaryInfo,
    cef_root: &Path,
    release: bool,
    wef_version: Option<&str>,
    wef_path: Option<&Path>,
    bundle_type: Option<&str>,
) -> Result<PathBuf> {
    let filename = exec_path.file_name().unwrap();
    let app_path = exec_path
        .parent()
        .unwrap()
        .join(format!("{}.app", filename.to_string_lossy()));

    let macos_path = app_path.join("Contents").join("MacOS");
    std::fs::create_dir_all(&macos_path).context("create app directory")?;

    std::fs::copy(exec_path, macos_path.join(filename)).context("copy binary to app bundle")?;

    let plist_path = app_path.join("Contents").join("Info.plist");
    let mut plist = create_plist(&binary_info, bundle_type)?;

    println!("Create ICNS file...");
    let resources_path = app_path.join("Contents").join("Resources");
    create_icns_file(&binary_info.package_path, &resources_path, &mut plist)
        .context("create ICNS file")?;

    plist
        .write_into(&mut File::create(&plist_path)?)
        .with_context(|| format!("create file at {}", plist_path.display()))?;

    println!("Add CEF Framework...");
    add_cef_framework(cef_root, &app_path, release, false)?;
    println!("Add Helper Processes...");
    add_helper(&app_path, wef_version, wef_path, release, false)?;
    Ok(macos_path.join(filename))
}

pub(crate) fn build(
    package: Option<String>,
    bin: Option<String>,
    example: Option<String>,
    release: bool,
    wef_version: Option<&str>,
    wef_path: Option<&Path>,
    bundle_type: Option<&str>,
) -> Result<PathBuf> {
    let cef_root = crate::internal::find_cef_root();
    println!("Using CEF_ROOT: {}", cef_root.display());

    let metadata = MetadataCommand::new()
        .current_dir(std::env::current_dir().unwrap())
        .exec()?;

    let mut command = Command::new("cargo");

    command.arg("build");

    if let Some(package) = &package {
        command.arg("--package").arg(package);
    }

    if let Some(bin) = &bin {
        command.arg("--bin").arg(bin);
    }

    if let Some(example) = &example {
        command.arg("--example").arg(example);
    }

    if release {
        command.arg("--release");
    }

    anyhow::ensure!(command.status()?.success(), "failed to build the project");

    let target_dir = metadata
        .target_directory
        .join(if release { "release" } else { "debug" });

    match std::env::consts::OS {
        "macos" => {
            let (exec_path, binary_info) = execute_path(
                &metadata,
                target_dir.as_std_path(),
                package.as_deref(),
                bin.as_deref(),
                example.as_deref(),
            )?;
            bundle_macos_app(
                &exec_path,
                binary_info,
                &cef_root,
                release,
                wef_version,
                wef_path,
                bundle_type,
            )
        }
        "windows" | "linux" => {
            anyhow::ensure!(
                bundle_type.is_none(),
                "bundle-type argument is used only on macOS"
            );

            add_cef_framework(&cef_root, target_dir.as_std_path(), release, false)?;
            execute_path(
                &metadata,
                target_dir.as_std_path(),
                package.as_deref(),
                bin.as_deref(),
                example.as_deref(),
            )
            .map(|(path, _)| path)
        }
        _ => {
            anyhow::bail!("Unsupported platform: {}", std::env::consts::OS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_bin() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/package_bin")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Bin);
        assert_eq!(binary_info.target_name, "package-bin");
        assert_eq!(binary_info.version, "0.3.0");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.3.0".to_string()),
                ..InfoPlist::new(
                    "test-package-bin",
                    "io.github.longbridge.wef.tests.package-bin"
                )
            })
        );
    }

    #[test]
    fn bin() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/bin")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            None,
            Some("bin1"),
            None,
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Bin);
        assert_eq!(binary_info.target_name, "bin1");
        assert_eq!(binary_info.version, "0.5.0");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.5.0".to_string()),
                ..InfoPlist::new("test-bin", "io.github.longbridge.wef.tests.bin")
            })
        );
    }

    #[test]
    fn example() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/example")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            None,
            None,
            Some("example1"),
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Example);
        assert_eq!(binary_info.target_name, "example1");
        assert_eq!(binary_info.version, "0.3.2");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.3.2".to_string()),
                ..InfoPlist::new("test-example", "io.github.longbridge.wef.tests.example")
            })
        );
    }

    #[test]
    fn workspace_package_bin() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/workspace")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            Some("package-bin"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Bin);
        assert_eq!(binary_info.target_name, "package-bin");
        assert_eq!(binary_info.version, "0.3.0");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.3.0".to_string()),
                ..InfoPlist::new(
                    "test-package-bin",
                    "io.github.longbridge.wef.tests.package-bin"
                )
            })
        );
    }

    #[test]
    fn workspace_bin() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/workspace")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            Some("pkg-bin"),
            Some("bin1"),
            None,
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Bin);
        assert_eq!(binary_info.target_name, "bin1");
        assert_eq!(binary_info.version, "0.5.0");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.5.0".to_string()),
                ..InfoPlist::new("test-bin", "io.github.longbridge.wef.tests.bin")
            })
        );
    }

    #[test]
    fn workspace_bin_without_package() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/workspace")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            None,
            Some("bin1"),
            None,
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Bin);
        assert_eq!(binary_info.target_name, "bin1");
        assert_eq!(binary_info.version, "0.5.0");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.5.0".to_string()),
                ..InfoPlist::new("test-bin", "io.github.longbridge.wef.tests.bin")
            })
        );
    }

    #[test]
    fn workspace_example() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/workspace")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            Some("pkg-example"),
            None,
            Some("example1"),
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Example);
        assert_eq!(binary_info.target_name, "example1");
        assert_eq!(binary_info.version, "0.3.2");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.3.2".to_string()),
                ..InfoPlist::new("test-example", "io.github.longbridge.wef.tests.example")
            })
        );
    }

    #[test]
    fn workspace_example_without_package() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/workspace")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            None,
            None,
            Some("example1"),
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Example);
        assert_eq!(binary_info.target_name, "example1");
        assert_eq!(binary_info.version, "0.3.2");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.3.2".to_string()),
                ..InfoPlist::new("test-example", "io.github.longbridge.wef.tests.example")
            })
        );
    }

    #[test]
    fn workspace_default_members() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/default_members")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            None,
            Some("bin2"),
            None,
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Bin);
        assert_eq!(binary_info.package_name, "bin2");
        assert_eq!(binary_info.target_name, "bin2");
        assert_eq!(binary_info.version, "0.5.1");

        assert_eq!(
            find_bundle_settings(&binary_info, None).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.5.1".to_string()),
                ..InfoPlist::new("test-bin2", "io.github.longbridge.wef.tests.bin2")
            })
        );

        let err = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            None,
            Some("bin1"),
            None,
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "no bin target named `bin1`");
    }

    #[test]
    fn bundle_type() {
        let metadata = MetadataCommand::new()
            .current_dir("tests/package_bin")
            .exec()
            .unwrap();

        let (_, binary_info) = execute_path(
            &metadata,
            metadata.target_directory.as_std_path(),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(binary_info.kind, BinaryKind::Bin);
        assert_eq!(binary_info.target_name, "package-bin");
        assert_eq!(binary_info.version, "0.3.0");

        assert_eq!(
            find_bundle_settings(&binary_info, Some("preview")).unwrap(),
            Some(InfoPlist {
                category: Some("Utility".to_string()),
                bundle_short_version: Some("0.3.0".to_string()),
                ..InfoPlist::new(
                    "test-package-bin-preview",
                    "io.github.longbridge.wef.tests.package-bin.preview"
                )
            })
        );
    }
}
