//! Settings that survive a restart, and the file they live in.
//!
//! Above the engine seam because none of it is engine-specific: a flat JSON
//! object, a cache, and an atomic write. What used to keep it below was the
//! background write reaching for the engine's scheduler, and the ambient `App`
//! reached through [`crate::scope`] answers that just as well — which is the
//! same reason the store can now belong to a [`crate::policy::Policy`] rather
//! than to the thread.
//!
//! # Why the cache is not optional
//!
//! `get` is reachable from `render`, and a file read per render would be
//! absurd. So reads answer from memory and writes go through it: the store is
//! read once, when the host names the file, and written in the background
//! whenever it changes.

use std::path::{Path, PathBuf};

use serde_json::Value as Json;

pub struct Store {
    pub(crate) path: PathBuf,
    values: Option<serde_json::Map<String, Json>>,
    /// The outcome of the read done when the path was set, so the first script
    /// call gets the answer rather than the syscall.
    pub(crate) warm: Option<Result<serde_json::Map<String, Json>, String>>,
    /// Mutated since the last write was scheduled.
    pub(crate) dirty: bool,
    /// A write is on its way to the disk. One at a time, so a burst of `set`
    /// calls becomes one file rather than one file each — which is also what the
    /// old synchronous version got wrong, quite apart from where it ran.
    pub(crate) writing: bool,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            values: None,
            warm: None,
            dirty: false,
            writing: false,
        }
    }

    /// Loads on first use. A missing file is an empty store — a first run is
    /// not an error. A malformed file is an error, because silently discarding
    /// a user's settings is worse than refusing to start.
    pub fn values(&mut self) -> Result<&mut serde_json::Map<String, Json>, String> {
        if self.values.is_none() {
            // Whatever [`set_store_path`] read at start-up. The fallback covers
            // a host that never called it, which is already an error the store
            // reports elsewhere — it must not also become a panic.
            let loaded = match self.warm.take() {
                Some(loaded) => loaded,
                None => self.load(),
            };
            self.values = Some(loaded?);
        }
        Ok(self.values.as_mut().expect("just populated"))
    }

    /// Reads the file. A missing one is an empty store — a first run is not an
    /// error. A malformed one is an error, because silently discarding a user's
    /// settings is worse than refusing to start.
    pub fn load(&self) -> Result<serde_json::Map<String, Json>, String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                format!(
                    "`{}` is not a valid store file: {error}",
                    self.path.display()
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(serde_json::Map::new())
            }
            Err(error) => Err(format!("cannot read `{}`: {error}", self.path.display())),
        }
    }

    /// Encodes the current values, for a write that will happen elsewhere.
    ///
    /// Encoding stays on this thread because it reads the cache, which is
    /// thread-local; only the bytes travel.
    pub fn encode(&self) -> Result<Option<Vec<u8>>, String> {
        let Some(values) = &self.values else {
            return Ok(None);
        };
        serde_json::to_vec_pretty(values)
            .map(Some)
            .map_err(|error| format!("cannot encode the store: {error}"))
    }
}

/// Writes to a temporary file and renames it over the target, so a crash
/// mid-write leaves the previous settings intact rather than a truncated file.
///
/// A free function rather than a method: it runs on the background executor,
/// where the store itself — a thread-local — cannot go.
pub fn persist(path: &Path, body: Vec<u8>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
    }

    let mut temporary = path.to_path_buf().into_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);

    std::fs::write(&temporary, body)
        .map_err(|error| format!("cannot write `{}`: {error}", temporary.display()))?;
    std::fs::rename(&temporary, path)
        .map_err(|error| format!("cannot write `{}`: {error}", path.display()))
}
