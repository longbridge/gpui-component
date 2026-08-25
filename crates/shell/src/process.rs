//! Bounded child-process execution for the standard runtime adapter.

use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
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
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("running `{command}` failed: {error}"))?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let exceeded = Arc::new(AtomicU8::new(0));
    let stdout_reader = read_stream(stdout, limits.stdout, STDOUT_EXCEEDED, exceeded.clone());
    let stderr_reader = read_stream(stderr, limits.stderr, STDERR_EXCEEDED, exceeded.clone());
    let started = Instant::now();

    let status = loop {
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
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(format!("`{command}` {reason}"));
        }

        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("waiting for `{command}` failed: {error}"));
            }
        }
    };

    let stdout = join_stream(stdout_reader, "stdout")?;
    let stderr = join_stream(stderr_reader, "stderr")?;
    Ok(Output {
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

fn read_stream(
    mut stream: impl Read + Send + 'static,
    limit: usize,
    flag: u8,
    exceeded: Arc<AtomicU8>,
) -> thread::JoinHandle<Result<Vec<u8>, String>> {
    thread::spawn(move || {
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
    })
}

fn join_stream(
    reader: thread::JoinHandle<Result<Vec<u8>, String>>,
    name: &str,
) -> Result<Vec<u8>, String> {
    reader
        .join()
        .map_err(|_| format!("reading child {name} panicked"))?
        .map_err(|error| format!("child {name} {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
}
