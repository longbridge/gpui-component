//! Assets an application ships with itself.
//!
//! `svg(path)` names a file, and the file has to come from somewhere. It comes
//! from the application directory and nowhere else — the same root that bounds
//! module resolution — so an application carries its own icons and cannot read
//! an image from outside the directory the user pointed the runtime at.
//!
//! Note the asymmetry, because it surprises people: `import "./counter.js"`
//! resolves against the *importing file*, the way every JavaScript module
//! system does, while `svg("icons/check.svg")` resolves against the
//! *application root*, the way a web application's public directory does. A
//! runtime cannot tell which module called `svg`, so per-file asset paths are
//! not available to it. The rule is therefore stated in the README, and a
//! missing asset says exactly where it was looked for rather than drawing
//! nothing.

use std::{borrow::Cow, cell::RefCell, collections::HashSet, path::PathBuf};

use gpui::{AssetSource, SharedString};

/// Serves files from one application directory.
#[derive(Clone, Debug)]
pub struct AppAssets {
    root: PathBuf,
}

impl AppAssets {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Resolves an asset path inside the root, rejecting traversal.
    ///
    /// The check is the same shape as the module resolver's: join, normalize,
    /// and require the result to still be under the root.
    fn resolve(&self, path: &str) -> Option<PathBuf> {
        let mut resolved = self.root.clone();
        for component in std::path::Path::new(path).components() {
            match component {
                std::path::Component::ParentDir => {
                    resolved.pop();
                }
                std::path::Component::CurDir | std::path::Component::RootDir => {}
                other => resolved.push(other),
            }
        }

        resolved.starts_with(&self.root).then_some(resolved)
    }
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let Some(resolved) = self.resolve(path) else {
            anyhow::bail!("`{path}` is outside the application directory");
        };

        match std::fs::read(&resolved) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            // A missing asset cannot be an error: GPUI asks for assets it may
            // not need, and returning one would fail the frame. But an icon
            // that silently does not appear is the hardest kind of mistake to
            // find, so it is reported — once per path, because this runs on
            // every paint.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                report_missing(path, &resolved);
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let Some(resolved) = self.resolve(path) else {
            return Ok(Vec::new());
        };

        let mut names: Vec<SharedString> = std::fs::read_dir(resolved)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| SharedString::from(entry.file_name().to_string_lossy().to_string()))
            .collect();
        names.sort();
        Ok(names)
    }
}

thread_local! {
    static REPORTED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn report_missing(requested: &str, resolved: &std::path::Path) {
    let first_time = REPORTED.with(|reported| reported.borrow_mut().insert(requested.to_owned()));
    if first_time {
        tracing::warn!(
            "asset `{requested}` was not found at {}; asset paths resolve against the \
             application directory, not against the file that asked for them",
            resolved.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traversal_is_refused() {
        let assets = AppAssets::new(PathBuf::from("/app"));
        assert!(assets.resolve("../secret.svg").is_none());
        assert_eq!(
            assets.resolve("icons/check.svg"),
            Some(PathBuf::from("/app/icons/check.svg"))
        );
    }

    #[test]
    fn a_missing_asset_is_not_an_error() {
        let assets = AppAssets::new(std::env::temp_dir());
        assert!(assets.load("definitely-not-here.svg").unwrap().is_none());
    }
}
