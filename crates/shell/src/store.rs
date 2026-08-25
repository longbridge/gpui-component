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
//! So every mutation bumps [`Store::revision`], one write is in flight at a
//! time, and a completed write records the revision it landed. `flush` is a
//! barrier that waits for its revision to reach the disk rather than a second
//! writer racing the first.

use std::path::{Path, PathBuf};

use serde_json::Value as Json;

/// Settles a `flush` once the revision it is waiting for reaches the disk.
///
/// A boxed closure rather than anything engine-shaped: the store is above the
/// seam and must not know what a promise is.
pub type Settle = Box<dyn FnOnce(Result<(), String>)>;

/// A waiter with its outcome already decided, ready to be called.
///
/// The store hands these back instead of calling them, because settling a
/// `flush` re-enters script — which may call `gpui.store.set` — and the store is
/// borrowed for as long as the method that decided the outcome is running.
pub type Wake = Box<dyn FnOnce()>;

pub struct Store {
    pub(crate) path: PathBuf,
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

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            values: None,
            warm: None,
            revision: 0,
            written: 0,
            in_flight: None,
            waiting: Vec::new(),
            stalled: Vec::new(),
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

    /// Records that the values changed. What makes the change reach the disk is
    /// the host driving [`Store::begin_write`] afterwards.
    pub fn touch(&mut self) {
        self.revision += 1;
    }

    /// Whether memory is ahead of the disk.
    pub fn is_dirty(&self) -> bool {
        self.revision > self.written
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

    /// The next write to perform, if there is one and nothing is in flight.
    ///
    /// Encoding stays on this thread because it reads the cache, which does not
    /// leave it; only the bytes travel.
    pub fn begin_write(&mut self) -> Option<PendingWrite> {
        if self.in_flight.is_some() || !self.is_dirty() {
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

    /// Waiters an encode failure inside [`Store::begin_write`] left to settle.
    ///
    /// That method has no room to return them — its return value is the write —
    /// so they are parked here and collected by the same driver, one step later.
    #[must_use = "the waiters have to be settled once the store is no longer borrowed"]
    pub fn take_stalled(&mut self) -> Vec<Wake> {
        std::mem::take(&mut self.stalled)
    }

    /// Records the outcome of the write [`Store::begin_write`] handed out.
    ///
    /// Returns the `flush` calls this write settles. They are returned rather
    /// than called because settling one re-enters script, which may call
    /// `gpui.store.set` — and the store is borrowed for the length of this
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
        } else {
            self.written = self.written.max(revision);
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
    /// reason [`Store::finish_write`] returns rather than calls.
    #[must_use = "an already-satisfied waiter still has to be settled"]
    pub fn wait(&mut self, settle: Settle) -> Option<Settle> {
        if !self.is_dirty() && self.in_flight.is_none() {
            return Some(settle);
        }
        self.waiting.push((self.revision, settle));
        None
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
        serde_json::to_vec_pretty(values)
            .map(Some)
            .map_err(|error| format!("cannot encode the store: {error}"))
    }
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

    let mut temporary = path.to_path_buf().into_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);

    std::fs::write(&temporary, body)
        .map_err(|error| format!("cannot write `{}`: {error}", temporary.display()))?;
    std::fs::rename(&temporary, path)
        .map_err(|error| format!("cannot write `{}`: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;

    fn store() -> Store {
        let mut store = Store::new(std::env::temp_dir().join("gpui-shell-store-test.json"));
        store.warm = Some(Ok(serde_json::Map::new()));
        // Populates the cache, so `encode` has something to hand out.
        store.values().expect("the warm read succeeds");
        store
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
        let settle = store.wait(Box::new(|_| {}));
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
                .is_none()
        );

        for wake in store.finish_write(pending.revision(), Err("disk is full".to_owned())) {
            wake();
        }
        assert_eq!(settled.take(), Some(Err("disk is full".to_owned())));

        // And the queue is not wedged: the revision never landed, so it is still
        // owed to the disk.
        assert!(store.is_dirty());
        assert!(store.begin_write().is_some());
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
}
