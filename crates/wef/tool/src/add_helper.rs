use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;
use askama::Template;
use colored::Colorize;
use serde::Deserialize;

use crate::utils::{find_cef_root, print_error};

#[derive(Debug)]
pub(crate) struct AddHelperSettings {
    pub(crate) cef_root: Option<PathBuf>,
    pub(crate) app_path: PathBuf,
    pub(crate) wef_version: Option<String>,
    pub(crate) wef_path: Option<PathBuf>,
    pub(crate) release: bool,
    pub(crate) force: bool,
}

/// ```askama
/// [package]
/// name = "helper"
/// version = "0.1.0"
/// edition = "2024"
///
/// [dependencies]
/// {% if let Some(wef_version) = wef_version %}
/// wef = "{{ wef_version }}"
/// {% endif %}
/// {% if let Some(wef_path) = wef_path %}
/// wef = { path = "{{ wef_path }}" }
/// {% endif %}
/// ```
#[derive(Template)]
#[template(ext = "txt", in_doc = true)]
struct TemplateCargoToml {
    wef_version: Option<String>,
    wef_path: Option<String>,
}

const MAIN_RS: &str = r#"
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = wef::FrameworkLoader::load_in_helper()?;
    wef::exec_process()?;
    Ok(())
}
"#;

/// ```askama
/// <?xml version="1.0" encoding="UTF-8"?>
/// <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
/// <plist version="1.0">
/// <dict>
///   <key>CFBundleDevelopmentRegion</key>
///   <string>en</string>
///   <key>CFBundleDisplayName</key>
///   <string>{{ name }}</string>
///   <key>CFBundleExecutable</key>
///   <string>{{ name }}</string>
///   <key>CFBundleIdentifier</key>
///   <string>{{ identifier }}</string>
///   <key>CFBundleInfoDictionaryVersion</key>
///   <string>6.0</string>
///   <key>CFBundleName</key>
///   <string>{{ name }}</string>
///   <key>CFBundlePackageType</key>
///   <string>APPL</string>
///   <key>CFBundleVersion</key>
///   <string></string>
///   <key>CFBundleShortVersionString</key>
///   <string></string>
/// </dict>
/// </plist>
/// ```
#[derive(Template)]
#[template(ext = "txt", in_doc = true)]
struct HelperInfoPlist {
    name: String,
    identifier: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelperKind {
    Main,
    Alerts,
    Gpu,
    Plugin,
    Renderer,
}

impl HelperKind {
    const ALL: &[HelperKind] = &[
        HelperKind::Main,
        HelperKind::Alerts,
        HelperKind::Gpu,
        HelperKind::Plugin,
        HelperKind::Renderer,
    ];

    fn bundle_name(&self, bundle_name: &str) -> String {
        let helper_name = match self {
            HelperKind::Main => "Helper",
            HelperKind::Alerts => "Helper (Alerts)",
            HelperKind::Gpu => "Helper (GPU)",
            HelperKind::Plugin => "Helper (Plugin)",
            HelperKind::Renderer => "Helper (Renderer)",
        };
        format!("{} {}", bundle_name, helper_name)
    }

    fn bundle_identifier(&self, bundle_identifier: &str) -> String {
        match self {
            HelperKind::Main => format!("{}.helper", bundle_identifier),
            HelperKind::Alerts => format!("{}.helper.alerts", bundle_identifier),
            HelperKind::Gpu => format!("{}.helper.gpu", bundle_identifier),
            HelperKind::Plugin => format!("{}.helper.plugin", bundle_identifier),
            HelperKind::Renderer => format!("{}.helper.renderer", bundle_identifier),
        }
    }
}

fn query_wef_max_stable_version() -> Result<String> {
    #[derive(Debug, Deserialize)]
    struct CrateInfo {
        max_stable_version: String,
    }

    #[derive(Debug, Deserialize)]
    struct Response {
        #[serde(rename = "crate")]
        crate_: CrateInfo,
    }

    let client = reqwest::blocking::Client::new();
    Ok(client
        .get("https://crates.io/api/v1/crates/wef")
        .header("user-agent", "curl/8.7.1")
        .send()?
        .error_for_status()?
        .json::<Response>()?
        .crate_
        .max_stable_version)
}

fn create_helper_bin<F, R>(settings: &AddHelperSettings, callback: F) -> Result<R>
where
    F: FnOnce(&Path) -> Result<R>,
{
    println!("Building the helper binary...");

    let proj_dir = dirs::home_dir()
        .expect("home directory not found")
        .join(".wef-tool")
        .join("helper");
    std::fs::create_dir_all(proj_dir.as_path()).inspect_err(|err| {
        print_error(format_args!(
            "failed to create directory {}: {}",
            proj_dir.display(),
            err
        ));
    })?;

    // query wef version
    let (wef_version, wef_path) = if let Some(wef_path) = &settings.wef_path {
        println!("Using local Wef path: {}", wef_path.display());
        (None, Some(wef_path.display().to_string()))
    } else {
        let wef_version = settings.wef_version.clone().map(Ok).unwrap_or_else(|| {
            println!("Querying crates.io for the latest stable version of Wef...");
            query_wef_max_stable_version().inspect_err(|err| {
                print_error(format_args!("failed to query Wef version: {}", err));
            })
        })?;
        println!("Using Wef version: {}", wef_version);
        (Some(wef_version), None)
    };

    // create Cargo.toml
    let cargo_toml_path = proj_dir.join("Cargo.toml");
    let mut cargo_toml_file = File::create(&cargo_toml_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create {}: {}",
            cargo_toml_path.display(),
            err
        ));
    })?;

    TemplateCargoToml {
        wef_version,
        wef_path,
    }
    .write_into(&mut cargo_toml_file)
    .inspect_err(|err| {
        print_error(format_args!(
            "failed to write {}: {}",
            cargo_toml_path.display(),
            err
        ));
    })?;

    // create src/main.rs
    let src_path = proj_dir.join("src");
    std::fs::create_dir_all(&src_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create directory {}: {}",
            src_path.display(),
            err
        ));
    })?;

    let main_rs_path = proj_dir.join("src").join("main.rs");
    std::fs::write(&main_rs_path, MAIN_RS).inspect_err(|err| {
        print_error(format_args!(
            "failed to create {}: {}",
            main_rs_path.display(),
            err
        ));
    })?;

    // build
    let mut command = Command::new("cargo");

    command
        .arg("build")
        .arg("--target-dir")
        .arg(proj_dir.join("target"));

    if settings.release {
        command.arg("--release");
    }

    let output = command.current_dir(&proj_dir).output().inspect_err(|err| {
        print_error(format_args!("failed to run cargo build: {}", err));
    })?;

    if !output.status.success() {
        println!();
        print_error("cargo build failed");

        println!();
        println!("{}", String::from_utf8_lossy(&output.stderr));

        anyhow::bail!("cargo build failed");
    }

    let target_path = proj_dir
        .join("target")
        .join(if !settings.release {
            "debug"
        } else {
            "release"
        })
        .join("helper");
    callback(&target_path)
}

#[derive(Debug, Deserialize)]
struct BundleInfo {
    #[serde(rename = "CFBundleName")]
    bundle_name: String,
    #[serde(rename = "CFBundleIdentifier")]
    bundle_identifier: String,
}

fn read_bundle_info(path: &Path) -> Result<BundleInfo> {
    plist::from_file(path).map_err(Into::into)
}

fn create_helper_app(
    app_path: &Path,
    kind: HelperKind,
    bundle_info: &BundleInfo,
    bin_path: &Path,
) -> Result<()> {
    let frameworks_path = app_path.join("Contents").join("Frameworks");

    // create frameworks directory
    std::fs::create_dir_all(&frameworks_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create directory {}: {}",
            frameworks_path.display(),
            err
        ));
    })?;

    let helper_app_path = frameworks_path.join(format!(
        "{}.app",
        kind.bundle_name(&bundle_info.bundle_name)
    ));

    // create app directory
    std::fs::create_dir_all(&helper_app_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create directory {}: {}",
            helper_app_path.display(),
            err
        ));
    })?;

    // create Contents directory
    let contents_path = helper_app_path.join("Contents");
    std::fs::create_dir_all(&contents_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create directory {}: {}",
            contents_path.display(),
            err
        ));
    })?;

    // create plist
    let plist_path = contents_path.join("Info.plist");
    HelperInfoPlist {
        name: kind.bundle_name(&bundle_info.bundle_name),
        identifier: kind.bundle_identifier(&bundle_info.bundle_identifier),
    }
    .write_into(&mut File::create(&plist_path)?)
    .inspect_err(|err| {
        print_error(format_args!(
            "failed to create {}: {}",
            plist_path.display(),
            err
        ));
    })?;

    // create MacOS directory
    let macos_path = contents_path.join("MacOS");
    std::fs::create_dir_all(&macos_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create directory {}: {}",
            macos_path.display(),
            err
        ));
    })?;

    // copy binary
    let target_path = macos_path.join(kind.bundle_name(&bundle_info.bundle_name));
    std::fs::copy(bin_path, &target_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create {}: {}",
            target_path.display(),
            err
        ));
    })?;

    Ok(())
}

pub(crate) fn add_helper(settings: &AddHelperSettings) -> Result<()> {
    _ = find_cef_root(settings.cef_root.as_deref())?;

    println!(
        "Creating helper app into {}...",
        settings.app_path.display()
    );

    let info_path = settings.app_path.join("Contents").join("Info.plist");

    if !info_path.exists() {
        print_error(format_args!(
            "{} is not a valid Macos app.",
            settings.app_path.display(),
        ));
        anyhow::bail!("Info.plist not found");
    }

    let bundle_info = read_bundle_info(&info_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to read {}: {}",
            info_path.display(),
            err
        ));
    })?;

    if !settings.force
        && HelperKind::ALL.iter().all(|kind| {
            let helper_path = settings
                .app_path
                .join("Contents")
                .join("Frameworks")
                .join(format!(
                    "{}.app",
                    kind.bundle_name(&bundle_info.bundle_name)
                ));
            helper_path.exists()
        })
    {
        println!(
            "Helper apps already exist in {}. Use {} to overwrite.",
            "--force".bright_white(),
            settings.app_path.display()
        );
        return Ok(());
    }

    println!("Bundle name: {}", bundle_info.bundle_name);
    println!("Bundle identifier: {}", bundle_info.bundle_identifier);

    create_helper_bin(settings, |path| {
        for kind in HelperKind::ALL {
            create_helper_app(&settings.app_path, *kind, &bundle_info, path)?;
        }
        Ok(())
    })?;

    println!("{}", "Successfully!".green());
    Ok(())
}
