//! What a script application is allowed to do.
//!
//! The default set is empty: a script gets no file, process, network, storage or
//! clipboard access until the host grants it. Grants come from a plugin manifest
//! (§18.1) or directly from the embedding application.

use std::path::{Path, PathBuf};

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
    pub fn resolve(&self, path: &Path, access: Access) -> Result<PathBuf, CapabilityError> {
        let roots = match access {
            Access::Read => &self.read_roots,
            Access::Write => &self.write_roots,
        };
        if roots.is_empty() {
            return Err(CapabilityError::NotGranted(access));
        }

        for root in roots {
            let candidate = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            let normalized = normalize(&candidate);
            if normalized.starts_with(root) {
                return Ok(normalized);
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

/// Lexical normalization. Symlink resolution happens at the syscall, where the
/// resolved path is checked again.
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
