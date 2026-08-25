//! Hot reload: watching an application directory and rebuilding its view.
//!
//! The design (`docs/gpui-shell.md` §21.2) is a five step pipeline:
//!
//! ```text
//! file change detected → debounce ~200ms → tear down the view →
//! re-evaluate the application module → rebuild the view →
//! optionally restore serialized state
//! ```
//!
//! Two properties matter more than the mechanism.
//!
//! The first is that a script error must never take the host down (§21.1). A
//! reload runs untrusted script: the module can throw while it is evaluated,
//! and the view constructor can throw while it runs. [`reload`] therefore does
//! all of its fallible work before it touches the live entity, so a broken save
//! leaves the previous working view on screen with the error reported to the
//! caller — the same promise the render-time error overlay makes.
//!
//! The second is that this module stays engine independent. It names
//! [`ShellRuntime`], [`ViewObject`] and [`Entity<ScriptView>`] and nothing
//! below them, exactly like every other module above `engine::`. Reloading is
//! the same three calls whatever the engine turns out to be.
//!
//! # Why polling, and what a real watcher would buy
//!
//! [`SourceWatcher`] compares modification stamps instead of subscribing to
//! filesystem events, because `gpui-shell` deliberately takes no dependency on
//! `notify` (§21.2: the host injects the watcher). Polling is honest for the
//! job it has: the host drives it from a GPUI timer at a low frequency — a
//! second is plenty for a human editing a file — and the cost is one `stat` per
//! source file in a directory that holds a handful of them.
//!
//! A `notify`-based watcher would improve three things, none of which is fatal
//! here: latency would drop from "up to one poll interval" to milliseconds; the
//! cost would stop scaling with the file count, which matters for an
//! application that vendors a large dependency tree; and it would see the
//! changes a stamp cannot — a rename or an atomic replace that preserves both
//! size and timestamp. If the host already runs a `notify` watcher for theme
//! files, feeding its events into [`SourceWatcher::notice`] is a smaller change
//! than replacing this type.

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context as _, Result, bail};
use gpui::{App, Entity, Window};

use crate::{engine::ShellRuntime, scope, view::ScriptView};

/// The default quiet period. An editor writes a file several times in a burst —
/// truncate, write, rename — and each of those is a distinct change. Reloading
/// on the first one would evaluate a half-written module.
const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(200);

/// How deep the scan descends before it gives up. An application directory is
/// flat by design; the limit exists so a symlink farm or a vendored package
/// tree cannot turn one poll into an unbounded walk.
const MAX_DEPTH: usize = 8;

/// The most files one scan will stat. Same reasoning as [`MAX_DEPTH`]: a poll
/// runs on the UI thread and must have a bounded cost.
const MAX_FILES: usize = 4096;

/// Extensions a reload can be triggered by. Only sources the runtime actually
/// evaluates count — a change to a README should not restart the application.
const SOURCE_EXTENSIONS: [&str; 2] = ["js", "mjs"];

/// Directory names that are never application source, skipped before their
/// contents are stated. Hidden entries are skipped separately.
const SKIPPED_DIRECTORIES: [&str; 2] = ["node_modules", "target"];

/// Watches an application directory and reports when its sources change.
///
/// The watcher answers one question — "has the tree settled after a change?" —
/// and deliberately does not know what to do about it. Deciding to reload, and
/// what to do when the reload fails, belongs to the host.
pub struct SourceWatcher {
    directory: PathBuf,
    debounce: Duration,
    /// The stamp reported by the last scan. A change is a difference from this.
    stamp: TreeStamp,
    /// When the tree was last seen changing. `Some` means a change has been
    /// observed but not yet reported, because the debounce window is still
    /// open. Every further change pushes this forward, which is what collapses
    /// a burst into one report.
    changed_at: Option<Instant>,
}

impl SourceWatcher {
    /// Starts watching `directory`, taking the current tree as the baseline.
    ///
    /// The baseline is captured here rather than on the first [`poll`] so that
    /// starting a watcher does not itself look like a change.
    ///
    /// [`poll`]: Self::poll
    pub fn new(directory: PathBuf) -> Self {
        let stamp = scan(&directory);
        Self {
            directory,
            debounce: DEFAULT_DEBOUNCE,
            stamp,
            changed_at: None,
        }
    }

    /// Sets the debounce window: how long the tree has to stay still before a
    /// change is reported.
    ///
    /// Shorten it in tests, where the writes are deliberate and there is no
    /// burst to absorb. [`Duration::ZERO`] makes [`poll`] report a change on
    /// the very poll that observes it.
    ///
    /// [`poll`]: Self::poll
    pub fn with_debounce(mut self, window: Duration) -> Self {
        self.debounce = window;
        self
    }

    /// The directory being watched.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The debounce window.
    pub fn debounce(&self) -> Duration {
        self.debounce
    }

    /// Returns true when the tree changed since the last poll and has been
    /// still for at least the debounce window.
    ///
    /// Returns true at most once per burst: reporting clears the pending
    /// change, so a host that polls in a loop reloads once per save rather than
    /// once per poll. A directory that has been deleted reads as an empty tree,
    /// which is a change like any other and then stays quiet.
    pub fn poll(&mut self) -> bool {
        let stamp = scan(&self.directory);
        if stamp != self.stamp {
            self.stamp = stamp;
            self.changed_at = Some(Instant::now());
        }

        match self.changed_at {
            Some(at) if at.elapsed() >= self.debounce => {
                self.changed_at = None;
                true
            }
            _ => false,
        }
    }

    /// Records a change observed by someone else — a host-injected `notify`
    /// watcher, or a menu command that forces a reload.
    ///
    /// The change still goes through the debounce window, so an external event
    /// stream and the poll loop can be mixed without reloading twice for one
    /// save.
    pub fn notice(&mut self) {
        self.changed_at = Some(Instant::now());
    }
}

/// A cheap summary of a source tree, compared for equality to detect a change.
///
/// It is three aggregates rather than a file list because a poll must not
/// allocate proportionally to the tree on every tick, and because equality is
/// the only question being asked. Each aggregate covers a case the others miss:
/// `newest` catches an edit in place, `files` catches an add or a delete, and
/// `bytes` catches an edit whose timestamp did not move — which happens on
/// filesystems with coarse timestamps, and in tests that write twice quickly.
///
/// What it cannot see is a change that preserves all three, such as swapping
/// two files' names. That is the honest cost of not using `notify`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct TreeStamp {
    newest: Option<SystemTime>,
    files: usize,
    bytes: u64,
}

/// Every watched file's newest modification time, plus the counts that make the
/// stamp sensitive to additions and edits.
///
/// A missing or unreadable directory is not an error: an application directory
/// can vanish mid-edit (a checkout, a move), and the watcher's job is to keep
/// running and report the tree it can see, which is an empty one.
fn scan(directory: &Path) -> TreeStamp {
    let mut stamp = TreeStamp::default();
    let mut pending = vec![(directory.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            if stamp.files >= MAX_FILES {
                return stamp;
            }

            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }

            // `DirEntry::file_type` does not follow symlinks, so a symlinked
            // directory is neither a file nor a directory here and is skipped.
            // That is what keeps a symlink cycle from hanging the poll.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_dir() {
                if depth < MAX_DEPTH && !SKIPPED_DIRECTORIES.contains(&name.as_ref()) {
                    pending.push((entry.path(), depth + 1));
                }
                continue;
            }

            if !file_type.is_file() || !is_source(&name) {
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };

            stamp.files += 1;
            stamp.bytes = stamp.bytes.saturating_add(metadata.len());
            if let Ok(modified) = metadata.modified() {
                stamp.newest = Some(match stamp.newest {
                    Some(newest) => newest.max(modified),
                    None => modified,
                });
            }
        }
    }

    stamp
}

/// Whether a file name is script source the runtime would evaluate.
fn is_source(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, extension)| SOURCE_EXTENSIONS.contains(&extension))
}

/// Reloads an application in place, keeping the window and its entity.
///
/// The window, the entity handle, and every host-side reference to it survive:
/// only the script object behind the view is replaced. That is what makes a
/// reload invisible to the host — no window is reopened, no layout is rebuilt.
///
/// # Atomicity
///
/// Both fallible steps happen before the live view is touched:
///
/// 1. re-evaluate the application module, which can throw;
/// 2. construct a new view instance, which can also throw;
/// 3. only then swap the object in and notify.
///
/// So a save that does not compile returns `Err` and changes nothing on screen.
/// The caller should surface the error — a toast, or the same error surface a
/// render failure uses — and keep the previous view running (§21.1).
///
/// # Phase
///
/// A reload mutates a view and requests a repaint, so it may only run from a
/// phase that allows a notify: an event or a task, never a render or a layout
/// pass (see [`crate::scope`]). Calling it mid-render would swap the object out
/// from under the element tree currently being built. Outside any scope — from
/// a host timer, which is the expected caller — there is no frame in progress
/// and the reload is safe.
///
/// # State preservation
///
/// §21.2 lists carrying state across a reload as optional, and routes it
/// through the same `serialize()` / `deserialize()` round trip as layout
/// persistence (§15.3). That path does not exist yet, so this function does not
/// invent a second one: the new instance starts from its constructor's state.
/// When the serialization path lands, it belongs between steps 2 and 3 above —
/// read the old object's state before the swap, hand it to the new object
/// after — which is why the swap is a single statement at the end.
/// How often an embedded watcher looks. The binary uses the same figure; a
/// quarter second is under the threshold at which a save feels like it did not
/// take, and far above the cost of one `stat` per source file.
pub const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Reloads a script view whenever its sources change, for a host that embeds the
/// runtime — **and does nothing at all in a release build.**
///
/// The `gpui-shell` binary has `--watch` because the person running it is the
/// person editing. A host that embeds the runtime has no such flag to offer, and
/// the answer is not to invent one: a debug build *is* the development build, so
/// editing a script and seeing the panel change is what should happen by
/// default, and a shipped binary must never sit and poll a directory it has no
/// reason to believe anyone is editing.
///
/// ```rust,ignore
/// // Once, where the view is created.
/// gpui_shell::watch::reload_in_debug(runtime.clone(), view.clone(), directory, "main.js", window, cx);
/// ```
///
/// A failed reload leaves the running view alone and reports on the log, the one
/// channel an embedded runtime can count on: it has no `ShellRoot` to raise a
/// toast on, and a host that wants one can watch for the log or drive
/// [`reload`] itself.
///
/// # What ends the loop
///
/// The view going away, the runtime going away, or the window closing —
/// whichever comes first, checked every tick.
///
/// Both handles are weak on purpose. A strong `Entity<ScriptView>` here would
/// keep a panel alive after the dock removed it: the view would never drop, the
/// runtime it points at would never drop, and the poller would go on stating the
/// directory for a panel nobody can see. Mount and unmount a few panels and the
/// pollers accumulate. So the loop holds nothing and asks each tick whether
/// there is still something to reload.
pub fn reload_in_debug(
    runtime: &Rc<ShellRuntime>,
    view: &Entity<ScriptView>,
    directory: PathBuf,
    entry: &'static str,
    window: &mut Window,
    cx: &mut App,
) {
    if !cfg!(debug_assertions) {
        return;
    }

    let handle = window.window_handle();
    let mut watcher = SourceWatcher::new(directory.clone());
    let runtime = Rc::downgrade(runtime);
    let view = view.downgrade();

    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(POLL_INTERVAL).await;

            let (Some(runtime), Some(view)) = (runtime.upgrade(), view.upgrade()) else {
                break;
            };

            if !watcher.poll() {
                continue;
            }

            let reached = handle.update(cx, |_, window, cx| {
                match reload(&runtime, &view, &directory, entry, window, cx) {
                    Ok(()) => tracing::info!("reloaded {}", directory.display()),
                    // `{error:#}` keeps the `anyhow` context chain, which is what
                    // names the file and the stage that failed.
                    Err(error) => tracing::error!("reload failed: {error:#}"),
                }
            });

            if reached.is_err() {
                break;
            }
        }
    })
    .detach();
}

pub fn reload(
    runtime: &Rc<ShellRuntime>,
    view: &Entity<ScriptView>,
    directory: &Path,
    entry: &str,
    window: &mut Window,
    cx: &mut App,
) -> Result<()> {
    if let Some(phase) = scope::current_phase()
        && !phase.allows_notify()
    {
        bail!(
            "a reload was requested during the {} phase; reload from an event, \
             a task, or a host timer instead",
            phase.as_str()
        );
    }

    // Everything that can fail runs first. On QuickJS this re-evaluates the
    // module into the same context, which §21.2 notes is one grade coarser than
    // a full teardown — an ES module cannot be unloaded, so old definitions stay
    // reachable from anything that captured them. Discarding and rebuilding the
    // whole context is the clean form, and belongs behind the engine seam
    // rather than here.
    let view_type = runtime
        .load_app(directory, entry)
        .with_context(|| format!("reloading {}", directory.display()))?;
    let object = runtime
        .instantiate(&view_type, window, cx)
        .with_context(|| format!("rebuilding the view from {}", directory.display()))?;

    view.update(cx, |view, cx| {
        view.replace_object(object);
        cx.notify();
    });
    window.refresh();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A unique directory under the system temp directory, removed on drop.
    ///
    /// The crate has no `tempfile` dependency and this is the only place that
    /// wants one, which is not enough reason to add it.
    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);

            let unique = format!(
                "gpui-shell-watch-{label}-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = std::env::temp_dir().join(unique);
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("creating the temporary tree");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write(&self, name: &str, contents: &str) {
            std::fs::write(self.0.join(name), contents).expect("writing a source file");
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn quiet_directory_reports_no_change() {
        let tree = TempTree::new("quiet");
        tree.write("main.js", "export default class {}\n");

        let mut watcher =
            SourceWatcher::new(tree.path().to_path_buf()).with_debounce(Duration::ZERO);

        assert!(!watcher.poll());
        assert!(!watcher.poll());
    }

    #[test]
    fn touched_file_reports_a_change_once() {
        let tree = TempTree::new("touched");
        tree.write("main.js", "export default class {}\n");

        let mut watcher =
            SourceWatcher::new(tree.path().to_path_buf()).with_debounce(Duration::ZERO);
        assert!(!watcher.poll());

        tree.write("main.js", "export default class { render() {} }\n");
        assert!(watcher.poll(), "an edited source file is a change");
        assert!(!watcher.poll(), "the change is reported once, not per poll");
    }

    #[test]
    fn non_source_file_is_ignored() {
        let tree = TempTree::new("non-source");
        tree.write("main.js", "export default class {}\n");

        let mut watcher =
            SourceWatcher::new(tree.path().to_path_buf()).with_debounce(Duration::ZERO);

        tree.write("README.md", "notes\n");
        assert!(!watcher.poll());
    }

    #[test]
    fn debounce_window_suppresses_a_burst() {
        let tree = TempTree::new("burst");
        tree.write("main.js", "export default class {}\n");

        let window = Duration::from_millis(300);
        let mut watcher = SourceWatcher::new(tree.path().to_path_buf()).with_debounce(window);

        // An editor's save: several writes in quick succession. None of them may
        // reload on its own, because the module is only whole after the last.
        tree.write("main.js", "");
        assert!(!watcher.poll());
        tree.write("main.js", "export default class { render() {} }\n");
        assert!(!watcher.poll());
        tree.write("helper.js", "export const helper = 1;\n");
        assert!(!watcher.poll());

        std::thread::sleep(window + Duration::from_millis(120));
        assert!(watcher.poll(), "the settled burst reloads exactly once");
        assert!(!watcher.poll());
    }

    #[test]
    fn missing_directory_does_not_panic() {
        let tree = TempTree::new("missing");
        tree.write("main.js", "export default class {}\n");

        let mut watcher =
            SourceWatcher::new(tree.path().to_path_buf()).with_debounce(Duration::ZERO);
        assert!(!watcher.poll());

        std::fs::remove_dir_all(tree.path()).expect("removing the tree");

        // Losing the tree is a change like any other, and then the watcher goes
        // quiet instead of reporting a change on every tick.
        assert!(watcher.poll());
        assert!(!watcher.poll());
    }

    #[test]
    fn never_existing_directory_is_quiet() {
        let path = std::env::temp_dir().join("gpui-shell-watch-does-not-exist");
        let mut watcher = SourceWatcher::new(path).with_debounce(Duration::ZERO);

        assert!(!watcher.poll());
        assert!(!watcher.poll());
    }

    #[test]
    fn nested_sources_are_watched_and_hidden_entries_are_not() {
        let tree = TempTree::new("nested");
        tree.write("main.js", "export default class {}\n");
        std::fs::create_dir_all(tree.path().join("lib")).expect("creating a nested directory");
        std::fs::create_dir_all(tree.path().join(".cache")).expect("creating a hidden directory");

        let mut watcher =
            SourceWatcher::new(tree.path().to_path_buf()).with_debounce(Duration::ZERO);

        std::fs::write(tree.path().join(".cache/main.js"), "ignored\n").expect("writing");
        assert!(
            !watcher.poll(),
            "hidden directories are not application source"
        );

        std::fs::write(tree.path().join("lib/helper.js"), "export const a = 1;\n")
            .expect("writing");
        assert!(watcher.poll(), "a nested source file is watched");
    }
}
