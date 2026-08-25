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

use cap_std::{ambient_authority, fs::Dir};
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

    /// Opens the application directory and answers the path within it.
    ///
    /// The same shape the `fs` capability uses, and for the same reason: a
    /// handle that cannot be made to name something outside itself, rather than
    /// a string that has to be judged and then trusted. An `icons` that is a
    /// link somewhere else is lexically inside the root and reads from outside
    /// it, and a link that appears between the judging and the reading is worse
    /// still.
    ///
    /// The lexical pass stays as the cheap half: it turns `../..` into a refusal
    /// here rather than an `errno` from below.
    fn resolve(&self, path: &str) -> Option<(Dir, PathBuf)> {
        let mut resolved = PathBuf::new();
        for component in std::path::Path::new(path).components() {
            match component {
                // The path is being built relative to the root, so `..` can only
                // mean "leave it". Refusing here rather than popping gives the
                // reason; `Dir` would refuse it too, with an `errno`.
                std::path::Component::ParentDir => return None,
                std::path::Component::CurDir | std::path::Component::RootDir => {}
                other => resolved.push(other),
            }
        }
        if resolved.as_os_str().is_empty() {
            return None;
        }

        let dir = Dir::open_ambient_dir(&self.root, ambient_authority()).ok()?;
        Some((dir, resolved))
    }
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let Some((dir, resolved)) = self.resolve(path) else {
            anyhow::bail!("`{path}` is outside the application directory");
        };

        match dir.read(&resolved) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            // A missing asset cannot be an error: GPUI asks for assets it may
            // not need, and returning one would fail the frame. But an icon
            // that silently does not appear is the hardest kind of mistake to
            // find, so it is reported — once per path, because this runs on
            // every paint.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                report_missing(path, &self.root.join(&resolved));
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let Some((dir, resolved)) = self.resolve(path) else {
            return Ok(Vec::new());
        };

        let mut names: Vec<SharedString> = dir
            .read_dir(&resolved)
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

    fn sandbox(name: &str) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("gpui-shell-assets-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("app")).expect("an application");
        base
    }

    #[test]
    fn traversal_is_refused() {
        let base = sandbox("traversal");
        let assets = AppAssets::new(base.join("app"));

        assert!(assets.resolve("../secret.svg").is_none());
        let (_, path) = assets.resolve("icons/check.svg").expect("a path inside");
        assert_eq!(path, PathBuf::from("icons/check.svg"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_missing_asset_is_not_an_error() {
        let assets = AppAssets::new(std::env::temp_dir());
        assert!(assets.load("definitely-not-here.svg").unwrap().is_none());
    }

    /// An asset path is a grant like any other: it names the application's own
    /// directory, and a link inside it must not turn that into the filesystem.
    ///
    /// The refusal is at the read rather than at the resolution, because what
    /// protects the directory is the handle, not the judgement.
    #[test]
    #[cfg(unix)]
    fn an_asset_behind_a_symlink_is_refused() {
        let base = sandbox("symlink");
        let root = base.join("app");
        let outside = base.join("outside");
        std::fs::create_dir_all(&outside).expect("somewhere outside");
        std::fs::write(outside.join("secret.svg"), b"secret").expect("a secret");
        std::fs::write(root.join("real.svg"), b"real").expect("an asset");
        std::os::unix::fs::symlink(&outside, root.join("icons")).expect("a symlink");

        let assets = AppAssets::new(root);

        assert!(
            assets.load("icons/secret.svg").is_err(),
            "an asset was read through a symlink out of the application directory"
        );
        assert_eq!(
            assets
                .load("real.svg")
                .expect("an ordinary asset")
                .expect("its bytes")
                .as_ref(),
            b"real"
        );
        assert!(assets.resolve("../outside/secret.svg").is_none());

        let _ = std::fs::remove_dir_all(&base);
    }
}
