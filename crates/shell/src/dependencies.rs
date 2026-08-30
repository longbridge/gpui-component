use std::{
    fs::File,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, bail};
use fs2::FileExt as _;
use sha2::{Digest as _, Sha256};
use wait_timeout::ChildExt as _;

use crate::plugin::GitDependency;

/// Materializes Git-backed JavaScript packages in gpui-shell's user cache.
pub(crate) struct GitDependencyStore {
    root: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct MaterializedDependency {
    pub(crate) root: PathBuf,
    pub(crate) entry: PathBuf,
}

impl GitDependencyStore {
    pub(crate) fn for_user() -> Result<Self> {
        Self::for_user_with_environment(|variable| std::env::var_os(variable))
    }

    fn for_user_with_environment(
        environment: impl Fn(&str) -> Option<std::ffi::OsString>,
    ) -> Result<Self> {
        let Some((variable, home)) = ["HOME", "USERPROFILE"].into_iter().find_map(|variable| {
            environment(variable)
                .filter(|value| !value.is_empty())
                .map(|value| (variable, PathBuf::from(value)))
        }) else {
            bail!(
                "cannot locate the Git dependency cache: HOME or USERPROFILE must name an absolute user directory"
            );
        };
        if !home.is_absolute() {
            bail!(
                "cannot locate the Git dependency cache: {variable} must be an absolute path, got `{}`",
                home.display()
            );
        }
        Ok(Self::new(dependency_cache_root(&home)))
    }

    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn materialize(
        &self,
        name: &str,
        dependency: &GitDependency,
    ) -> Result<MaterializedDependency> {
        std::fs::create_dir_all(&self.root)
            .with_context(|| format!("creating Git dependency cache {}", self.root.display()))?;
        let remote_key = digest(&[("git", dependency.git())]);
        let locks = self.root.join("locks");
        let mirrors = self.root.join("mirrors");
        let checkouts = self.root.join("checkouts").join(&remote_key);
        std::fs::create_dir_all(&locks)?;
        std::fs::create_dir_all(&mirrors)?;
        std::fs::create_dir_all(&checkouts)?;
        let _lock = CacheLock::acquire(&locks.join(format!("{remote_key}.lock")), name)?;

        let mirror = mirrors.join(format!("{remote_key}.git"));
        if !mirror.is_dir() {
            let temporary = temporary_path(&mirrors, &remote_key);
            let mut command = git_command();
            command
                .args(["clone", "--mirror", "--"])
                .arg(dependency.git())
                .arg(&temporary);
            if let Err(error) = run_command(name, "clone", command) {
                let _ = std::fs::remove_dir_all(&temporary);
                return Err(error);
            }
            match std::fs::rename(&temporary, &mirror) {
                Ok(()) => {}
                Err(error) if mirror.is_dir() => {
                    let _ = std::fs::remove_dir_all(&temporary);
                    tracing::debug!("another process published {}: {error}", mirror.display());
                }
                Err(error) => return Err(error).context("publishing Git dependency mirror"),
            }
        }

        let mut origin = git_command();
        origin.args(["remote", "get-url", "origin"]);
        origin.current_dir(&mirror);
        let configured = output_text(name, "inspect cached origin", origin)?;
        if configured.trim() != dependency.git() {
            bail!(
                "Git dependency `{name}` cache origin is `{}`, expected `{}`; remove {} and retry",
                configured.trim(),
                dependency.git(),
                mirror.display()
            );
        }

        let reference = match (dependency.branch(), dependency.tag()) {
            (Some(branch), None) => format!("refs/heads/{branch}"),
            (None, Some(tag)) => format!("refs/tags/{tag}"),
            _ => unreachable!("manifest validation requires exactly one Git ref"),
        };
        let mut fetch = git_command();
        fetch.args(["fetch", "--force", "--depth", "1", "origin", &reference]);
        fetch.current_dir(&mirror);
        run_command(name, "fetch", fetch)?;

        let mut rev_parse = git_command();
        rev_parse.args(["rev-parse", "FETCH_HEAD"]);
        rev_parse.current_dir(&mirror);
        let commit = output_text(name, "resolve fetched commit", rev_parse)?;
        let commit = commit.trim();
        if !(commit.len() == 40 || commit.len() == 64)
            || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("Git dependency `{name}` resolved an invalid commit id `{commit}`");
        }

        let checkout = checkouts.join(commit);
        if !checkout.join(".git").is_dir() {
            let temporary = temporary_path(&checkouts, commit);
            let mut clone = git_command();
            clone
                .args(["clone", "--no-checkout", "--"])
                .arg(&mirror)
                .arg(&temporary);
            if let Err(error) = run_command(name, "create immutable checkout", clone) {
                let _ = std::fs::remove_dir_all(&temporary);
                return Err(error);
            }
            let mut checkout_commit = git_command();
            checkout_commit
                .args(["checkout", "--force", "--detach", commit])
                .current_dir(&temporary);
            if let Err(error) = run_command(name, "checkout fetched commit", checkout_commit) {
                let _ = std::fs::remove_dir_all(&temporary);
                return Err(error);
            }
            match std::fs::rename(&temporary, &checkout) {
                Ok(()) => {}
                Err(error) if checkout.join(".git").is_dir() => {
                    let _ = std::fs::remove_dir_all(&temporary);
                    tracing::debug!("another process published {}: {error}", checkout.display());
                }
                Err(error) => return Err(error).context("publishing Git dependency checkout"),
            }
        }

        let root = checkout
            .canonicalize()
            .with_context(|| format!("resolving dependency checkout {}", checkout.display()))?;
        let entry = root
            .join(dependency.entry())
            .canonicalize()
            .with_context(|| {
                format!(
                    "Git dependency `{name}` has no entry `{}`",
                    dependency.entry()
                )
            })?;
        if !entry.starts_with(&root) || !entry.is_file() {
            bail!(
                "Git dependency `{name}` entry `{}` is not a file inside its checkout",
                dependency.entry()
            );
        }

        Ok(MaterializedDependency { root, entry })
    }
}

fn dependency_cache_root(home: &Path) -> PathBuf {
    home.join(".gpui-shell").join("cache").join("dependencies")
}

const GIT_TIMEOUT: Duration = Duration::from_secs(30);
const LOCK_TIMEOUT: Duration = Duration::from_secs(2 * 60);

fn git_command() -> Command {
    let mut command = Command::new("git");
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn run_command(name: &str, operation: &str, mut command: Command) -> Result<Output> {
    let mut child = command
        .spawn()
        .with_context(|| format!("starting git to {operation} dependency `{name}`"))?;
    let status = child
        .wait_timeout(GIT_TIMEOUT)
        .with_context(|| format!("waiting for git to {operation} dependency `{name}`"))?;
    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        bail!(
            "git timed out after {} seconds while trying to {operation} dependency `{name}`",
            GIT_TIMEOUT.as_secs()
        );
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(output);
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    bail!(
        "could not {operation} Git dependency `{name}`: {}",
        detail.trim()
    )
}

fn output_text(name: &str, operation: &str, command: Command) -> Result<String> {
    let output = run_command(name, operation, command)?;
    String::from_utf8(output.stdout).with_context(|| {
        format!("git returned non-UTF-8 output while trying to {operation} `{name}`")
    })
}

fn digest(fields: &[(&str, &str)]) -> String {
    let mut digest = Sha256::new();
    for (kind, value) in fields {
        digest.update(kind.len().to_le_bytes());
        digest.update(kind.as_bytes());
        digest.update(value.len().to_le_bytes());
        digest.update(value.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn temporary_path(parent: &Path, label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    parent.join(format!(
        ".{label}.tmp-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

struct CacheLock(File);

impl CacheLock {
    fn acquire(path: &Path, name: &str) -> Result<Self> {
        let file = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("opening Git dependency cache lock {}", path.display()))?;
        let started = Instant::now();
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self(file)),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if started.elapsed() >= LOCK_TIMEOUT {
                        bail!(
                            "timed out waiting for another process to finish Git dependency `{name}`"
                        );
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(error) => return Err(error).context("locking Git dependency cache"),
            }
        }
    }
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::{GitDependencyStore, dependency_cache_root};
    use crate::plugin::PluginManifest;
    use std::{
        ffi::OsString,
        path::{Path, PathBuf},
        process::Command,
        sync::{
            Arc, Barrier,
            atomic::{AtomicU64, Ordering},
        },
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn a_user_dependency_cache_lives_in_the_shell_cache() {
        assert_eq!(
            dependency_cache_root(Path::new("/home/example")),
            PathBuf::from("/home/example/.gpui-shell/cache/dependencies")
        );
    }

    #[test]
    fn for_user_wires_home_to_the_shell_cache_root() {
        let store = GitDependencyStore::for_user_with_environment(|variable| match variable {
            "HOME" => Some(OsString::from("/home/example")),
            _ => None,
        })
        .expect("an absolute HOME should select a private cache root");

        assert_eq!(
            store.root,
            PathBuf::from("/home/example/.gpui-shell/cache/dependencies")
        );
    }

    #[test]
    fn for_user_uses_userprofile_when_home_is_missing() {
        let store = GitDependencyStore::for_user_with_environment(|variable| match variable {
            "USERPROFILE" => Some(OsString::from("/profiles/example")),
            _ => None,
        })
        .expect("an absolute USERPROFILE should select a private cache root");

        assert_eq!(
            store.root,
            PathBuf::from("/profiles/example/.gpui-shell/cache/dependencies")
        );
    }

    #[test]
    fn for_user_ignores_an_empty_home_before_userprofile() {
        let store = GitDependencyStore::for_user_with_environment(|variable| match variable {
            "HOME" => Some(OsString::new()),
            "USERPROFILE" => Some(OsString::from("/profiles/example")),
            _ => None,
        })
        .expect("an empty HOME should allow an absolute USERPROFILE");

        assert_eq!(
            store.root,
            PathBuf::from("/profiles/example/.gpui-shell/cache/dependencies")
        );
    }

    #[test]
    fn for_user_rejects_missing_or_empty_home_variables() {
        for (home, userprofile) in [(None, None), (Some(OsString::new()), Some(OsString::new()))] {
            let result = GitDependencyStore::for_user_with_environment(|variable| match variable {
                "HOME" => home.clone(),
                "USERPROFILE" => userprofile.clone(),
                _ => None,
            });
            let error = result.err().expect("a private home directory is required");

            assert!(
                error.to_string().contains("HOME or USERPROFILE"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn for_user_rejects_a_relative_selected_home() {
        let result = GitDependencyStore::for_user_with_environment(|variable| match variable {
            "HOME" => Some(OsString::from("relative/home")),
            "USERPROFILE" => Some(OsString::from("/profiles/example")),
            _ => None,
        });
        let error = result
            .err()
            .expect("a relative HOME must not select a shared working-directory cache");

        assert!(error.to_string().contains("HOME"), "{error:#}");
        assert!(error.to_string().contains("absolute"), "{error:#}");
        assert!(error.to_string().contains("relative/home"), "{error:#}");
    }

    struct GitFixture {
        root: PathBuf,
        remote: PathBuf,
        cache: PathBuf,
    }

    impl GitFixture {
        fn new() -> Self {
            let unique = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "gpui-shell-git-dependency-{}-{unique}",
                std::process::id()
            ));
            let remote = root.join("remote");
            let cache = root.join("cache");
            std::fs::create_dir_all(&remote).expect("fixture directory");
            git(&remote, &["init", "--initial-branch=main"]);
            git(&remote, &["config", "user.name", "gpui-shell test"]);
            git(
                &remote,
                &["config", "user.email", "gpui-shell@example.invalid"],
            );
            Self {
                root,
                remote,
                cache,
            }
        }

        fn commit(&self, source: &str, message: &str) {
            std::fs::write(self.remote.join("index.js"), source).expect("dependency source");
            git(&self.remote, &["add", "index.js"]);
            git(&self.remote, &["commit", "-m", message]);
        }

        fn dependency(&self, selector: &str) -> crate::plugin::GitDependency {
            let manifest = format!(
                r#"{{
                    "id": "com.example.fixture",
                    "name": "Fixture",
                    "entry": "main.js",
                    "dependencies": {{
                        "omarchy-ui": {{
                            "git": {},
                            {selector}
                        }}
                    }}
                }}"#,
                serde_json::to_string(&self.remote).expect("remote path as JSON")
            );
            PluginManifest::parse(&manifest)
                .expect("fixture manifest")
                .dependencies()["omarchy-ui"]
                .clone()
        }
    }

    impl Drop for GitFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn git(directory: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .output()
            .expect("git must be installed for the test");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn a_branch_dependency_refreshes_to_the_remote_head_on_each_materialization() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "first");
        let dependency = fixture.dependency(r#""branch": "main""#);
        let store = GitDependencyStore::new(fixture.cache.clone());

        let first = store
            .materialize("omarchy-ui", &dependency)
            .expect("first checkout");
        assert_eq!(
            std::fs::read_to_string(&first.entry).unwrap(),
            "export const version = 1;"
        );

        fixture.commit("export const version = 2;", "second");
        let second = store
            .materialize("omarchy-ui", &dependency)
            .expect("updated checkout");
        assert_eq!(
            std::fs::read_to_string(&second.entry).unwrap(),
            "export const version = 2;"
        );
        assert_eq!(
            std::fs::read_to_string(&first.entry).unwrap(),
            "export const version = 1;",
            "a refresh must not mutate a checkout retained by an older module generation"
        );
    }

    #[test]
    fn a_tag_dependency_stays_at_the_tagged_commit() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "tagged");
        git(&fixture.remote, &["tag", "v1"]);
        fixture.commit("export const version = 2;", "later");
        let dependency = fixture.dependency(r#""tag": "v1""#);
        let store = GitDependencyStore::new(fixture.cache.clone());

        let package = store
            .materialize("omarchy-ui", &dependency)
            .expect("tag checkout");
        assert_eq!(
            std::fs::read_to_string(&package.entry).unwrap(),
            "export const version = 1;"
        );
    }

    #[test]
    fn concurrent_materializations_publish_one_valid_checkout() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "first");
        let dependency = fixture.dependency(r#""branch": "main""#);
        let barrier = Arc::new(Barrier::new(2));

        let workers: Vec<_> = (0..2)
            .map(|_| {
                let cache = fixture.cache.clone();
                let dependency = dependency.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    GitDependencyStore::new(cache)
                        .materialize("omarchy-ui", &dependency)
                        .expect("concurrent checkout")
                })
            })
            .collect();
        let packages: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().expect("materialization worker"))
            .collect();

        assert_eq!(packages[0].root, packages[1].root);
        assert_eq!(
            std::fs::read_to_string(&packages[0].entry).unwrap(),
            "export const version = 1;"
        );
    }

    #[test]
    fn a_cached_mirror_with_the_wrong_origin_is_refused() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "first");
        let dependency = fixture.dependency(r#""branch": "main""#);
        let store = GitDependencyStore::new(fixture.cache.clone());
        store
            .materialize("omarchy-ui", &dependency)
            .expect("initial checkout");
        let mirror = std::fs::read_dir(fixture.cache.join("mirrors"))
            .expect("mirror directory")
            .next()
            .expect("one mirror")
            .expect("mirror entry")
            .path();
        git(&mirror, &["remote", "set-url", "origin", "/wrong/remote"]);

        let error = store
            .materialize("omarchy-ui", &dependency)
            .expect_err("a cache may not silently change repository identity");
        assert!(error.to_string().contains("cache origin"), "{error:#}");
    }
}
