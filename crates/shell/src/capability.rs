//! What a script application is allowed to do.
//!
//! The default set is empty: a script gets no file, process, network, storage or
//! clipboard access until the host grants it. Grants come from a plugin manifest
//! (§18.1) or directly from the embedding application.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

/// A capability grant. Every field is private so adding a capability later is
/// not a breaking change for embedders.
#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    read_roots: Vec<PathBuf>,
    write_roots: Vec<PathBuf>,
    execute: ExecuteGrant,
    network_hosts: Vec<String>,
    store: bool,
    clipboard_read: bool,
    clipboard_write: bool,
}

/// Which external commands a script may run.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ExecuteGrant {
    /// `os.execute` is unavailable.
    #[default]
    Denied,
    /// Only these command names may run.
    Allowed(Vec<String>),
    /// Any command may run. Shown to the user at the highest severity.
    Unrestricted,
}

impl Capabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_read_roots(mut self, roots: impl IntoIterator<Item = PathBuf>) -> Self {
        self.read_roots = roots.into_iter().collect();
        self
    }

    pub fn with_write_roots(mut self, roots: impl IntoIterator<Item = PathBuf>) -> Self {
        self.write_roots = roots.into_iter().collect();
        self
    }

    pub fn with_execute(mut self, grant: ExecuteGrant) -> Self {
        self.execute = grant;
        self
    }

    pub fn with_network_hosts(mut self, hosts: impl IntoIterator<Item = String>) -> Self {
        self.network_hosts = hosts.into_iter().collect();
        self
    }

    pub fn store(mut self, allowed: bool) -> Self {
        self.store = allowed;
        self
    }

    pub fn clipboard_read(mut self, allowed: bool) -> Self {
        self.clipboard_read = allowed;
        self
    }

    pub fn clipboard_write(mut self, allowed: bool) -> Self {
        self.clipboard_write = allowed;
        self
    }

    pub fn has_store(&self) -> bool {
        self.store
    }

    pub fn is_clipboard_readable(&self) -> bool {
        self.clipboard_read
    }

    pub fn is_clipboard_writable(&self) -> bool {
        self.clipboard_write
    }

    pub fn execute(&self) -> &ExecuteGrant {
        &self.execute
    }

    /// Whether any filesystem write is permitted at all. `os.remove` and
    /// friends need this before their path is even resolved.
    pub fn has_write_access(&self) -> bool {
        !self.write_roots.is_empty()
    }

    pub fn has_read_access(&self) -> bool {
        !self.read_roots.is_empty()
    }

    pub fn may_run(&self, command: &str) -> bool {
        match &self.execute {
            ExecuteGrant::Denied => false,
            ExecuteGrant::Unrestricted => true,
            ExecuteGrant::Allowed(names) => names.iter().any(|name| name == command),
        }
    }

    pub fn may_reach(&self, host: &str) -> bool {
        self.network_hosts.iter().any(|allowed| allowed == host)
    }

    /// Resolves `path` against the granted roots, rejecting traversal.
    ///
    /// The same resolver serves `gpui.fs` and the capability-gated `os.*`
    /// functions, so there is no second path policy to keep in sync.
    ///
    /// # Symlinks
    ///
    /// A grant is a promise about a *directory*, and comparing strings cannot
    /// keep it: `inside/escape/passwd` is lexically under the root and reads
    /// `/etc/passwd` if `escape` is a link. So the check is made against the
    /// filesystem — see [`contain`] — rather than against the text.
    pub fn resolve(&self, path: &Path, access: Access) -> Result<PathBuf, CapabilityError> {
        let roots = match access {
            Access::Read => &self.read_roots,
            Access::Write => &self.write_roots,
        };
        if roots.is_empty() {
            return Err(CapabilityError::NotGranted(access));
        }

        for root in roots {
            // The root is resolved the same way a path is. On macOS the
            // temporary directory is reached through `/var`, which is a link to
            // `/private/var`, so a grant and a path resolved differently would
            // disagree about a directory neither of them left.
            let Some(root) = resolved_root(root) else {
                continue;
            };
            let candidate = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            // Lexical first: it costs nothing and rejects the ordinary `../..`
            // before anything touches the disk.
            let normalized = normalize(&candidate);
            if !normalized.starts_with(&root) {
                continue;
            }
            if let Some(contained) = contain(&normalized, &root) {
                return Ok(contained);
            }
        }

        Err(CapabilityError::OutsideRoots {
            path: path.to_path_buf(),
            access,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

impl Access {
    fn as_str(self) -> &'static str {
        match self {
            Access::Read => "read",
            Access::Write => "write",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum CapabilityError {
    NotGranted(Access),
    OutsideRoots { path: PathBuf, access: Access },
    ExecuteDenied(String),
    NetworkDenied(String),
    StoreDenied,
    ClipboardDenied,
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapabilityError::NotGranted(access) => write!(
                f,
                "filesystem {} is not granted; declare capabilities.fs.{} in the manifest",
                access.as_str(),
                access.as_str()
            ),
            CapabilityError::OutsideRoots { path, access } => write!(
                f,
                "`{}` is outside every granted {} root",
                path.display(),
                access.as_str()
            ),
            CapabilityError::ExecuteDenied(command) => write!(
                f,
                "running `{command}` is not granted; add it to capabilities.fs.execute in the manifest"
            ),
            CapabilityError::NetworkDenied(host) => {
                write!(f, "`{host}` is not in capabilities.network.hosts")
            }
            CapabilityError::StoreDenied => {
                f.write_str("storage is not granted; set capabilities.store to true")
            }
            CapabilityError::ClipboardDenied => {
                f.write_str("clipboard access is not granted; declare capabilities.clipboard")
            }
        }
    }
}

impl std::error::Error for CapabilityError {}

// ---------------------------------------------------------------------------
// The installed grant
// ---------------------------------------------------------------------------

thread_local! {
    /// The VM and GPUI's `App` are both main-thread only, so a thread-local is
    /// the whole story: no lock, and no `Send` bound forced onto
    /// [`Capabilities`] for the sake of a runtime that never leaves its thread.
    static INSTALLED: RefCell<Capabilities> = RefCell::new(Capabilities::default());
}

/// Installs the grant the loaded application runs under.
///
/// **Above the seam on purpose.** An earlier arrangement kept this inside the
/// QuickJS engine and had the crate root call into it, with a silent no-op for
/// any other build — which meant a second engine could compile, run, and ignore
/// the security configuration without anything saying so. A grant is a decision
/// about the *application*, not about the interpreter, and there is now nowhere
/// for an engine to answer it differently.
///
/// The host calls this before loading an application. Loading a different one
/// means calling it again; the default is [`Capabilities::default`], which
/// allows nothing.
pub fn install(capabilities: Capabilities) {
    INSTALLED.with_borrow_mut(|current| *current = capabilities);
}

/// The grant in force on this thread.
pub fn installed() -> Capabilities {
    INSTALLED.with_borrow(Clone::clone)
}

/// Holds a path to a root by asking the filesystem, not the string.
///
/// The deepest ancestor that resolves is canonicalized — which follows every
/// link on the way — and that real path must still be under the root. Whatever
/// is below it does not exist, so nothing there can be a link to anywhere:
/// `inside/new/file.txt` resolves through `inside`, and `inside/escape/x`
/// resolves through `/etc` and is refused.
///
/// The one component that *can* exist below the deepest resolvable ancestor is
/// the first: a dangling symlink resolves to nothing while still being
/// something a write would follow and create. That is refused rather than
/// guessed at, because where it points cannot be proven.
///
/// # What this does not close
///
/// A component can still be replaced with a link between this check and the
/// syscall that follows it. Closing that needs descriptor-relative opens —
/// `openat2(RESOLVE_BENEATH)` on Linux, `O_NOFOLLOW` walks elsewhere — which is
/// a different piece of work and not portable in `std`. The window is narrow
/// and requires a second writer inside the root; the escape this replaces
/// needed only a link that was already there.
/// Whether a path is really inside a directory, for a caller outside this
/// module.
///
/// The asset source asks the same question the `fs` grant does — "is this file
/// actually in that directory?" — and asking it through one implementation is
/// what keeps a symlink from being refused by one and followed by the other.
pub fn contained_in(candidate: &Path, root: &Path) -> Option<PathBuf> {
    contain(candidate, &resolved_root(root)?)
}

fn contain(candidate: &Path, root: &Path) -> Option<PathBuf> {
    let (resolved, tail) = deepest_resolvable(candidate)?;

    let mut out = resolved;
    for (depth, name) in tail.iter().enumerate() {
        out.push(name);
        // Only the first component below the resolved ancestor can exist, and
        // if it does it is a link that resolves to nothing — something a write
        // would follow and create, at a target that cannot be proven.
        if depth == 0 && std::fs::symlink_metadata(&out).is_ok() {
            return None;
        }
    }

    // Checked on the reconstructed path rather than on the resolved ancestor,
    // because a root that does not exist yet — an application's data directory
    // on first run — resolves to an ancestor *above* itself.
    out.starts_with(root).then_some(out)
}

/// A root in the same form a resolved path takes, so the two can be compared.
///
/// A granted directory need not exist yet, so this cannot simply canonicalize:
/// it resolves as far as the filesystem goes and keeps the rest verbatim, which
/// is safe for exactly the reason [`contain`] relies on — what does not exist
/// cannot be a link.
fn resolved_root(root: &Path) -> Option<PathBuf> {
    let (resolved, tail) = deepest_resolvable(root)?;
    let mut out = resolved;
    out.extend(tail.iter());
    Some(out)
}

/// Splits a path into its deepest ancestor that resolves, canonicalized, and
/// the components below it in order.
///
/// `canonicalize` fails for a path that does not exist and for one that dangles,
/// which is exactly the set that has to be walked past — and it follows every
/// link on the part that does resolve, which is the point.
fn deepest_resolvable(path: &Path) -> Option<(PathBuf, Vec<std::ffi::OsString>)> {
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut current = path.to_path_buf();

    let resolved = loop {
        if let Ok(real) = current.canonicalize() {
            break real;
        }
        let name = current.file_name()?.to_os_string();
        if !current.pop() {
            return None;
        }
        tail.push(name);
    };

    tail.reverse();
    Some((resolved, tail))
}

/// Lexical normalization, which is the cheap half of the check. [`contain`] is
/// the half that holds when a component is a symlink.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn granted() -> Capabilities {
        Capabilities::new()
            .with_read_roots([PathBuf::from("/app/data")])
            .with_write_roots([PathBuf::from("/app/data")])
    }

    #[test]
    fn traversal_out_of_a_root_is_rejected() {
        let error = granted()
            .resolve(Path::new("../../etc/passwd"), Access::Read)
            .unwrap_err();
        assert!(matches!(error, CapabilityError::OutsideRoots { .. }));
    }

    #[test]
    fn a_relative_path_resolves_inside_its_root() {
        let path = granted()
            .resolve(Path::new("items.json"), Access::Write)
            .unwrap();
        assert_eq!(path, PathBuf::from("/app/data/items.json"));
    }

    #[test]
    fn nothing_resolves_without_a_grant() {
        let error = Capabilities::new()
            .resolve(Path::new("items.json"), Access::Read)
            .unwrap_err();
        assert_eq!(error, CapabilityError::NotGranted(Access::Read));
    }

    #[test]
    fn execute_is_denied_by_default_and_allowlisted_when_granted() {
        assert!(!Capabilities::new().may_run("git"));
        let capabilities =
            Capabilities::new().with_execute(ExecuteGrant::Allowed(vec!["git".into()]));
        assert!(capabilities.may_run("git"));
        assert!(!capabilities.may_run("curl"));
    }
}

#[cfg(test)]
mod symlink_tests {
    use super::*;

    /// A grant is a promise about a *directory*, and a symlink is the oldest
    /// way to make a path that is lexically inside one point somewhere else.
    /// These use real files, because the whole question is what the filesystem
    /// does rather than what the string looks like.
    struct Sandbox {
        root: PathBuf,
    }

    impl Sandbox {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("gpui-shell-escape-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("inside")).expect("a sandbox");
            std::fs::write(root.join("inside/ours.txt"), "ours").expect("a file");
            Self {
                root: root.canonicalize().expect("a canonical sandbox"),
            }
        }

        /// Somewhere outside the grant, standing in for `/etc`.
        fn outside(&self) -> PathBuf {
            let outside = self.root.join("outside");
            std::fs::create_dir_all(&outside).expect("somewhere outside");
            std::fs::write(outside.join("secret.txt"), "secret").expect("a secret");
            outside
        }

        fn granted(&self) -> Capabilities {
            Capabilities::new()
                .with_read_roots([self.root.join("inside")])
                .with_write_roots([self.root.join("inside")])
        }

        fn link(&self, name: &str, target: &Path) {
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, self.root.join("inside").join(name))
                .expect("a symlink");
            #[cfg(windows)]
            let _ = std::os::windows::fs::symlink_dir(target, self.root.join("inside").join(name));
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    #[cfg(unix)]
    fn a_symlink_out_of_a_root_does_not_resolve() {
        let sandbox = Sandbox::new("read");
        let outside = sandbox.outside();
        sandbox.link("escape", &outside);
        let capabilities = sandbox.granted();

        // The lexical path is under the root; the file it names is not.
        let escaped = capabilities.resolve(Path::new("escape/secret.txt"), Access::Read);
        assert!(
            escaped.is_err(),
            "reading through a symlink left the granted root: {escaped:?}"
        );

        // The link itself is no better an answer than a path through it.
        assert!(
            capabilities
                .resolve(Path::new("escape"), Access::Read)
                .is_err()
        );

        // And an ordinary path still works, which is the point of the grant.
        assert!(
            capabilities
                .resolve(Path::new("ours.txt"), Access::Read)
                .is_ok()
        );
    }

    #[test]
    #[cfg(unix)]
    fn a_symlink_out_of_a_root_cannot_be_written_through() {
        let sandbox = Sandbox::new("write");
        let outside = sandbox.outside();
        sandbox.link("escape", &outside);
        let capabilities = sandbox.granted();

        // A file that does not exist yet, below a link that does.
        let escaped = capabilities.resolve(Path::new("escape/planted.txt"), Access::Write);
        assert!(
            escaped.is_err(),
            "writing through a symlink left the granted root: {escaped:?}"
        );

        // Creating a directory through it is the same escape with a different
        // syscall.
        assert!(
            capabilities
                .resolve(Path::new("escape/planted"), Access::Write)
                .is_err()
        );

        // A new file directly in the root is still allowed.
        assert!(
            capabilities
                .resolve(Path::new("new.txt"), Access::Write)
                .is_ok()
        );
    }

    #[test]
    #[cfg(unix)]
    fn a_dangling_symlink_is_refused_rather_than_followed() {
        let sandbox = Sandbox::new("dangling");
        let outside = sandbox.outside();
        sandbox.link("dangling", &outside.join("not-there.txt"));
        let capabilities = sandbox.granted();

        // Nothing is there to canonicalize, so where it points cannot be
        // proven — and a write would create the target, outside the root.
        let escaped = capabilities.resolve(Path::new("dangling"), Access::Write);
        assert!(
            escaped.is_err(),
            "a dangling symlink was treated as a path that may be created: {escaped:?}"
        );
    }
}
