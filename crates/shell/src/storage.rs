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
//!
//! # Why the store owns its write order
//!
//! Writing is asynchronous, so "what is on the disk" and "what is in memory"
//! are two different versions of the same object and the gap between them has
//! to be a number rather than a flag. A `dirty` bit cannot answer either
//! question a correct store has to answer:
//!
//! - *Did the write that just finished cover the change made while it was in
//!   flight?* With a flag, a mutation during a write set `dirty` and nothing
//!   ever looked at it again: if no further mutation followed, the last one a
//!   user made stayed in memory for ever.
//! - *Is this `flush` allowed to resolve?* A flush that starts its own write
//!   races the automatic one — same temporary file, no ordering — so the older
//!   revision could land last and undo the newer.
//!
//! So every mutation bumps [`Storage::revision`], one write is in flight at a
//! time, and a completed write records the revision it landed. `flush` is a
//! barrier that waits for its revision to reach the disk rather than a second
//! writer racing the first.

use std::path::{Path, PathBuf};

use serde_json::Value as Json;

const MAX_STORE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_STORE_KEYS: usize = 4096;
const MAX_STORE_VALUE_BYTES: usize = 1024 * 1024;
const MAX_FLUSH_WAITERS: usize = 1024;

/// Settles a `flush` once the revision it is waiting for reaches the disk.
///
/// A boxed closure rather than anything engine-shaped: the store is above the
/// seam and must not know what a promise is.
pub type Settle = Box<dyn FnOnce(Result<(), String>)>;

/// A waiter with its outcome already decided, ready to be called.
///
/// The store hands these back instead of calling them, because settling a
/// `flush` re-enters script — which may call `localStorage.setItem` — and the store is
/// borrowed for as long as the method that decided the outcome is running.
pub type Wake = Box<dyn FnOnce()>;

pub struct Storage {
    pub(crate) path: PathBuf,
    /// Whether anything here reaches a disk.
    ///
    /// `sessionStorage` is the same store with this off: the Web Storage API
    /// gives the two the same surface and distinguishes them only by how long
    /// they last, which is exactly the difference between having a file and
    /// not having one. Deno draws the same line — one SQLite database on disk,
    /// one in memory — and so does Node, whose `sessionStorage` is process-only
    /// while `localStorage` needs `--localstorage-file`.
    persisted: bool,
    values: Option<serde_json::Map<String, Json>>,
    /// The outcome of the read done when the path was set, so the first script
    /// call gets the answer rather than the syscall.
    pub(crate) warm: Option<Result<serde_json::Map<String, Json>, String>>,
    /// The version of what is in memory. Bumped by every mutation.
    revision: u64,
    /// The highest revision known to have reached the disk.
    written: u64,
    /// The revision a write now on its way to the disk will land, if there is
    /// one. At most one at a time: two concurrent writes share `<path>.tmp` and
    /// land in whatever order the disk chooses, so the older can finish last
    /// and undo the newer.
    in_flight: Option<u64>,
    /// A revision whose persistence attempt failed. It remains dirty, but the
    /// completion callback must not immediately retry it in a tight loop. A
    /// later mutation has a newer revision; an explicit `flush` clears this
    /// pause because the caller is deliberately asking to try again.
    failed: Option<u64>,
    /// `flush` callers, each waiting for a revision.
    waiting: Vec<(u64, Settle)>,
    /// Waiters that an encode failure settled, parked for the driver to collect.
    stalled: Vec<Wake>,
}

/// A write the host should perform, and the revision it will land.
///
/// Returned rather than performed because the store cannot spawn: it is a plain
/// data structure above the seam, and the executor lives below it.
pub struct PendingWrite {
    revision: u64,
    path: PathBuf,
    body: Vec<u8>,
}

impl PendingWrite {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn into_parts(self) -> (PathBuf, Vec<u8>) {
        (self.path, self.body)
    }
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            persisted: true,
            values: None,
            warm: None,
            revision: 0,
            written: 0,
            in_flight: None,
            failed: None,
            waiting: Vec::new(),
            stalled: Vec::new(),
        }
    }

    /// The store behind `sessionStorage`: it starts empty, never reads a file
    /// and never writes one, so it is gone with the process.
    pub fn in_memory() -> Self {
        Self {
            values: Some(serde_json::Map::new()),
            persisted: false,
            ..Self::new(PathBuf::new())
        }
    }

    /// Loads on first use. A missing file is an empty store — a first run is
    /// not an error. A malformed file is an error, because silently discarding
    /// a user's settings is worse than refusing to start.
    pub fn values(&mut self) -> Result<&mut serde_json::Map<String, Json>, String> {
        if self.values.is_none() {
            // Whatever [`set_storage_path`] read at start-up. The fallback covers
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

    /// Records that the values changed. What makes the change reach the disk is
    /// the host driving [`Storage::begin_write`] afterwards.
    pub fn touch(&mut self) {
        self.revision += 1;
    }

    pub fn set(&mut self, key: String, value: Json) -> Result<(), String> {
        let value_size = serde_json::to_vec(&value)
            .map_err(|error| format!("cannot encode store value `{key}`: {error}"))?
            .len();
        if value_size > MAX_STORE_VALUE_BYTES {
            return Err(format!(
                "store value `{key}` is {value_size} bytes, over the \
                 {MAX_STORE_VALUE_BYTES}-byte per-value limit"
            ));
        }

        let mut candidate = self.values()?.clone();
        candidate.insert(key, value);
        validate_values(&candidate)?;
        self.values = Some(candidate);
        self.touch();
        Ok(())
    }

    /// Whether memory is ahead of the disk.
    pub fn is_dirty(&self) -> bool {
        // A store with no file is never behind one: `flush` on `sessionStorage`
        // resolves at once rather than waiting for a write that will not happen.
        self.persisted && self.revision > self.written
    }

    /// Reads the file. A missing one is an empty store — a first run is not an
    /// error. A malformed one is an error, because silently discarding a user's
    /// settings is worse than refusing to start.
    pub fn load(&self) -> Result<serde_json::Map<String, Json>, String> {
        match std::fs::File::open(&self.path) {
            Ok(mut file) => {
                use std::io::Read as _;
                let size = file
                    .metadata()
                    .map_err(|error| format!("cannot read `{}`: {error}", self.path.display()))?
                    .len();
                if size > MAX_STORE_BYTES {
                    return Err(store_too_large(&self.path, size));
                }
                let mut bytes = Vec::with_capacity(size as usize);
                file.by_ref()
                    .take(MAX_STORE_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|error| format!("cannot read `{}`: {error}", self.path.display()))?;
                if bytes.len() as u64 > MAX_STORE_BYTES {
                    return Err(store_too_large(&self.path, bytes.len() as u64));
                }
                let values = serde_json::from_slice(&bytes).map_err(|error| {
                    format!(
                        "`{}` is not a valid store file: {error}",
                        self.path.display()
                    )
                })?;
                validate_values(&values)?;
                Ok(values)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(serde_json::Map::new())
            }
            Err(error) => Err(format!("cannot read `{}`: {error}", self.path.display())),
        }
    }

    /// The next write to perform, if there is one and nothing is in flight.
    ///
    /// Encoding stays on this thread because it reads the cache, which does not
    /// leave it; only the bytes travel.
    pub fn begin_write(&mut self) -> Option<PendingWrite> {
        if self.in_flight.is_some() || !self.is_dirty() || self.failed == Some(self.revision) {
            return None;
        }

        let revision = self.revision;
        match self.encode() {
            Ok(Some(body)) => {
                self.in_flight = Some(revision);
                Some(PendingWrite {
                    revision,
                    path: self.path.clone(),
                    body,
                })
            }
            // Nothing has been read or written, so there is nothing to land.
            // Marking it written stops a flush from waiting for a write that
            // will never happen.
            Ok(None) => {
                self.written = revision;
                None
            }
            Err(error) => {
                tracing::error!("{error}");
                // Not retried: the same values would fail to encode again, and
                // a waiter for this revision would hang for ever.
                self.written = revision;
                self.stalled = self.settle_up_to(revision, &Err(error));
                None
            }
        }
    }

    /// Waiters an encode failure inside [`Storage::begin_write`] left to settle.
    ///
    /// That method has no room to return them — its return value is the write —
    /// so they are parked here and collected by the same driver, one step later.
    #[must_use = "the waiters have to be settled once the store is no longer borrowed"]
    pub fn take_stalled(&mut self) -> Vec<Wake> {
        std::mem::take(&mut self.stalled)
    }

    /// Records the outcome of the write [`Storage::begin_write`] handed out.
    ///
    /// Returns the `flush` calls this write settles. They are returned rather
    /// than called because settling one re-enters script, which may call
    /// `localStorage.setItem` — and the store is borrowed for the length of this
    /// method.
    #[must_use = "the waiters have to be settled once the store is no longer borrowed"]
    pub fn finish_write(&mut self, revision: u64, result: Result<(), String>) -> Vec<Wake> {
        if self.in_flight == Some(revision) {
            self.in_flight = None;
        }

        if let Err(error) = &result {
            // The revision did not land. Its waiters are settled with the
            // failure rather than left pending, because the next write carries a
            // higher revision and would never satisfy them.
            tracing::error!("the store could not be written: {error}");
            self.failed = Some(revision);
        } else {
            self.written = self.written.max(revision);
            if self.failed.is_some_and(|failed| failed <= revision) {
                self.failed = None;
            }
        }

        self.settle_up_to(revision, &result)
    }

    /// Releases a write that was never started, so the queue does not stall.
    pub fn abort_write(&mut self, revision: u64) {
        if self.in_flight == Some(revision) {
            self.in_flight = None;
        }
    }

    /// Waits for everything written so far to reach the disk.
    ///
    /// Settles `settle` immediately when it already has. Returns it unused in
    /// that case so the caller settles it outside the borrow, for the same
    /// reason [`Storage::finish_write`] returns rather than calls.
    #[must_use = "an already-satisfied waiter still has to be settled"]
    pub fn wait(&mut self, settle: Settle) -> Result<Option<Settle>, String> {
        if !self.is_dirty() && self.in_flight.is_none() {
            return Ok(Some(settle));
        }
        if self.waiting.len() == MAX_FLUSH_WAITERS {
            return Err(format!(
                "localStorage.flush() exceeded the {MAX_FLUSH_WAITERS} pending-waiter limit"
            ));
        }
        if self.in_flight.is_none() && self.failed == Some(self.revision) {
            self.failed = None;
        }
        self.waiting.push((self.revision, settle));
        Ok(None)
    }

    pub fn ensure_waiter_capacity(&self) -> Result<(), String> {
        if self.is_dirty() && self.waiting.len() == MAX_FLUSH_WAITERS {
            return Err(format!(
                "localStorage.flush() exceeded the {MAX_FLUSH_WAITERS} pending-waiter limit"
            ));
        }
        Ok(())
    }

    fn settle_up_to(&mut self, revision: u64, outcome: &Result<(), String>) -> Vec<Wake> {
        let mut ready = Vec::new();
        let mut still_waiting = Vec::new();
        for (wanted, settle) in self.waiting.drain(..) {
            if wanted <= revision {
                let outcome = outcome.clone();
                ready.push(Box::new(move || settle(outcome)) as Wake);
            } else {
                still_waiting.push((wanted, settle));
            }
        }
        self.waiting = still_waiting;
        ready
    }

    fn encode(&self) -> Result<Option<Vec<u8>>, String> {
        let Some(values) = &self.values else {
            return Ok(None);
        };
        validate_values(values)?;
        serde_json::to_vec_pretty(values)
            .map(Some)
            .map_err(|error| format!("cannot encode the store: {error}"))
    }
}

fn validate_values(values: &serde_json::Map<String, Json>) -> Result<(), String> {
    if values.len() > MAX_STORE_KEYS {
        return Err(format!("store exceeded the {MAX_STORE_KEYS}-key limit"));
    }
    for (key, value) in values {
        let size = serde_json::to_vec(value)
            .map_err(|error| format!("cannot encode store value `{key}`: {error}"))?
            .len();
        if size > MAX_STORE_VALUE_BYTES {
            return Err(format!(
                "store value `{key}` is {size} bytes, over the \
                 {MAX_STORE_VALUE_BYTES}-byte per-value limit"
            ));
        }
    }
    // Persistence uses pretty JSON, so enforce the limit against the bytes
    // that will actually be written rather than a smaller compact estimate.
    let size = serde_json::to_vec_pretty(values)
        .map_err(|error| format!("cannot encode the store: {error}"))?
        .len() as u64;
    if size > MAX_STORE_BYTES {
        return Err(format!(
            "store is {size} bytes, over the {MAX_STORE_BYTES}-byte limit"
        ));
    }
    Ok(())
}

fn store_too_large(path: &Path, size: u64) -> String {
    format!(
        "store `{}` is {size} bytes, over the {MAX_STORE_BYTES}-byte limit",
        path.display()
    )
}

/// Writes to a temporary file and renames it over the target, so a crash
/// mid-write leaves the previous settings intact rather than a truncated file.
///
/// A free function rather than a method: it runs on the background executor,
/// where the store itself — which never leaves the main thread — cannot go.
pub fn persist(path: &Path, body: Vec<u8>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
    }

    let temporary = temporary_path(path);

    let result = write_private(&temporary, &body)
        .map_err(|error| format!("cannot write `{}`: {error}", temporary.display()))
        .and_then(|()| {
            replace_file(&temporary, path)
                .map_err(|error| format!("cannot write `{}`: {error}", path.display()))
        })
        .and_then(|()| sync_parent(path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn temporary_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(format!(
        ".tmp.{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    PathBuf::from(temporary)
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("cannot sync `{}`: {error}", parent.display()))
}

#[cfg(not(unix))]
fn sync_parent(_: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_file(temporary: &Path, path: &Path) -> std::io::Result<()> {
    std::fs::rename(temporary, path)
}

#[cfg(target_os = "windows")]
fn replace_file(temporary: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows::{
        Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        },
        core::PCWSTR,
    };

    let temporary = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    unsafe {
        MoveFileExW(
            PCWSTR(temporary.as_ptr()),
            PCWSTR(path.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(std::io::Error::other)
    }
}

#[cfg(unix)]
fn write_private(path: &Path, body: &[u8]) -> std::io::Result<()> {
    use std::{
        fs::OpenOptions,
        io::Write as _,
        os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    };

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(body)?;
    file.sync_all()
}

#[cfg(not(unix))]
fn write_private(path: &Path, body: &[u8]) -> std::io::Result<()> {
    use std::{fs::OpenOptions, io::Write as _};

    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(body)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;

    fn store() -> Storage {
        let mut store = Storage::new(std::env::temp_dir().join("gpui-shell-store-test.json"));
        store.warm = Some(Ok(serde_json::Map::new()));
        // Populates the cache, so `encode` has something to hand out.
        store.values().expect("the warm read succeeds");
        store
    }

    #[cfg(unix)]
    #[test]
    fn persisted_store_is_private_to_its_owner() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory =
            std::env::temp_dir().join(format!("gpui-shell-private-store-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("test directory");
        let path = directory.join("store.json");
        persist(&path, br#"{"token":"secret"}"#.to_vec()).expect("persist private store");

        let mode = std::fs::metadata(&path)
            .expect("store metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "store files may contain OAuth tokens");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn persist_replaces_an_existing_store() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-replace-store-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("test directory");
        let path = directory.join("store.json");

        persist(&path, br#"{"revision":1}"#.to_vec()).expect("first persist");
        persist(&path, br#"{"revision":2}"#.to_vec()).expect("replacement persist");

        assert_eq!(
            std::fs::read(&path).expect("persisted store"),
            br#"{"revision":2}"#
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn concurrent_persists_use_distinct_temporary_paths() {
        let path = Path::new("settings.json");
        let first = temporary_path(path);
        let second = temporary_path(path);

        assert_ne!(first, second);
        assert_eq!(first.parent(), path.parent());
        assert_eq!(second.parent(), path.parent());
    }

    /// The bug a `dirty` flag could not see: a change made *while* a write is on
    /// its way has to be written after it.
    #[test]
    fn a_mutation_during_a_write_is_written_by_the_next_one() {
        let mut store = store();

        store.touch();
        let first = store.begin_write().expect("the first write");
        assert_eq!(first.revision(), 1);

        // The user changes something else before the disk has answered.
        store.touch();
        assert!(
            store.begin_write().is_none(),
            "a second write must not run beside the first"
        );

        assert!(store.finish_write(1, Ok(())).is_empty());
        let second = store
            .begin_write()
            .expect("the change made during the write");
        assert_eq!(second.revision(), 2);
    }

    #[test]
    fn a_store_with_nothing_left_to_write_starts_no_write() {
        let mut store = store();
        assert!(store.begin_write().is_none());

        store.touch();
        let pending = store.begin_write().expect("the write");
        assert!(store.finish_write(pending.revision(), Ok(())).is_empty());
        assert!(!store.is_dirty());
        assert!(store.begin_write().is_none());
    }

    /// `flush` is a barrier, not a second writer: it resolves when the revision
    /// it was called at reaches the disk.
    #[test]
    fn flush_waits_for_the_write_in_flight_rather_than_racing_it() {
        let mut store = store();
        store.touch();
        let pending = store.begin_write().expect("the write");

        let settled = Rc::new(Cell::new(None));
        let record = settled.clone();
        assert!(
            store
                .wait(Box::new(move |outcome| record.set(Some(outcome))))
                .expect("within waiter limit")
                .is_none(),
            "a flush with a write in flight has to wait"
        );
        assert!(settled.take().is_none(), "nothing has reached the disk yet");

        let woken = store.finish_write(pending.revision(), Ok(()));
        assert_eq!(woken.len(), 1);
        for wake in woken {
            wake();
        }
        assert_eq!(settled.take(), Some(Ok(())));
    }

    #[test]
    fn flush_resolves_at_once_when_the_disk_is_already_current() {
        let mut store = store();
        let settle = store.wait(Box::new(|_| {})).expect("within waiter limit");
        assert!(settle.is_some(), "nothing to wait for");
    }

    /// A failed write must settle its waiters rather than leave them pending:
    /// the next write carries a higher revision and would never satisfy them.
    #[test]
    fn a_failed_write_rejects_the_flush_that_was_waiting_for_it() {
        let mut store = store();
        store.touch();
        let pending = store.begin_write().expect("the write");

        let settled = Rc::new(Cell::new(None));
        let record = settled.clone();
        assert!(
            store
                .wait(Box::new(move |outcome| record.set(Some(outcome))))
                .expect("within waiter limit")
                .is_none()
        );

        for wake in store.finish_write(pending.revision(), Err("disk is full".to_owned())) {
            wake();
        }
        assert_eq!(settled.take(), Some(Err("disk is full".to_owned())));

        // The revision never landed, so it remains dirty, but its automatic
        // completion must not immediately drive the same failing write again.
        assert!(store.is_dirty());
        assert!(
            store.begin_write().is_none(),
            "the failed revision should park until new intent retries it"
        );

        // A later mutation is new intent and carries a newer revision.
        store.touch();
        assert_eq!(
            store
                .begin_write()
                .expect("the later mutation retries")
                .revision(),
            2
        );
    }

    /// A write that is never started must release the queue, or every later one
    /// waits behind a write that does not exist.
    #[test]
    fn an_abandoned_write_releases_the_queue() {
        let mut store = store();
        store.touch();
        let pending = store.begin_write().expect("the write");
        store.abort_write(pending.revision());

        assert!(store.begin_write().is_some(), "the queue has to move again");
    }

    #[test]
    fn warm_load_refuses_an_oversized_store() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-oversized-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("directory");
        let path = directory.join("store.json");
        let file = std::fs::File::create(&path).expect("store");
        file.set_len(MAX_STORE_BYTES + 1).expect("sparse store");

        let error = Storage::new(path)
            .load()
            .expect_err("oversized store must fail");
        assert!(
            error.contains("store") && error.contains("limit"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn warm_load_validates_key_count_and_value_size() {
        let directory = std::env::temp_dir().join(format!(
            "gpui-shell-invalid-store-shape-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("directory");
        let path = directory.join("store.json");

        let too_many: serde_json::Map<String, Json> = (0..=MAX_STORE_KEYS)
            .map(|index| (index.to_string(), Json::Bool(true)))
            .collect();
        std::fs::write(
            &path,
            serde_json::to_vec(&too_many).expect("encode fixture"),
        )
        .expect("write fixture");
        let error = Storage::new(path.clone()).load().expect_err("key limit");
        assert!(error.contains("key limit"), "{error}");

        let oversized = serde_json::json!({
            "huge": "x".repeat(MAX_STORE_VALUE_BYTES + 1)
        });
        std::fs::write(
            &path,
            serde_json::to_vec(&oversized).expect("encode fixture"),
        )
        .expect("write fixture");
        let error = Storage::new(path).load().expect_err("value limit");
        assert!(error.contains("per-value limit"), "{error}");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn store_rejects_too_many_keys_and_an_oversized_value() {
        let mut many_keys = store();
        for index in 0..MAX_STORE_KEYS {
            many_keys
                .set(index.to_string(), Json::Bool(true))
                .expect("within key limit");
        }
        let error = many_keys
            .set("one-too-many".into(), Json::Bool(true))
            .expect_err("key limit");
        assert!(error.contains("key limit"), "{error}");

        let mut store = store();
        let error = store
            .set(
                "huge".into(),
                Json::String("x".repeat(MAX_STORE_VALUE_BYTES + 1)),
            )
            .expect_err("value limit");
        assert!(error.contains("per-value limit"), "{error}");
    }

    #[test]
    fn encoded_store_aggregate_is_bounded() {
        let values = (0..MAX_STORE_KEYS)
            .map(|index| (index.to_string(), Json::String("x".repeat(2048))))
            .collect();
        let error = validate_values(&values).expect_err("aggregate limit must reject");
        assert!(
            error.contains("store is") && error.contains("byte limit"),
            "{error}"
        );
    }

    #[test]
    fn flush_waiters_are_bounded() {
        let mut store = store();
        store.touch();
        for _ in 0..MAX_FLUSH_WAITERS {
            assert!(
                store
                    .wait(Box::new(|_| {}))
                    .expect("within limit")
                    .is_none()
            );
        }
        let error = match store.wait(Box::new(|_| {})) {
            Err(error) => error,
            Ok(_) => panic!("waiter limit must reject"),
        };
        assert!(error.contains("pending-waiter limit"), "{error}");
    }
}
