//! Bounded child-process execution for the standard runtime adapter.

use std::{
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

const STDOUT_EXCEEDED: u8 = 1;
const STDERR_EXCEEDED: u8 = 2;

#[derive(Clone)]
pub(crate) struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub(crate) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Limits {
    timeout: Duration,
    stdout: usize,
    stderr: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            stdout: 8 * 1024 * 1024,
            stderr: 8 * 1024 * 1024,
        }
    }
}

#[cfg(test)]
impl Limits {
    fn for_test(timeout: Duration, output: usize) -> Self {
        Self {
            timeout,
            stdout: output,
            stderr: output,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Output {
    pub(crate) code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn run_bounded(
    command: &str,
    args: &[String],
    limits: Limits,
    cancellation: Cancellation,
) -> Result<Output, String> {
    let executable = resolve_executable(command, std::env::var_os("PATH").as_deref())?;
    let mut command_builder = Command::new(&executable);
    command_builder
        // A process grant authorizes one executable, not the host's ambient
        // credentials. Keep the child environment empty until the public API
        // grows an explicit, capability-reviewed environment allowlist.
        .env_clear()
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_tree(&mut command_builder);
    let mut process_tree = ProcessTree::new()?;
    let mut child = command_builder
        .spawn()
        .map_err(|error| format!("running `{command}` failed: {error}"))?;
    if let Err(error) = process_tree.attach(&child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "isolating `{command}` process tree failed: {error}"
        ));
    }

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let exceeded = Arc::new(AtomicU8::new(0));
    let stdout_reader = read_stream(stdout, limits.stdout, STDOUT_EXCEEDED, exceeded.clone());
    let stderr_reader = read_stream(stderr, limits.stderr, STDERR_EXCEEDED, exceeded.clone());
    let started = Instant::now();
    let mut status = None;
    let mut stdout = None;
    let mut stderr = None;

    loop {
        if let Err(error) = receive_stream(&stdout_reader, &mut stdout, "stdout")
            .and_then(|()| receive_stream(&stderr_reader, &mut stderr, "stderr"))
        {
            kill_process_tree(&mut process_tree, &mut child);
            let _ = child.wait();
            return Err(error);
        }

        let reason = if cancellation.is_cancelled() {
            Some("was cancelled".to_owned())
        } else if started.elapsed() >= limits.timeout {
            Some(format!("timed out after {} ms", limits.timeout.as_millis()))
        } else {
            match exceeded.load(Ordering::Acquire) {
                STDOUT_EXCEEDED => Some(format!("stdout exceeded {} bytes", limits.stdout)),
                STDERR_EXCEEDED => Some(format!("stderr exceeded {} bytes", limits.stderr)),
                _ => None,
            }
        };

        if let Some(reason) = reason {
            kill_process_tree(&mut process_tree, &mut child);
            let _ = child.wait();
            return Err(format!("`{command}` {reason}"));
        }

        if status.is_none() {
            match child.try_wait() {
                Ok(Some(done)) => status = Some(done),
                Ok(None) => {}
                Err(error) => {
                    kill_process_tree(&mut process_tree, &mut child);
                    let _ = child.wait();
                    return Err(format!("waiting for `{command}` failed: {error}"));
                }
            }
        }

        if status.is_some() && stdout.is_some() && stderr.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    let status = status.expect("loop exits only with child status");
    let stdout = stdout.expect("loop exits only after stdout closes")?;
    let stderr = stderr.expect("loop exits only after stderr closes")?;
    Ok(Output {
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

fn resolve_executable(command: &str, path: Option<&OsStr>) -> Result<PathBuf, String> {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return Ok(command_path.to_path_buf());
    }
    let path = path.ok_or_else(|| {
        format!("running `{command}` failed: the host PATH environment variable is not set")
    })?;
    for directory in std::env::split_paths(path) {
        let candidate = directory.join(command);
        if executable_file(&candidate) {
            return Ok(candidate);
        }
        #[cfg(windows)]
        if candidate.extension().is_none() {
            for extension in ["exe", "com", "bat", "cmd"] {
                let candidate = candidate.with_extension(extension);
                if executable_file(&candidate) {
                    return Ok(candidate);
                }
            }
        }
    }
    Err(format!(
        "running `{command}` failed: executable was not found on the host PATH"
    ))
}

#[cfg(unix)]
fn executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable_file(path: &Path) -> bool {
    path.is_file()
}

fn read_stream(
    mut stream: impl Read + Send + 'static,
    limit: usize,
    flag: u8,
    exceeded: Arc<AtomicU8>,
) -> mpsc::Receiver<Result<Vec<u8>, String>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = (|| {
            let mut output = Vec::new();
            let mut chunk = [0_u8; 8192];
            loop {
                let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                if count == 0 {
                    return Ok(output);
                }
                if output.len().saturating_add(count) > limit {
                    let _ = exceeded.compare_exchange(0, flag, Ordering::AcqRel, Ordering::Acquire);
                    return Err(format!("exceeded {limit} bytes"));
                }
                output.extend_from_slice(&chunk[..count]);
            }
        })();
        let _ = sender.send(result);
    });
    receiver
}

fn receive_stream(
    reader: &mpsc::Receiver<Result<Vec<u8>, String>>,
    output: &mut Option<Result<Vec<u8>, String>>,
    name: &str,
) -> Result<(), String> {
    if output.is_some() {
        return Ok(());
    }
    match reader.try_recv() {
        Ok(result) => *output = Some(result.map_err(|error| format!("child {name} {error}"))),
        Err(mpsc::TryRecvError::Empty) => {}
        Err(mpsc::TryRecvError::Disconnected) => {
            return Err(format!("reading child {name} stopped unexpectedly"));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn configure_process_tree(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

#[cfg(windows)]
fn configure_process_tree(command: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    use windows::Win32::System::Threading::CREATE_SUSPENDED;

    // Assignment to a Job Object is not an atomic option on `Command`. Keep
    // the primary thread suspended so the process cannot create a descendant
    // in the interval between `spawn` and `AssignProcessToJobObject`.
    command.creation_flags(CREATE_SUSPENDED.0);
}

#[cfg(not(any(unix, windows)))]
fn configure_process_tree(_: &mut Command) {}

#[cfg(unix)]
struct ProcessTree;

#[cfg(unix)]
impl ProcessTree {
    fn new() -> Result<Self, String> {
        Ok(Self)
    }

    fn attach(&mut self, _: &std::process::Child) -> Result<(), String> {
        Ok(())
    }

    fn terminate(&mut self, child: &mut std::process::Child) {
        // The child is the leader of the process group configured above. A
        // negative pid targets the whole group, including descendants that
        // inherited stdout/stderr and would otherwise keep the readers alive.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
struct ProcessTree {
    job: Option<windows::Win32::Foundation::HANDLE>,
}

#[cfg(windows)]
impl ProcessTree {
    fn new() -> Result<Self, String> {
        use windows::{
            Win32::System::JobObjects::{
                CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
            core::PCWSTR,
        };

        let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
            .map_err(|error| format!("creating Windows Job Object failed: {error}"))?;
        let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if let Err(error) = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&information).cast(),
                std::mem::size_of_val(&information) as u32,
            )
        } {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(job);
            }
            return Err(format!("configuring Windows Job Object failed: {error}"));
        }
        Ok(Self { job: Some(job) })
    }

    fn attach(&mut self, child: &std::process::Child) -> Result<(), String> {
        use std::os::windows::io::AsRawHandle as _;
        use windows::Win32::{Foundation::HANDLE, System::JobObjects::AssignProcessToJobObject};

        let job = self.job.expect("a live process tree owns its job");
        let process = HANDLE(child.as_raw_handle());
        unsafe { AssignProcessToJobObject(job, process) }
            .map_err(|error| format!("assigning child to Windows Job Object failed: {error}"))?;
        resume_suspended_process(child.id())
    }

    fn terminate(&mut self, _: &mut std::process::Child) {
        self.close();
    }

    fn close(&mut self) {
        if let Some(job) = self.job.take() {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(job);
            }
        }
    }
}

#[cfg(windows)]
fn resume_suspended_process(process_id: u32) -> Result<(), String> {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
        },
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }
        .map_err(|error| format!("snapshotting Windows threads failed: {error}"))?;
    let result = (|| {
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        unsafe { Thread32First(snapshot, &mut entry) }
            .map_err(|error| format!("enumerating Windows threads failed: {error}"))?;
        let thread_id = loop {
            if entry.th32OwnerProcessID == process_id {
                break entry.th32ThreadID;
            }
            if unsafe { Thread32Next(snapshot, &mut entry) }.is_err() {
                return Err(format!(
                    "the suspended Windows process {process_id} had no primary thread"
                ));
            }
        };
        let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, thread_id) }
            .map_err(|error| format!("opening the suspended Windows thread failed: {error}"))?;
        let resumed = unsafe { ResumeThread(thread) };
        unsafe {
            let _ = CloseHandle(thread);
        }
        if resumed == u32::MAX {
            return Err(format!(
                "resuming the suspended Windows thread failed: {}",
                windows::core::Error::from_win32()
            ));
        }
        Ok(())
    })();
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    result
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(not(any(unix, windows)))]
struct ProcessTree;

#[cfg(not(any(unix, windows)))]
impl ProcessTree {
    fn new() -> Result<Self, String> {
        Ok(Self)
    }

    fn attach(&mut self, _: &std::process::Child) -> Result<(), String> {
        Ok(())
    }

    fn terminate(&mut self, _: &mut std::process::Child) {}
}

fn kill_process_tree(tree: &mut ProcessTree, child: &mut std::process::Child) {
    tree.terminate(child);
    let _ = child.kill();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[cfg(unix)]
    use std::ffi::OsStr;

    #[cfg(unix)]
    #[test]
    fn resolves_a_bare_command_from_the_host_path_before_clearing_the_environment() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-command-path-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("test directory");
        let executable = directory.join("outside-default-path");
        std::fs::write(&executable, "#!/bin/sh\nprintf resolved").expect("test executable");
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .expect("executable permissions");

        assert_eq!(
            resolve_executable("outside-default-path", Some(directory.as_os_str()))
                .expect("command on PATH"),
            executable
        );
        assert!(resolve_executable("missing", Some(OsStr::new("/not/here"))).is_err());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[cfg(unix)]
    #[test]
    fn captures_a_successful_command() {
        let result = run_bounded(
            "/bin/sh",
            &["-c".into(), "printf out; printf err >&2".into()],
            Limits::for_test(Duration::from_secs(2), 1024),
            Cancellation::new(),
        )
        .expect("command");
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "out");
        assert_eq!(result.stderr, "err");
    }

    #[cfg(unix)]
    #[test]
    fn child_does_not_inherit_the_host_environment() {
        let result = run_bounded(
            "/usr/bin/env",
            &[],
            Limits::for_test(Duration::from_secs(2), 1024),
            Cancellation::new(),
        )
        .expect("environment probe");

        assert_eq!(result.stdout, "");
        assert_eq!(result.stderr, "");
    }

    #[cfg(unix)]
    #[test]
    fn kills_a_command_that_times_out() {
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "sleep 5".into()],
            Limits::for_test(Duration::from_millis(30), 1024),
            Cancellation::new(),
        )
        .expect_err("timeout");
        assert!(error.contains("timed out"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn a_descendant_inheriting_the_pipes_cannot_extend_the_timeout() {
        let started = Instant::now();
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "(sleep 5) & exit 0".into()],
            Limits::for_test(Duration::from_millis(50), 1024),
            Cancellation::new(),
        )
        .expect_err("the inherited pipes must remain under the deadline");
        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(unix)]
    #[test]
    fn kills_a_command_whose_stdout_exceeds_the_limit() {
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "yes x | head -c 4096".into()],
            Limits::for_test(Duration::from_secs(2), 128),
            Cancellation::new(),
        )
        .expect_err("limit");
        assert!(error.contains("stdout exceeded"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn kills_a_command_whose_stderr_exceeds_the_limit() {
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "yes x | head -c 4096 >&2".into()],
            Limits::for_test(Duration::from_secs(2), 128),
            Cancellation::new(),
        )
        .expect_err("limit");
        assert!(error.contains("stderr exceeded"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_kills_and_reaps_the_command() {
        let cancellation = Cancellation::new();
        cancellation.cancel();
        let started = std::time::Instant::now();
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "sleep 5".into()],
            Limits::for_test(Duration::from_secs(2), 1024),
            cancellation,
        )
        .expect_err("cancelled");
        assert!(error.contains("cancelled"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(windows)]
    #[test]
    fn windows_successful_command_is_resumed_after_job_assignment() {
        let result = run_bounded(
            "cmd.exe",
            &["/D".into(), "/C".into(), "echo out & echo err 1>&2".into()],
            Limits::for_test(Duration::from_secs(2), 1024),
            Cancellation::new(),
        )
        .expect("the suspended command must resume");

        assert_eq!(result.code, 0);
        assert_eq!(result.stdout.trim(), "out");
        assert_eq!(result.stderr.trim(), "err");
    }

    #[cfg(windows)]
    #[test]
    fn windows_timeout_terminates_the_job() {
        let started = Instant::now();
        let error = run_bounded(
            "cmd.exe",
            &[
                "/D".into(),
                "/C".into(),
                "ping.exe -n 20 127.0.0.1 >NUL".into(),
            ],
            Limits::for_test(Duration::from_millis(100), 1024),
            Cancellation::new(),
        )
        .expect_err("timeout");

        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(windows)]
    #[test]
    fn windows_cancellation_terminates_the_job() {
        let cancellation = Cancellation::new();
        cancellation.cancel();
        let started = Instant::now();
        let error = run_bounded(
            "cmd.exe",
            &[
                "/D".into(),
                "/C".into(),
                "ping.exe -n 20 127.0.0.1 >NUL".into(),
            ],
            Limits::for_test(Duration::from_secs(2), 1024),
            cancellation,
        )
        .expect_err("cancelled");

        assert!(error.contains("cancelled"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(windows)]
    #[test]
    fn windows_timeout_kills_descendants() {
        let directory = std::env::temp_dir().join(format!(
            "gpui-shell-job-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let child_script = directory.join("child.cmd");
        let parent_script = directory.join("parent.cmd");
        let marker = directory.join("descendant-survived");
        std::fs::write(
            &child_script,
            format!(
                "@ping.exe -n 3 127.0.0.1 >NUL\r\n@echo survived>\"{}\"\r\n",
                marker.display()
            ),
        )
        .unwrap();
        std::fs::write(
            &parent_script,
            format!(
                "@start \"\" /B cmd.exe /D /C call \"{}\"\r\n@ping.exe -n 20 127.0.0.1 >NUL\r\n",
                child_script.display()
            ),
        )
        .unwrap();

        let error = run_bounded(
            "cmd.exe",
            &[
                "/D".into(),
                "/C".into(),
                parent_script.into_os_string().into_string().unwrap(),
            ],
            Limits::for_test(Duration::from_millis(100), 1024),
            Cancellation::new(),
        )
        .expect_err("timeout");
        assert!(error.contains("timed out"), "{error}");
        std::thread::sleep(Duration::from_secs(3));
        assert!(
            !marker.exists(),
            "a descendant escaped the Windows Job Object"
        );

        let _ = std::fs::remove_dir_all(directory);
    }
}
