use std::{fs::File, path::Path, process::Command};

use anyhow::{Context, Result};
use askama::Template;
use serde::Deserialize;

use crate::internal::InfoPlist;

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
    let _ = wef::SandboxContext::new();
    let _ = wef::FrameworkLoader::load_in_helper()?;
    wef::exec_process()?;
    Ok(())
}
"#;

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

fn create_helper_bin<F, R>(
    wef_version: Option<&str>,
    wef_path: Option<&Path>,
    release: bool,
    callback: F,
) -> Result<R>
where
    F: FnOnce(&Path) -> Result<R>,
{
    let proj_dir = dirs::home_dir()
        .expect("home directory not found")
        .join(".wef-tool")
        .join("helper");
    std::fs::create_dir_all(proj_dir.as_path())
        .with_context(|| format!("create project directory at {}", proj_dir.display()))?;

    // query wef version
    let (wef_version, wef_path) = if let Some(wef_path) = &wef_path {
        (None, Some(wef_path.display().to_string()))
    } else {
        match wef_version {
            Some(version) => (Some(version.to_string()), None),
            None => (
                Some(
                    query_wef_max_stable_version()
                        .with_context(|| "query latest stable version of Wef from crates.io")?,
                ),
                None,
            ),
        }
    };

    // create Cargo.toml
    let cargo_toml_path = proj_dir.join("Cargo.toml");
    let mut cargo_toml_file = File::create(&cargo_toml_path)
        .with_context(|| format!("create Cargo.toml at {}", cargo_toml_path.display()))?;

    TemplateCargoToml {
        wef_version,
        wef_path,
    }
    .write_into(&mut cargo_toml_file)
    .with_context(|| format!("create file at {}", cargo_toml_path.display()))?;

    // create src/main.rs
    let src_path = proj_dir.join("src");
    std::fs::create_dir_all(&src_path)
        .with_context(|| format!("create directory at {}", src_path.display()))?;

    let main_rs_path = proj_dir.join("src").join("main.rs");
    std::fs::write(&main_rs_path, MAIN_RS)
        .with_context(|| format!("create file at {}", main_rs_path.display()))?;

    // build
    let mut command = Command::new("cargo");

    command
        .arg("build")
        .arg("--target-dir")
        .arg(proj_dir.join("target"));

    if release {
        command.arg("--release");
    }

    let output = command
        .current_dir(&proj_dir)
        .output()
        .with_context(|| format!("run cargo build in {}", proj_dir.display()))?;

    anyhow::ensure!(
        output.status.success(),
        "failed to compile helper binary:\n{}\n",
        String::from_utf8_lossy(&output.stderr)
    );

    let target_path = proj_dir
        .join("target")
        .join(if !release { "debug" } else { "release" })
        .join("helper");
    callback(&target_path)
}

#[derive(Debug, Deserialize)]
struct BundleInfo {
    #[serde(rename = "CFBundleName")]
    bundle_name: String,
    #[serde(rename = "CFBundleIdentifier")]
    bundle_identifier: String,
    #[serde(rename = "CFBundleExecutable")]
    executable_name: Option<String>,
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
    std::fs::create_dir_all(&frameworks_path).with_context(|| {
        format!(
            "create frameworks directory at {}",
            frameworks_path.display()
        )
    })?;

    let helper_app_path = frameworks_path.join(format!(
        "{}.app",
        kind.bundle_name(&bundle_info.bundle_name)
    ));

    // create app directory
    std::fs::create_dir_all(&helper_app_path).with_context(|| {
        format!(
            "create helper app directory at {}",
            helper_app_path.display()
        )
    })?;

    // create Contents directory
    let contents_path = helper_app_path.join("Contents");
    std::fs::create_dir_all(&contents_path)
        .with_context(|| format!("create directory at {}", contents_path.display()))?;

    // create plist
    let plist_path = contents_path.join("Info.plist");
    let mut plist = InfoPlist::new(
        kind.bundle_name(&bundle_info.bundle_name),
        kind.bundle_identifier(&bundle_info.bundle_identifier),
    );
    plist.executable_name = Some(
        bundle_info
            .executable_name
            .as_ref()
            .map(|executable_name| kind.bundle_name(executable_name))
            .unwrap_or_else(|| kind.bundle_name(&bundle_info.bundle_name)),
    );
    plist.agent_app = true;

    plist
        .write_into(&mut File::create(&plist_path)?)
        .with_context(|| format!("create file at {}", plist_path.display()))?;

    // create MacOS directory
    let macos_path = contents_path.join("MacOS");
    std::fs::create_dir_all(&macos_path)
        .with_context(|| format!("create directory at {}", macos_path.display()))?;

    // copy binary
    let target_path = macos_path.join(kind.bundle_name(&bundle_info.bundle_name));
    std::fs::copy(bin_path, &target_path).with_context(|| {
        format!(
            "copy binary from {} to {}",
            bin_path.display(),
            target_path.display()
        )
    })?;

    Ok(())
}

pub(crate) fn add_helper(
    app_path: &Path,
    wef_version: Option<&str>,
    wef_path: Option<&Path>,
    release: bool,
    force: bool,
) -> Result<()> {
    let info_path = app_path.join("Contents").join("Info.plist");

    anyhow::ensure!(
        info_path.exists(),
        "{} is not a valid Macos app.",
        app_path.display()
    );

    let bundle_info = read_bundle_info(&info_path)
        .with_context(|| format!("read bundle info from {}", info_path.display()))?;

    if !force
        && HelperKind::ALL.iter().all(|kind| {
            let helper_path = app_path.join("Contents").join("Frameworks").join(format!(
                "{}.app",
                kind.bundle_name(&bundle_info.bundle_name)
            ));
            helper_path.exists()
        })
    {
        return Ok(());
    }

    create_helper_bin(wef_version, wef_path, release, |path| {
        for kind in HelperKind::ALL {
            create_helper_app(app_path, *kind, &bundle_info, path)?;
        }
        Ok(())
    })?;

    Ok(())
}
