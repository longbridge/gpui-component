use std::{path::Path, process::Command};

use anyhow::Result;

pub(crate) fn run(
    package: Option<String>,
    bin: Option<String>,
    example: Option<String>,
    release: bool,
    wef_version: Option<&str>,
    wef_path: Option<&Path>,
    args: Vec<String>,
) -> Result<()> {
    let exec_path =
        crate::commands::build(package, bin, example, release, wef_version, wef_path, None)?;
    Command::new(&exec_path).args(args).status()?;
    Ok(())
}
