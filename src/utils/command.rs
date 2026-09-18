//! Command execution utilities
//!
//! This module provides a unified interface for executing external commands
//! asynchronously. It handles command execution, output capture, error handling,
//! and debug logging in a consistent manner across the application.

use crate::constants::env_vars::RUST_LOG;
use crate::constants::research::process::{DEFAULT_SAFE_PATH, DEFAULT_TERM};
use crate::constants::timeouts::{INITIAL_RETRY_DELAY, MAX_RETRY_DELAY};
use crate::utils::command_process::{
    CommandOutput, CommandSpec, OsProcessBackend, ProcessHandle, StdioPolicy, read_bounded,
};
use anyhow::{Context, Result, anyhow};
use std::ffi::OsStr;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::Command;

/// A utility for executing external commands asynchronously.
///
/// CommandRunner provides a consistent interface for running external tools
/// like Android SDK utilities, iOS simulator commands, and other system tools.
/// It handles output capture, error propagation, and optional debug logging.
///
/// # Examples
/// ```rust,no_run
/// use emu::utils::CommandRunner;
///
/// # async fn example() -> anyhow::Result<()> {
/// let runner = CommandRunner::new();
/// let output = runner.run("adb", &["devices"]).await?;
/// println!("Connected devices: {}", output);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CommandRunner;

impl Default for CommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRunner {
    /// Creates a new CommandRunner instance.
    ///
    /// # Returns
    /// A new CommandRunner ready to execute commands
    pub fn new() -> Self {
        Self
    }

    /// Executes a command and waits for it to complete, returning stdout.
    ///
    /// This method runs the specified command with arguments and captures
    /// both stdout and stderr. If the command fails (non-zero exit code),
    /// an error is returned with details from stderr and stdout.
    ///
    /// Debug logging is enabled when `RUST_LOG=debug` environment variable is set,
    /// which will print the command being executed and its output to stderr.
    ///
    /// # Arguments
    /// * `program` - The command/program to execute
    /// * `args` - Iterator of arguments to pass to the command
    ///
    /// # Returns
    /// * `Ok(String)` - Command stdout if execution succeeds
    /// * `Err(anyhow::Error)` - If command fails or cannot be executed
    ///
    /// # Examples
    /// ```rust,no_run
    /// use emu::utils::CommandRunner;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let runner = CommandRunner::new();
    /// let devices = runner.run("adb", &["devices"]).await?;
    /// let avds = runner.run("avdmanager", &["list", "avd"]).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Handling
    /// Errors can occur for several reasons:
    /// - Command not found in PATH
    /// - Command execution failure (non-zero exit code)
    /// - Permission denied
    /// - Invalid arguments
    pub async fn run<S, I, A>(&self, program: S, args: I) -> Result<String>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let program_ref = program.as_ref();
        let args_vec: Vec<_> = args
            .into_iter()
            .map(|a| a.as_ref().to_os_string())
            .collect();

        // Debug logging only when RUST_LOG=debug is set
        if std::env::var(RUST_LOG)
            .unwrap_or_default()
            .contains("debug")
        {
            let command_str = format!(
                "{} {}",
                program_ref.to_string_lossy(),
                args_vec
                    .iter()
                    .map(|a| a.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            eprintln!("[DEBUG] Executing command: {command_str}");
        }

        let output = Command::new(program_ref)
            .args(&args_vec)
            .output()
            .await
            .context("Failed to execute command")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Debug logging only when RUST_LOG=debug is set
        if std::env::var(RUST_LOG)
            .unwrap_or_default()
            .contains("debug")
        {
            eprintln!(
                "[DEBUG] Command exit code: {exit_code:?}",
                exit_code = output.status.code()
            );
            eprintln!("[DEBUG] Command stdout: {stdout}");
            eprintln!("[DEBUG] Command stderr: {stderr}");
        }

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Command failed with exit code {}: stderr: {} stdout: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim(),
                stdout.trim()
            ));
        }

        Ok(stdout.into_owned())
    }

    /// Spawns a command in the background and returns immediately.
    ///
    /// This method starts a command as a detached background process
    /// without waiting for it to complete. All stdio streams are
    /// redirected to null to prevent output interference.
    ///
    /// This is useful for launching GUI applications or long-running
    /// processes that should continue independently of the main application.
    ///
    /// # Arguments
    /// * `program` - The command/program to spawn
    /// * `args` - Iterator of arguments to pass to the command
    ///
    /// # Returns
    /// * `Ok(u32)` - Process ID of the spawned command
    /// * `Err(anyhow::Error)` - If the command cannot be spawned
    ///
    /// # Examples
    /// ```rust,no_run
    /// use emu::utils::CommandRunner;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let runner = CommandRunner::new();
    /// let pid = runner.spawn("open", &["-a", "Simulator"]).await?;
    /// println!("Launched Simulator with PID: {}", pid);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Note
    /// The spawned process runs independently and its exit status
    /// is not monitored. Use `run()` if you need to capture output
    /// or wait for completion.
    pub async fn spawn<S, I, A>(&self, program: S, args: I) -> Result<u32>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let child = Command::new(program)
            .args(args)
            .stdout(std::process::Stdio::null()) // Suppress stdout output
            .stderr(std::process::Stdio::null()) // Suppress stderr output
            .stdin(std::process::Stdio::null()) // No stdin needed
            .spawn()
            .context("Failed to spawn command")?;

        Ok(child.id().unwrap_or(0))
    }

    /// Executes a command ignoring specific error patterns (useful for "already in state" errors).
    ///
    /// This method runs a command and only returns an error if it doesn't match
    /// any of the provided ignore patterns. This is particularly useful for
    /// platform commands that return errors when a device is already in the
    /// requested state.
    ///
    /// # Arguments
    /// * `program` - The command/program to execute
    /// * `args` - Iterator of arguments to pass to the command
    /// * `ignore_patterns` - Patterns to ignore in error messages
    ///
    /// # Returns
    /// * `Ok(String)` - Command stdout if execution succeeds or error is ignored
    /// * `Err(anyhow::Error)` - If command fails with an error not in ignore patterns
    ///
    /// # Examples
    /// ```rust,no_run
    /// use emu::utils::CommandRunner;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let runner = CommandRunner::new();
    /// let device_id = "12345-6789";
    /// // Ignore "already booted" errors when starting iOS simulator
    /// runner.run_ignoring_errors(
    ///     "xcrun",
    ///     &["simctl", "boot", device_id],
    ///     &["Unable to boot device in current state: Booted"]
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_ignoring_errors<S, I, A>(
        &self,
        program: S,
        args: I,
        ignore_patterns: &[&str],
    ) -> Result<String>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        match self.run(program, args).await {
            Ok(output) => Ok(output),
            Err(e) => {
                let error_msg = e.to_string();

                // Check if error matches any ignore pattern
                for pattern in ignore_patterns {
                    if error_msg.contains(pattern) {
                        // Log that we're ignoring this error
                        log::info!("Ignoring expected error: {error_msg}");
                        return Ok(String::new());
                    }
                }

                // Re-throw the error if it doesn't match any pattern
                Err(e)
            }
        }
    }

    /// Executes a command with retry logic for transient failures.
    ///
    /// This method attempts to run a command multiple times with exponential
    /// backoff between attempts. Useful for commands that may fail due to
    /// temporary resource contention or network issues.
    ///
    /// # Arguments
    /// * `program` - The command/program to execute
    /// * `args` - Iterator of arguments to pass to the command
    /// * `max_retries` - Maximum number of retry attempts (0 = no retries)
    ///
    /// # Returns
    /// * `Ok(String)` - Command stdout if any attempt succeeds
    /// * `Err(anyhow::Error)` - If all attempts fail
    ///
    /// # Examples
    /// ```rust,no_run
    /// use emu::utils::CommandRunner;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let runner = CommandRunner::new();
    /// // Try up to 3 times to list devices
    /// let devices = runner.run_with_retry("adb", &["devices"], 2).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_with_retry<S, I, A>(
        &self,
        program: S,
        args: I,
        max_retries: u32,
    ) -> Result<String>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = A> + Clone,
        A: AsRef<OsStr>,
    {
        let mut last_error = None;
        let mut delay = INITIAL_RETRY_DELAY;

        for attempt in 0..=max_retries {
            match self.run(program.as_ref(), args.clone()).await {
                Ok(output) => return Ok(output),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < max_retries {
                        log::warn!(
                            "Command failed (attempt {}/{}), retrying after {:?}",
                            attempt + 1,
                            max_retries + 1,
                            delay
                        );
                        tokio::time::sleep(delay).await;

                        // Exponential backoff with max delay
                        delay = std::cmp::min(delay * 2, MAX_RETRY_DELAY);
                    }
                }
            }
        }

        Err(last_error.unwrap()).context(format!(
            "Command failed after {attempts} attempts",
            attempts = max_retries + 1
        ))
    }

    /// Executes a typed command specification and captures bounded output.
    ///
    /// The environment is cleared and populated only with a safe default PATH, TERM,
    /// and any explicitly provided variables in `spec.env`.
    /// Stdout and stderr are captured concurrently up to `spec.max_output_bytes`.
    /// Execution can be cancelled cooperatively via `spec.cancellation` or bounded by `spec.timeout`.
    /// Unlike legacy `run()`, non-zero exit codes are not treated as errors but returned
    /// in `CommandOutput.status` for the caller to evaluate.
    pub async fn run_typed(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }

        cmd.env_clear();
        cmd.env("PATH", DEFAULT_SAFE_PATH);
        cmd.env("TERM", DEFAULT_TERM);
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            if spec.process_group
                == crate::utils::command_process::ProcessGroupPolicy::NewProcessGroup
            {
                cmd.as_std_mut().process_group(0);
            }
        }

        match spec.stdin {
            StdioPolicy::Null => cmd.stdin(std::process::Stdio::null()),
            StdioPolicy::Inherit => cmd.stdin(std::process::Stdio::inherit()),
            StdioPolicy::Piped | StdioPolicy::Capture => cmd.stdin(std::process::Stdio::piped()),
        };

        match spec.stdout {
            StdioPolicy::Null => cmd.stdout(std::process::Stdio::null()),
            StdioPolicy::Inherit => cmd.stdout(std::process::Stdio::inherit()),
            StdioPolicy::Piped | StdioPolicy::Capture => cmd.stdout(std::process::Stdio::piped()),
        };

        match spec.stderr {
            StdioPolicy::Null => cmd.stderr(std::process::Stdio::null()),
            StdioPolicy::Inherit => cmd.stderr(std::process::Stdio::inherit()),
            StdioPolicy::Piped | StdioPolicy::Capture => cmd.stderr(std::process::Stdio::piped()),
        };
        let mut child = cmd.spawn().context(format!(
            "Failed to spawn command: {}",
            spec.program.display()
        ))?;

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let max_bytes = spec.max_output_bytes;
        let mut stdout_task = tokio::spawn(read_bounded(stdout_handle, max_bytes));
        let mut stderr_task = tokio::spawn(read_bounded(stderr_handle, max_bytes));
        let stdout_abort = stdout_task.abort_handle();
        let stderr_abort = stderr_task.abort_handle();

        let cancel_fut = async {
            if let Some(mut rx) = spec.cancellation.clone() {
                if *rx.borrow() {
                    return;
                }
                while rx.changed().await.is_ok() {
                    if *rx.borrow() {
                        return;
                    }
                }
            }
            std::future::pending::<()>().await;
        };

        let timeout_fut = async {
            if let Some(timeout_dur) = spec.timeout {
                tokio::time::sleep(timeout_dur).await;
            } else {
                std::future::pending::<()>().await;
            }
        };

        let mut stdout_res: Option<Result<Vec<u8>, anyhow::Error>> = None;
        let mut stderr_res: Option<Result<Vec<u8>, anyhow::Error>> = None;
        let mut child_status: Option<std::process::ExitStatus> = None;

        let exec_fut = async {
            loop {
                if child_status.is_some() && stdout_res.is_some() && stderr_res.is_some() {
                    break;
                }

                tokio::select! {
                    res = child.wait(), if child_status.is_none() => {
                        let status = res.context("Failed to wait on child process")?;
                        child_status = Some(status);
                        // Child process exited. Bound remaining stream drain to avoid deadlock
                        // if grandchild background processes inherited stdout/stderr pipes.
                        if stdout_res.is_none() || stderr_res.is_none() {
                            let drain_res = tokio::time::timeout(
                                crate::constants::research::process::INHERITED_STREAM_DRAIN_TIMEOUT,
                                async {
                                    if stdout_res.is_none() {
                                        let out = stdout_task
                                            .await
                                            .map_err(|e| anyhow!("Stdout capture task panicked: {e}"))?
                                            .map_err(|e| anyhow!("Command stdout overflow/error: {e}"))?;
                                        stdout_res = Some(Ok(out));
                                    }
                                    if stderr_res.is_none() {
                                        let err = stderr_task
                                            .await
                                            .map_err(|e| anyhow!("Stderr capture task panicked: {e}"))?
                                            .map_err(|e| anyhow!("Command stderr overflow/error: {e}"))?;
                                        stderr_res = Some(Ok(err));
                                    }
                                    Ok::<_, anyhow::Error>(())
                                },
                            ).await;

                            if drain_res.is_err() {
                                // Inherited stream drain timed out after child exit
                                stdout_abort.abort();
                                stderr_abort.abort();
                                if stdout_res.is_none() {
                                    stdout_res = Some(Ok(Vec::new()));
                                }
                                if stderr_res.is_none() {
                                    stderr_res = Some(Ok(Vec::new()));
                                }
                            }
                            break;
                        }
                    }
                    res = &mut stdout_task, if stdout_res.is_none() => {
                        match res {
                            Ok(Ok(bytes)) => {
                                stdout_res = Some(Ok(bytes));
                            }
                            Ok(Err(err)) => {
                                let _ = child.start_kill();
                                let _ = child.wait().await;
                                stderr_task.abort();
                                return Err(anyhow!("Command stdout overflow/error: {err}"));
                            }
                            Err(join_err) => {
                                let _ = child.start_kill();
                                let _ = child.wait().await;
                                stderr_task.abort();
                                return Err(anyhow!("Stdout capture task panicked: {join_err}"));
                            }
                        }
                    }
                    res = &mut stderr_task, if stderr_res.is_none() => {
                        match res {
                            Ok(Ok(bytes)) => {
                                stderr_res = Some(Ok(bytes));
                            }
                            Ok(Err(err)) => {
                                let _ = child.start_kill();
                                let _ = child.wait().await;
                                stdout_task.abort();
                                return Err(anyhow!("Command stderr overflow/error: {err}"));
                            }
                            Err(join_err) => {
                                let _ = child.start_kill();
                                let _ = child.wait().await;
                                stdout_task.abort();
                                return Err(anyhow!("Stderr capture task panicked: {join_err}"));
                            }
                        }
                    }
                }
            }

            let status = child_status.expect("child status must be set");
            let stdout = stdout_res.unwrap_or_else(|| Ok(Vec::new()))?;
            let stderr = stderr_res.unwrap_or_else(|| Ok(Vec::new()))?;
            Ok(CommandOutput {
                status,
                stdout,
                stderr,
            })
        };

        tokio::select! {
            res = exec_fut => res,
            _ = cancel_fut => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                stdout_abort.abort();
                stderr_abort.abort();
                Err(anyhow!("Command execution cancelled"))
            }
            _ = timeout_fut => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                stdout_abort.abort();
                stderr_abort.abort();
                Err(anyhow!("Command execution timed out after {:?}", spec.timeout.unwrap()))
            }
        }
    }

    /// Spawns a typed command specification and returns an owned `ProcessHandle`.
    ///
    /// Streams are configured per `spec.stdin`, `spec.stdout`, and `spec.stderr`.
    /// The returned `ProcessHandle` owns the child process and provides asynchronous
    /// wait, timeout wait without termination, and graceful shutdown/reap methods.
    pub async fn spawn_typed(&self, spec: &CommandSpec) -> Result<ProcessHandle> {
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }

        cmd.env_clear();
        cmd.env("PATH", DEFAULT_SAFE_PATH);
        cmd.env("TERM", DEFAULT_TERM);
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            if spec.process_group
                == crate::utils::command_process::ProcessGroupPolicy::NewProcessGroup
            {
                cmd.as_std_mut().process_group(0);
            }
        }

        match spec.stdin {
            StdioPolicy::Null => cmd.stdin(std::process::Stdio::null()),
            StdioPolicy::Inherit => cmd.stdin(std::process::Stdio::inherit()),
            StdioPolicy::Piped | StdioPolicy::Capture => cmd.stdin(std::process::Stdio::piped()),
        };

        match spec.stdout {
            StdioPolicy::Null => cmd.stdout(std::process::Stdio::null()),
            StdioPolicy::Inherit => cmd.stdout(std::process::Stdio::inherit()),
            StdioPolicy::Piped | StdioPolicy::Capture => cmd.stdout(std::process::Stdio::piped()),
        };

        match spec.stderr {
            StdioPolicy::Null => cmd.stderr(std::process::Stdio::null()),
            StdioPolicy::Inherit => cmd.stderr(std::process::Stdio::inherit()),
            StdioPolicy::Piped | StdioPolicy::Capture => cmd.stderr(std::process::Stdio::piped()),
        };

        let mut child = cmd.spawn().context(format!(
            "Failed to spawn command: {}",
            spec.program.display()
        ))?;

        let pid = child.id().unwrap_or(0);
        let stdin: Option<Box<dyn AsyncWrite + Send + Unpin>> = child
            .stdin
            .take()
            .map(|s| Box::new(s) as Box<dyn AsyncWrite + Send + Unpin>);
        let stdout: Option<Box<dyn AsyncRead + Send + Unpin>> = child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn AsyncRead + Send + Unpin>);
        let stderr: Option<Box<dyn AsyncRead + Send + Unpin>> = child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn AsyncRead + Send + Unpin>);

        let backend = Box::new(OsProcessBackend::new(child, pid));
        Ok(ProcessHandle::new(pid, stdin, stdout, stderr, backend))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_runner_new() {
        let runner = CommandRunner::new();
        // Test that we can create a CommandRunner instance
        // The actual command execution tests are integration tests
        assert!(std::ptr::eq(&runner, &runner));
    }

    #[tokio::test]
    async fn test_run_simple_command() {
        let runner = CommandRunner::new();
        // Test with echo which should be available on all platforms
        let result = runner.run("echo", &["test"]).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("test"));
    }

    #[tokio::test]
    async fn test_run_command_failure() {
        let runner = CommandRunner::new();
        // Test with a non-existent command
        let empty_args: Vec<&str> = vec![];
        let result = runner
            .run("this_command_does_not_exist_12345", &empty_args)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_spawn_command() {
        let runner = CommandRunner::new();
        // Test spawning a simple command
        let result = runner.spawn("echo", &["test"]).await;
        assert!(result.is_ok());
        let pid = result.unwrap();
        assert!(pid > 0);
    }

    #[tokio::test]
    async fn test_run_ignoring_errors() {
        let runner = CommandRunner::new();
        // Test ignoring specific error patterns
        let result = runner
            .run_ignoring_errors("echo", &["error: already exists"], &["already exists"])
            .await;
        assert!(result.is_ok());

        // Test with error that shouldn't be ignored
        let empty_args: Vec<&str> = vec![];
        let result = runner
            .run_ignoring_errors(
                "this_command_does_not_exist_12345",
                &empty_args,
                &["different error"],
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_with_retry() {
        let runner = CommandRunner::new();
        // Test with a command that succeeds
        let result = runner.run_with_retry("echo", &["test"], 2).await;
        assert!(result.is_ok());

        // Test with a command that fails
        let empty_args: Vec<&str> = vec![];
        let start = std::time::Instant::now();
        let result = runner
            .run_with_retry("this_command_does_not_exist_12345", &empty_args, 1)
            .await;
        let duration = start.elapsed();
        assert!(result.is_err());
        // Should have retried once with delay
        assert!(duration.as_millis() >= 100);
    }

    #[test]
    fn test_command_runner_send_sync() {
        // Ensure CommandRunner is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CommandRunner>();
    }
}
