//! Builds and installs Kit runners in NSH's portable runtime layout.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, ExitCode, Stdio},
};

fn main() -> ExitCode {
    match install() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("runtime installation failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn install() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let mut manifest = None;
    let mut destination = None;
    let mut profile = "release".to_owned();
    let mut features = vec!["dev".to_owned(), "cli".to_owned()];
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--manifest-path") => {
                manifest = Some(PathBuf::from(
                    arguments.next().ok_or("missing manifest path")?,
                ));
            }
            Some("--profile") => {
                profile = arguments
                    .next()
                    .ok_or("missing profile")?
                    .into_string()
                    .map_err(|_| "profile must be UTF-8")?;
                if profile != "dev" && profile != "release" {
                    return Err("profile must be dev or release".into());
                }
            }
            Some("--features") => {
                features.push(
                    arguments
                        .next()
                        .ok_or("missing features")?
                        .into_string()
                        .map_err(|_| "features must be UTF-8")?,
                );
            }
            Some("--help" | "-h") => {
                println!(
                    "gpui-kit-nsh-runtime --manifest-path <crates/nsh/Cargo.toml> [--profile release|dev] [--features speech,tree-sitter-languages] <runtime-directory>"
                );
                return Ok(());
            }
            Some(value) if value.starts_with('-') => {
                return Err(format!("unknown argument {value}").into());
            }
            _ if destination.is_none() => destination = Some(PathBuf::from(argument)),
            _ => return Err("expected one runtime destination".into()),
        }
    }
    let manifest = manifest
        .ok_or("--manifest-path is required")?
        .canonicalize()?;
    let destination = destination.ok_or("a runtime directory is required")?;
    let mut child = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .arg("build")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(&manifest)
        .args(["--profile", &profile, "--features", &features.join(",")])
        .args([
            "--bin",
            "nsh-gpui-host",
            "--bin",
            "nsh-gpui-app",
            "--bin",
            "gpui-kit-nsh",
        ])
        .arg("--message-format=json-render-diagnostics")
        .stdout(Stdio::piped())
        .spawn()?;
    let mut executables = HashMap::new();
    for line in BufReader::new(child.stdout.take().ok_or("cargo stdout is unavailable")?).lines() {
        let line = line?;
        if let Ok(message) = serde_json::from_str::<serde_json::Value>(&line)
            && message.get("reason").and_then(|value| value.as_str()) == Some("compiler-artifact")
            && let Some(executable) = message.get("executable").and_then(|value| value.as_str())
            && let Some(name) = message
                .get("target")
                .and_then(|value| value.get("name"))
                .and_then(|value| value.as_str())
        {
            executables.insert(name.to_owned(), PathBuf::from(executable));
        }
    }
    if !child.wait()?.success() {
        return Err("cargo could not build the runners".into());
    }
    let directory = destination.join("gpui");
    std::fs::create_dir_all(&directory)?;
    for name in ["nsh-gpui-host", "nsh-gpui-app"] {
        let executable = executables
            .get(name)
            .ok_or_else(|| format!("cargo emitted no {name}"))?;
        std::fs::copy(
            executable,
            directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        )?;
    }
    let cli = executables
        .get("gpui-kit-nsh")
        .ok_or("cargo emitted no gpui-kit-nsh CLI")?;
    std::fs::copy(
        cli,
        destination.join(format!("gpui-kit-nsh{}", std::env::consts::EXE_SUFFIX)),
    )?;
    println!(
        "Installed {} Kit runners in {}",
        profile,
        destination.canonicalize()?.display()
    );
    println!(
        "Use {}/gpui-kit-nsh run --target gpui or package --target gpui; the CLI discovers its sibling runtime.",
        destination.display()
    );
    Ok(())
}
