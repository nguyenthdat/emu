//! Typed command execution, process handle abstraction, and process backend.
//!
//! Provides `CommandSpec`, `CommandOutput`, `StdioPolicy`, `ProcessHandle`,
//! and backend implementations (`OsProcessBackend`, `MockProcessBackend`).

use crate::constants::research::process::{DEFAULT_MAX_OUTPUT_BYTES, PROCESS_REAP_POLL_INTERVAL};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

/// Stdio policy for subprocess streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StdioPolicy {
    /// Discard stream content (/dev/null).
    #[default]
    Null,
    /// Inherit stream from the parent process.
    Inherit,
    /// Connect stream to an asynchronous pipe exposed on the `ProcessHandle`.
    Piped,
    /// Capture stream output in memory up to a declared byte boundary.
    Capture,
}

/// Process group / session isolation policy for spawned subprocesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessGroupPolicy {
    /// Child inherits parent's process group (default).
    #[default]
    Inherit,
    /// Child creates and leads a new process group (`setpgid(0, 0)`).
    /// Isolates child from signals sent to the parent's process group (e.g. terminal Ctrl+C).
    NewProcessGroup,
}

/// Typed command specification.
///
/// Encapsulates all execution parameters: binary path, ordered arguments,
/// working directory, explicit sanitized environment, stdio policies,
/// optional timeout, and cooperative watch cancellation.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// Target executable binary.
    pub program: PathBuf,
    /// Ordered command-line arguments.
    pub args: Vec<String>,
    /// Explicit working directory (None defaults to caller's current working directory).
    pub cwd: Option<PathBuf>,
    /// Explicit environment variables. Host credentials and sensitive variables
    /// are stripped by clearing environment before setting these.
    pub env: BTreeMap<String, String>,
    /// Stdio policy for stdin (defaults to `StdioPolicy::Null`).
    pub stdin: StdioPolicy,
    /// Stdio policy for stdout (defaults to `StdioPolicy::Capture`).
    pub stdout: StdioPolicy,
    /// Stdio policy for stderr (defaults to `StdioPolicy::Capture`).
    pub stderr: StdioPolicy,
    /// Execution deadline for `run_typed`.
    pub timeout: Option<Duration>,
    /// Cooperative cancellation receiver using `tokio::sync::watch`.
    pub cancellation: Option<tokio::sync::watch::Receiver<bool>>,
    /// Maximum buffer size for captured stdout and stderr in bytes.
    pub max_output_bytes: usize,
    /// Process group and session isolation policy.
    pub process_group: ProcessGroupPolicy,
}

impl CommandSpec {
    /// Create a new command specification for the given program with safe defaults.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: BTreeMap::new(),
            stdin: StdioPolicy::Null,
            stdout: StdioPolicy::Capture,
            stderr: StdioPolicy::Capture,
            timeout: None,
            cancellation: None,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            process_group: ProcessGroupPolicy::Inherit,
        }
    }

    /// Append a single argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Append multiple arguments from an iterator.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self.args.push(arg.into());
        }
        self
    }

    /// Set explicit working directory.
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set a single environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables.
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in vars {
            self.env.insert(k.into(), v.into());
        }
        self
    }

    /// Set stdin policy.
    pub fn stdin(mut self, policy: StdioPolicy) -> Self {
        self.stdin = policy;
        self
    }

    /// Set stdout policy.
    pub fn stdout(mut self, policy: StdioPolicy) -> Self {
        self.stdout = policy;
        self
    }

    /// Set stderr policy.
    pub fn stderr(mut self, policy: StdioPolicy) -> Self {
        self.stderr = policy;
        self
    }

    /// Set caller execution timeout deadline.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set cooperative cancellation watch receiver.
    pub fn cancellation(mut self, rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.cancellation = Some(rx);
        self
    }

    /// Override maximum captured output byte ceiling per stream.
    pub fn max_output_bytes(mut self, max_bytes: usize) -> Self {
        self.max_output_bytes = max_bytes;
        self
    }

    /// Set process group / session isolation policy.
    pub fn process_group(mut self, policy: ProcessGroupPolicy) -> Self {
        self.process_group = policy;
        self
    }

    /// Helper to borrow args as string slices.
    pub fn args_as_str_slice(&self) -> Vec<&str> {
        self.args.iter().map(|s| s.as_str()).collect()
    }
}

/// Captured output of a completed typed command execution.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Exit status capturing return code or terminating signal.
    pub status: std::process::ExitStatus,
    /// Bounded raw bytes captured from stdout.
    pub stdout: Vec<u8>,
    /// Bounded raw bytes captured from stderr.
    pub stderr: Vec<u8>,
}

impl CommandOutput {
    /// True if the process completed with an exit code of 0.
    pub fn success(&self) -> bool {
        self.status.success()
    }

    /// Numeric exit code if the process exited normally.
    pub fn code(&self) -> Option<i32> {
        self.status.code()
    }

    /// Terminating Unix signal if terminated by signal.
    pub fn signal(&self) -> Option<i32> {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            self.status.signal()
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    /// Return stdout as a valid UTF-8 string slice, or return an error if invalid UTF-8.
    pub fn stdout_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.stdout).context("stdout contains invalid UTF-8 bytes")
    }

    /// Return stderr as a valid UTF-8 string slice, or return an error if invalid UTF-8.
    pub fn stderr_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.stderr).context("stderr contains invalid UTF-8 bytes")
    }

    /// Return stdout with invalid UTF-8 bytes replaced by replacement characters.
    pub fn stdout_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    /// Return stderr with invalid UTF-8 bytes replaced by replacement characters.
    pub fn stderr_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stderr)
    }
}

/// Abstract process backend trait for mockable process execution.
#[async_trait]
pub trait ProcessBackend: Send + Sync {
    /// Await process termination and reap exit status.
    async fn wait(&mut self) -> Result<std::process::ExitStatus>;

    /// Await process termination up to the declared timeout without killing or terminating the process.
    /// Returns `Ok(Some(status))` if the process exited, or `Ok(None)` if deadline elapsed while still running.
    async fn wait_timeout(&mut self, timeout: Duration)
    -> Result<Option<std::process::ExitStatus>>;

    /// Send graceful termination signal (SIGTERM on Unix).
    async fn terminate(&mut self) -> Result<()>;

    /// Send immediate forceful kill signal (SIGKILL on Unix).
    async fn kill(&mut self) -> Result<()>;

    /// Perform graceful SIGTERM, wait up to `grace_period`, escalate to SIGKILL if still active, and reap exit status.
    async fn shutdown_and_reap(
        &mut self,
        grace_period: Duration,
    ) -> Result<std::process::ExitStatus>;

    /// Non-blocking check for process completion.
    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>>;

    /// Check if the process has already been reaped.
    fn is_reaped(&self) -> bool;

    /// Relinquish ownership of the child process, delegating backend release.
    fn detach(&mut self) -> Result<u32>;
    /// Optional cleanup hook invoked when owning ProcessHandle is dropped.
    fn on_drop(&mut self, _pid: u32) {}
}

/// Owned handle to a running process, providing asynchronous stream I/O and owner-managed lifecycle control.
pub struct ProcessHandle {
    /// Process ID of the spawned child.
    pub pid: u32,
    /// Standard input stream if piped.
    pub stdin: Option<Box<dyn AsyncWrite + Send + Unpin>>,
    /// Standard output stream if piped or captured.
    pub stdout: Option<Box<dyn AsyncRead + Send + Unpin>>,
    /// Standard error stream if piped or captured.
    pub stderr: Option<Box<dyn AsyncRead + Send + Unpin>>,
    backend: Box<dyn ProcessBackend>,
    reaped: bool,
    cached_status: Option<std::process::ExitStatus>,
}

impl ProcessHandle {
    /// Create a new `ProcessHandle` facade around a backend.
    pub fn new(
        pid: u32,
        stdin: Option<Box<dyn AsyncWrite + Send + Unpin>>,
        stdout: Option<Box<dyn AsyncRead + Send + Unpin>>,
        stderr: Option<Box<dyn AsyncRead + Send + Unpin>>,
        backend: Box<dyn ProcessBackend>,
    ) -> Self {
        Self {
            pid,
            stdin,
            stdout,
            stderr,
            backend,
            reaped: false,
            cached_status: None,
        }
    }

    /// Return the process ID.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Take the stdin stream handle if available.
    pub fn take_stdin(&mut self) -> Option<Box<dyn AsyncWrite + Send + Unpin>> {
        self.stdin.take()
    }

    /// Take the stdout stream handle if available.
    pub fn take_stdout(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>> {
        self.stdout.take()
    }

    /// Take the stderr stream handle if available.
    pub fn take_stderr(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>> {
        self.stderr.take()
    }

    /// Check whether the process has already been reaped.
    pub fn is_reaped(&self) -> bool {
        self.cached_status.is_some() || self.reaped || self.backend.is_reaped()
    }

    /// Non-blocking query for child process exit status.
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>> {
        if let Some(status) = self.cached_status {
            return Ok(Some(status));
        }
        let res = self.backend.try_wait()?;
        if let Some(status) = res {
            self.reaped = true;
            self.cached_status = Some(status);
        }
        Ok(res)
    }

    /// Await child process termination and reap process status.
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        if let Some(status) = self.cached_status {
            return Ok(status);
        }
        let status = self.backend.wait().await?;
        self.reaped = true;
        self.cached_status = Some(status);
        Ok(status)
    }

    /// Await child process termination up to the declared timeout.
    ///
    /// If the timeout elapses while the child is still executing, returns `Ok(None)`.
    /// The process continues running unaffected (observer timeout non-interference).
    pub async fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<std::process::ExitStatus>> {
        if let Some(status) = self.cached_status {
            return Ok(Some(status));
        }
        let res = self.backend.wait_timeout(timeout).await?;
        if let Some(status) = res {
            self.reaped = true;
            self.cached_status = Some(status);
        }
        Ok(res)
    }

    /// Send graceful termination signal (SIGTERM on Unix).
    pub async fn terminate(&mut self) -> Result<()> {
        self.backend.terminate().await
    }

    /// Send forceful kill signal (SIGKILL on Unix).
    pub async fn kill(&mut self) -> Result<()> {
        self.backend.kill().await
    }

    /// Graceful SIGTERM, wait up to `grace_period`, escalate to SIGKILL, and reap exit status.
    pub async fn shutdown_and_reap(
        &mut self,
        grace_period: Duration,
    ) -> Result<std::process::ExitStatus> {
        if let Some(status) = self.cached_status {
            return Ok(status);
        }
        let status = self.backend.shutdown_and_reap(grace_period).await?;
        self.reaped = true;
        self.cached_status = Some(status);
        Ok(status)
    }

    /// Relinquish ownership of the child process, disarming RAII `Drop` termination.
    ///
    /// Pipes are closed so file descriptors are not leaked to the detached process.
    /// The underlying child process continues executing independently.
    /// Returns the process ID of the detached process.
    pub fn detach(mut self) -> Result<u32> {
        self.stdin = None;
        self.stdout = None;
        self.stderr = None;
        self.reaped = true;
        self.backend.detach()
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if !self.reaped {
            self.backend.on_drop(self.pid);
        }
    }
}

/// OS-backed process implementation wrapping `tokio::process::Child`.
pub struct OsProcessBackend {
    child: Option<tokio::process::Child>,
    pid: u32,
    reaped_status: Option<std::process::ExitStatus>,
    detached: bool,
}

impl OsProcessBackend {
    /// Create a new OS process backend.
    pub fn new(child: tokio::process::Child, pid: u32) -> Self {
        Self {
            child: Some(child),
            pid,
            reaped_status: None,
            detached: false,
        }
    }
}

#[async_trait]
impl ProcessBackend for OsProcessBackend {
    async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        if let Some(status) = self.reaped_status {
            return Ok(status);
        }
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| anyhow!("Child process already reaped or missing"))?;
        let status = child
            .wait()
            .await
            .context("Failed to wait on child process")?;
        self.reaped_status = Some(status);
        Ok(status)
    }

    async fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<std::process::ExitStatus>> {
        if let Some(status) = self.reaped_status {
            return Ok(Some(status));
        }
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| anyhow!("Child process already reaped or missing"))?;

        tokio::select! {
            res = child.wait() => {
                let status = res.context("Failed to wait on child process")?;
                self.reaped_status = Some(status);
                Ok(Some(status))
            }
            _ = tokio::time::sleep(timeout) => {
                // Deadline elapsed; process continues running. Do NOT kill or reap.
                Ok(None)
            }
        }
    }

    async fn terminate(&mut self) -> Result<()> {
        if self.try_wait()?.is_some() {
            return Ok(());
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        let Some(raw_pid) = child.id() else {
            return Ok(());
        };
        let pid = i32::try_from(raw_pid)
            .map_err(|_| anyhow!("Child PID {raw_pid} exceeds valid i32 range"))?;
        if pid <= 0 {
            return Err(anyhow!("Invalid non-positive PID {pid}"));
        }
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;
            let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
            Ok(())
        }
        #[cfg(not(unix))]
        {
            child.start_kill().context("Failed to terminate process")?;
            Ok(())
        }
    }

    async fn kill(&mut self) -> Result<()> {
        if self.try_wait()?.is_some() {
            return Ok(());
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        let Some(raw_pid) = child.id() else {
            return Ok(());
        };
        let pid = i32::try_from(raw_pid)
            .map_err(|_| anyhow!("Child PID {raw_pid} exceeds valid i32 range"))?;
        if pid <= 0 {
            return Err(anyhow!("Invalid non-positive PID {pid}"));
        }
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;
            let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = child.start_kill();
            Ok(())
        }
    }
    async fn shutdown_and_reap(
        &mut self,
        grace_period: Duration,
    ) -> Result<std::process::ExitStatus> {
        if let Some(status) = self.reaped_status {
            return Ok(status);
        }

        // 1. Issue graceful SIGTERM
        let _ = self.terminate().await;

        // 2. Poll for termination up to grace_period
        let poll_start = tokio::time::Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            if poll_start.elapsed() >= grace_period {
                break;
            }
            tokio::time::sleep(PROCESS_REAP_POLL_INTERVAL).await;
        }

        // 3. Grace period elapsed: escalate to SIGKILL
        log::warn!(
            "Process PID {} did not exit within grace period {:?}; escalating to SIGKILL",
            self.pid,
            grace_period
        );
        let _ = self.kill().await;

        // 4. Await confirmed reaping after SIGKILL
        let status = match self.child.as_mut() {
            Some(child) => child
                .wait()
                .await
                .context("Failed to reap child process after SIGKILL")?,
            None => return Err(anyhow!("Child process already reaped or missing")),
        };
        self.reaped_status = Some(status);
        Ok(status)
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>> {
        if let Some(status) = self.reaped_status {
            return Ok(Some(status));
        }
        let status_opt = match self.child.as_mut() {
            Some(child) => child.try_wait()?,
            None => None,
        };
        if let Some(status) = status_opt {
            self.reaped_status = Some(status);
            return Ok(Some(status));
        }
        Ok(None)
    }

    fn is_reaped(&self) -> bool {
        self.reaped_status.is_some()
    }

    fn on_drop(&mut self, _pid: u32) {
        if self.detached || self.is_reaped() {
            return;
        }
        if let Ok(Some(_)) = self.try_wait() {
            return;
        }
        let Some(raw_pid) = self.child.as_ref().and_then(|c| c.id()) else {
            return;
        };
        let Ok(pid) = i32::try_from(raw_pid) else {
            return;
        };
        if pid <= 0 {
            return;
        }
        log::warn!(
            "ProcessHandle for PID {pid} dropped before explicit asynchronous reaping; sending best-effort SIGTERM"
        );
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;
            let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
        }
        #[cfg(not(unix))]
        {
            if let Some(child) = self.child.as_mut() {
                let _ = child.start_kill();
            }
        }
    }

    fn detach(&mut self) -> Result<u32> {
        if self.detached {
            return Err(anyhow!("Process backend already detached"));
        }
        let mut child = self
            .child
            .take()
            .ok_or_else(|| anyhow!("Child process already reaped or missing"))?;
        let raw_pid = child
            .id()
            .ok_or_else(|| anyhow!("Child process missing PID"))?;
        self.detached = true;
        // Spawn background task to reap child if it terminates while the parent runtime is alive.
        // If the parent process exits, the child continues running as an OS orphan reparented to init/launchd.
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(raw_pid)
    }
}

/// In-memory mock process backend for deterministic unit tests.
pub struct MockProcessBackend {
    /// Configured exit status to return when reaped.
    pub exit_status: std::process::ExitStatus,
    /// Simulated PID.
    pub pid: u32,
    /// Configured simulated running duration before normal completion.
    pub delay: Option<Duration>,
    /// Whether the process has completed and been reaped.
    pub reaped: bool,
    /// Whether SIGTERM was received.
    pub terminated: bool,
    /// Whether SIGKILL was received.
    pub killed: bool,
    /// Whether process was detached.
    pub detached: bool,
    spawn_time: tokio::time::Instant,
}

impl MockProcessBackend {
    /// Create a new mock process backend with an exit status.
    pub fn new(exit_status: std::process::ExitStatus) -> Self {
        Self {
            exit_status,
            pid: 0,
            delay: None,
            reaped: false,
            terminated: false,
            killed: false,
            detached: false,
            spawn_time: tokio::time::Instant::now(),
        }
    }

    /// Set simulated PID.
    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = pid;
        self
    }

    /// Add a simulated process execution delay.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
}

#[async_trait]
impl ProcessBackend for MockProcessBackend {
    async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        if self.reaped {
            return Ok(self.exit_status);
        }
        if let Some(delay) = self.delay {
            if !self.terminated && !self.killed {
                let elapsed = self.spawn_time.elapsed();
                if elapsed < delay {
                    tokio::time::sleep(delay - elapsed).await;
                }
            }
        }
        self.reaped = true;
        Ok(self.exit_status)
    }

    async fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<std::process::ExitStatus>> {
        if self.reaped {
            return Ok(Some(self.exit_status));
        }
        if self.terminated || self.killed {
            self.reaped = true;
            return Ok(Some(self.exit_status));
        }
        if let Some(delay) = self.delay {
            let elapsed = self.spawn_time.elapsed();
            if elapsed >= delay {
                self.reaped = true;
                return Ok(Some(self.exit_status));
            }
            let remaining = delay - elapsed;
            if remaining <= timeout {
                tokio::time::sleep(remaining).await;
                self.reaped = true;
                Ok(Some(self.exit_status))
            } else {
                tokio::time::sleep(timeout).await;
                Ok(None)
            }
        } else {
            self.reaped = true;
            Ok(Some(self.exit_status))
        }
    }

    async fn terminate(&mut self) -> Result<()> {
        self.terminated = true;
        Ok(())
    }

    async fn kill(&mut self) -> Result<()> {
        self.killed = true;
        Ok(())
    }

    async fn shutdown_and_reap(
        &mut self,
        _grace_period: Duration,
    ) -> Result<std::process::ExitStatus> {
        self.terminated = true;
        self.reaped = true;
        Ok(self.exit_status)
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>> {
        if self.reaped {
            return Ok(Some(self.exit_status));
        }
        if self.terminated || self.killed {
            self.reaped = true;
            return Ok(Some(self.exit_status));
        }
        if let Some(delay) = self.delay {
            if self.spawn_time.elapsed() >= delay {
                self.reaped = true;
                return Ok(Some(self.exit_status));
            }
            Ok(None)
        } else {
            self.reaped = true;
            Ok(Some(self.exit_status))
        }
    }

    fn is_reaped(&self) -> bool {
        self.reaped
    }
    fn on_drop(&mut self, _pid: u32) {
        // Synthetic handles never signal OS.
    }

    fn detach(&mut self) -> Result<u32> {
        self.detached = true;
        // Synthetic handles never signal OS and do not spawn OS child tasks.
        // Returns synthetic PID without interacting with OS.
        Ok(self.pid)
    }
}

/// Helper to synthesize a normal `std::process::ExitStatus` from an exit code.
pub fn exit_status_from_code(code: i32) -> std::process::ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatusExt::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatusExt::from_raw(code as u32)
    }
    #[cfg(not(any(unix, windows)))]
    {
        compile_error!("Unsupported platform for ExitStatus creation");
    }
}

/// Helper to synthesize a signal termination `std::process::ExitStatus` from a Unix signal.
#[cfg(unix)]
pub fn exit_status_from_signal(signal: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatusExt::from_raw(signal)
}

/// Helper function to asynchronously read bounded bytes from an AsyncRead stream.
///
/// Reads chunks into a buffer up to `max_bytes`. If the stream produces more bytes
/// than `max_bytes`, an error is returned to report overflow rather than silent truncation.
pub async fn read_bounded<R: AsyncRead + Unpin>(
    reader: Option<R>,
    max_bytes: usize,
) -> std::io::Result<Vec<u8>> {
    let Some(mut r) = reader else {
        return Ok(Vec::new());
    };
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = r.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        if buf.len() + n > max_bytes {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Output stream exceeded maximum byte limit of {max_bytes} bytes"),
            ));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(buf)
}
