use std::path::PathBuf;

use anyhow::Result;

#[allow(unused_variables)]
pub(crate) fn add_framework(
    app_path: PathBuf,
    release: bool,
    force: bool,
    wef_version: Option<String>,
    wef_path: Option<PathBuf>,
) -> Result<()> {
    let cef_root = crate::internal::find_cef_root();
    crate::internal::add_cef_framework(&cef_root, &app_path, release, force)?;

    #[cfg(target_os = "macos")]
    crate::internal::add_helper(
        &app_path,
        wef_version.as_deref(),
        wef_path.as_deref(),
        release,
        force,
    )?;
    Ok(())
}
