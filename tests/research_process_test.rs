//! Tests for typed process execution, process handles, bounded capture,
//! observer timeout process survival, explicit shutdown/reap, detach semantics,
//! session isolation, and mock synthetic streams.

use emu::utils::command::CommandRunner;
#[cfg(unix)]
use emu::utils::command_executor::exit_status_from_signal;
use emu::utils::command_executor::mock::{MockCommandExecutor, SyntheticProcessSpec};
use emu::utils::command_executor::{
    CommandExecutor, CommandOutput, CommandSpec, ProcessGroupPolicy, StdioPolicy,
    exit_status_from_code,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_command_output_status_and_utf8() {
    let output = CommandOutput {
        status: exit_status_from_code(0),
        stdout: b"hello world\n".to_vec(),
        stderr: b"no error".to_vec(),
    };

    assert!(output.success());
    assert_eq!(output.code(), Some(0));
    assert_eq!(output.stdout_str().unwrap(), "hello world\n");
    assert_eq!(output.stderr_str().unwrap(), "no error");

    // Non-zero status
    let err_output = CommandOutput {
        status: exit_status_from_code(5),
        stdout: Vec::new(),
        stderr: b"concurrency conflict".to_vec(),
    };
    assert!(!err_output.success());
    assert_eq!(err_output.code(), Some(5));

    // Signal status (Unix)
    #[cfg(unix)]
    {
        let sig_output = CommandOutput {
            status: exit_status_from_signal(9),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        assert!(!sig_output.success());
        assert_eq!(sig_output.signal(), Some(9));
    }

    // Invalid UTF-8 handling
    let bad_utf8 = CommandOutput {
        status: exit_status_from_code(1),
        stdout: vec![0xFF, 0xFE, 0xFD],
        stderr: Vec::new(),
    };
    assert!(bad_utf8.stdout_str().is_err());
    assert!(bad_utf8.stdout_lossy().contains('\u{FFFD}'));
}

#[tokio::test]
async fn test_runner_typed_env_and_cwd() {
    let runner = CommandRunner::new();
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let temp_path = temp_dir.path().to_path_buf();

    let spec = CommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("pwd; echo TEST_VAR=$TEST_VAR")
        .cwd(&temp_path)
        .env("TEST_VAR", "process_setup_safe_val");

    let output = runner.run_typed(&spec).await.expect("run_typed failed");
    assert!(output.success(), "Command should succeed");

    let stdout = output.stdout_str().expect("Valid UTF-8 stdout");
    assert!(
        stdout.contains("TEST_VAR=process_setup_safe_val"),
        "Explicit environment variable must be set"
    );

    // Canonical path verification
    let canonical_temp = temp_path.canonicalize().unwrap_or(temp_path);
    assert!(
        stdout.contains(canonical_temp.to_str().unwrap()),
        "Working directory must match spec.cwd"
    );
}

#[tokio::test]
async fn test_runner_typed_bounded_capture_overflow_reported() {
    let runner = CommandRunner::new();

    // Generate ~5000 bytes of output against a 256-byte maximum buffer.
    // Bounded capture must report overflow rather than silent successful truncation.
    let spec = CommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("for i in $(seq 1 500); do echo \"line-$i-padding-data-for-bounds\"; done")
        .max_output_bytes(256);

    let res = runner.run_typed(&spec).await;
    assert!(
        res.is_err(),
        "Output exceeding max_output_bytes must return an error"
    );
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("exceeded maximum") || err_msg.contains("overflow"),
        "Error message must report buffer overflow: {err_msg}"
    );
}

#[tokio::test]
async fn test_runner_typed_nonzero_status_forwarded() {
    let runner = CommandRunner::new();

    let spec = CommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("echo custom_failure >&2; exit 42");

    let output = runner
        .run_typed(&spec)
        .await
        .expect("run_typed should return Ok(CommandOutput) for nonzero exit code");

    assert!(!output.success());
    assert_eq!(output.code(), Some(42));
    assert!(output.stderr_str().unwrap().contains("custom_failure"));
}

#[tokio::test]
async fn test_runner_typed_cancellation() {
    let runner = CommandRunner::new();
    let (tx, rx) = tokio::sync::watch::channel(false);

    let spec = CommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("sleep 10")
        .cancellation(rx);

    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = tx.send(true);
    });

    let res = runner.run_typed(&spec).await;
    handle.await.unwrap();

    assert!(res.is_err(), "Cancelled execution must return an error");
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("cancelled"));
}

#[tokio::test]
async fn test_false_watch_channel_close_does_not_cancel() {
    let runner = CommandRunner::new();
    let (tx, rx) = tokio::sync::watch::channel(false);

    // Drop the sender immediately without setting true.
    // Channel close MUST NOT be interpreted as a cooperative cancellation request.
    drop(tx);

    let spec = CommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("echo channel_closed_ok")
        .cancellation(rx);

    let output = runner
        .run_typed(&spec)
        .await
        .expect("Dropped sender must not trigger false cancellation");
    assert!(output.success());
    assert!(output.stdout_str().unwrap().contains("channel_closed_ok"));
}

#[tokio::test]
async fn test_process_handle_observer_wait_timeout_survival() {
    let runner = CommandRunner::new();

    // Process sleeps for 400ms then exits 0
    let spec = CommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("sleep 0.4; exit 0");

    let mut handle = runner.spawn_typed(&spec).await.expect("spawn_typed failed");
    assert!(handle.pid() > 0);

    // Observer wait timeout after 50ms - MUST NOT kill or terminate process
    let timed_out = handle
        .wait_timeout(Duration::from_millis(50))
        .await
        .expect("wait_timeout call failed");
    assert!(
        timed_out.is_none(),
        "Observer wait_timeout must return None on timeout without killing child"
    );

    // Verify process is still alive
    assert!(!handle.is_reaped(), "Process must remain unreaped");

    // Supervisor then awaits completion
    let final_status = handle.wait().await.expect("Final wait failed");
    assert!(final_status.success(), "Child must complete normally");
    assert!(handle.is_reaped(), "Process is now reaped");
}

#[tokio::test]
async fn test_repeated_wait_and_try_wait_final_state() {
    let runner = CommandRunner::new();

    let spec = CommandSpec::new("/bin/sh").arg("-c").arg("exit 0");

    let mut handle = runner.spawn_typed(&spec).await.expect("spawn_typed failed");

    // Wait once to completion
    let s1 = handle.wait().await.expect("first wait");
    assert!(s1.success());
    assert!(handle.is_reaped());

    // Subsequent wait must return the cached final state without error
    let s2 = handle.wait().await.expect("second wait");
    assert_eq!(s1.code(), s2.code());

    // Subsequent try_wait must also return the cached final state
    let s3 = handle
        .try_wait()
        .expect("try_wait after wait")
        .expect("status present");
    assert_eq!(s1.code(), s3.code());
    assert!(handle.is_reaped());
}

#[tokio::test]
async fn test_process_handle_shutdown_and_reap() {
    let runner = CommandRunner::new();

    // Process running in background
    let spec = CommandSpec::new("/bin/sh").arg("-c").arg("sleep 30");

    let mut handle = runner.spawn_typed(&spec).await.expect("spawn_typed failed");
    assert!(handle.pid() > 0);

    // Explicit graceful shutdown and reaping with 100ms grace period
    let status = handle
        .shutdown_and_reap(Duration::from_millis(100))
        .await
        .expect("shutdown_and_reap failed");

    assert!(
        handle.is_reaped(),
        "Handle must be marked reaped after shutdown_and_reap"
    );
    // On Unix, terminated by SIGTERM or SIGKILL
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(status.signal().is_some() || status.code().is_some());
    }
    #[cfg(not(unix))]
    {
        let _ = status;
    }
}

#[tokio::test]
async fn test_process_handle_detach_survival() {
    let runner = CommandRunner::new();

    // Spawn a child process that sleeps for 2 seconds
    let spec = CommandSpec::new("/bin/sh").arg("-c").arg("sleep 2");

    let handle = runner.spawn_typed(&spec).await.expect("spawn_typed failed");
    let pid = handle.pid();
    assert!(pid > 0);

    // Explicitly detach ownership
    let detached_pid = handle.detach().expect("detach must succeed");
    assert_eq!(detached_pid, pid);

    // On Unix, verify the process is still running after handle was consumed and dropped
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;

        let res = kill(Pid::from_raw(pid as i32), None);
        assert!(
            res.is_ok(),
            "Detached process PID {pid} must survive handle drop"
        );

        // Clean up detached process
        let _ = kill(Pid::from_raw(pid as i32), Some(Signal::SIGTERM));
    }
}

#[tokio::test]
#[cfg(unix)]
async fn test_process_group_isolation() {
    let runner = CommandRunner::new();

    // Spawn child with NewProcessGroup isolation
    let spec = CommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("sleep 1")
        .process_group(ProcessGroupPolicy::NewProcessGroup);

    let mut handle = runner.spawn_typed(&spec).await.expect("spawn_typed failed");
    let pid = handle.pid();
    assert!(pid > 0);

    use nix::unistd::{Pid, getpgid};
    let pgid = getpgid(Some(Pid::from_raw(pid as i32))).expect("getpgid failed");
    assert_eq!(
        pgid,
        Pid::from_raw(pid as i32),
        "Child process group ID must equal its PID when NewProcessGroup is configured"
    );

    let _ = handle.shutdown_and_reap(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_stream_drain_after_inherited_pipes() {
    let runner = CommandRunner::new();

    // Parent exits immediately, but launches background sleep that inherits stdout.
    // run_typed must complete bounded by INHERITED_STREAM_DRAIN_TIMEOUT without hanging.
    let start = tokio::time::Instant::now();
    let spec = CommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("(sleep 3 >/dev/null 2>&1 &) ; echo direct_done");

    let output = runner.run_typed(&spec).await.expect("run_typed failed");
    assert!(output.success());
    assert!(output.stdout_str().unwrap().contains("direct_done"));
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "Command must not hang waiting for grandchild inherited streams"
    );
}

#[tokio::test]
async fn test_mock_executor_synthetic_streams() {
    let mock = MockCommandExecutor::new().with_synthetic_process(
        "qemu-system-aarch64",
        &["--monitor", "stdio"],
        SyntheticProcessSpec::new(9999, exit_status_from_code(0))
            .with_stdout(b"QEMU 8.2.0 monitor - (qemu)\n")
            .with_stderr(b"warning: tcg acceleration active\n")
            .with_delay(Duration::from_millis(20)),
    );

    let spec = CommandSpec::new("qemu-system-aarch64")
        .arg("--monitor")
        .arg("stdio")
        .stdout(StdioPolicy::Piped)
        .stderr(StdioPolicy::Piped);

    let mut handle = mock
        .spawn_typed(&spec)
        .await
        .expect("spawn_typed mock failed");
    assert_eq!(handle.pid(), 9999);

    // Read synthetic stdout stream
    let mut stdout_stream = handle.take_stdout().expect("stdout must be present");
    let mut stdout_buf = Vec::new();
    stdout_stream
        .read_to_end(&mut stdout_buf)
        .await
        .expect("Read stdout stream");
    assert_eq!(
        String::from_utf8_lossy(&stdout_buf),
        "QEMU 8.2.0 monitor - (qemu)\n"
    );

    // Read synthetic stderr stream
    let mut stderr_stream = handle.take_stderr().expect("stderr must be present");
    let mut stderr_buf = Vec::new();
    stderr_stream
        .read_to_end(&mut stderr_buf)
        .await
        .expect("Read stderr stream");
    assert_eq!(
        String::from_utf8_lossy(&stderr_buf),
        "warning: tcg acceleration active\n"
    );

    // Await mock process completion
    let status = handle.wait().await.expect("Mock wait failed");
    assert!(status.success());
    assert!(handle.is_reaped());
}

#[tokio::test]
async fn test_mock_executor_sustained_synthetic_stdin() {
    // Synthetic stdin must support sustained writes (>64 KiB) without blocking on 1KiB pipes
    let mock = MockCommandExecutor::new().with_synthetic_process(
        "test-daemon",
        &[],
        SyntheticProcessSpec::new(1001, exit_status_from_code(0)),
    );

    let spec = CommandSpec::new("test-daemon")
        .stdin(StdioPolicy::Piped)
        .stdout(StdioPolicy::Piped);

    let mut handle = mock.spawn_typed(&spec).await.expect("spawn_typed");
    let mut stdin = handle.take_stdin().expect("stdin stream");

    // Write 128 KiB of data to synthetic stdin
    let large_payload = vec![b'A'; 128 * 1024];
    stdin
        .write_all(&large_payload)
        .await
        .expect("Writing sustained payload to synthetic stdin must succeed without deadlock");
    drop(stdin);

    let status = handle.wait().await.expect("wait");
    assert!(status.success());
}

#[tokio::test]
async fn test_mock_executor_bidirectional_protocol() {
    // Test sustained bidirectional protocol use: client writes command, receives response
    let io_handler = Arc::new(
        |mut stdin_rx: tokio::io::DuplexStream, mut stdout_tx: tokio::io::DuplexStream| {
            Box::pin(async move {
                let mut buf = [0u8; 1024];
                while let Ok(n) = stdin_rx.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    let req = String::from_utf8_lossy(&buf[..n]);
                    if req.contains("PING") {
                        let _ = stdout_tx.write_all(b"PONG\n").await;
                    }
                }
            }) as std::pin::Pin<Box<dyn Future<Output = ()> + Send>>
        },
    );

    let mock = MockCommandExecutor::new().with_synthetic_process(
        "qmp-bridge",
        &[],
        SyntheticProcessSpec::new(2002, exit_status_from_code(0)).with_io_handler(io_handler),
    );

    let spec = CommandSpec::new("qmp-bridge")
        .stdin(StdioPolicy::Piped)
        .stdout(StdioPolicy::Piped);

    let mut handle = mock.spawn_typed(&spec).await.expect("spawn_typed");
    let mut stdin = handle.take_stdin().expect("stdin");
    let mut stdout = handle.take_stdout().expect("stdout");

    // Send PING 1
    stdin.write_all(b"PING 1\n").await.expect("write ping 1");
    let mut resp = [0u8; 5];
    stdout.read_exact(&mut resp).await.expect("read pong 1");
    assert_eq!(&resp, b"PONG\n");

    // Send PING 2
    stdin.write_all(b"PING 2\n").await.expect("write ping 2");
    stdout.read_exact(&mut resp).await.expect("read pong 2");
    assert_eq!(&resp, b"PONG\n");

    drop(stdin);
    let status = handle.wait().await.expect("wait");
    assert!(status.success());
}

#[tokio::test]
async fn test_mock_executor_no_legacy_fallback() {
    let mock = MockCommandExecutor::new()
        .with_typed_success("hdiutil", &["attach", "disk.dmg"], b"/dev/disk4s1\n")
        .with_typed_error("hdiutil", &["detach", "disk.dmg"], 1, b"resource busy\n")
        .with_success("uname", &["-m"], "arm64\n"); // legacy response

    // 1. Typed success works
    let spec_attach = CommandSpec::new("hdiutil").arg("attach").arg("disk.dmg");
    let attach_out = mock
        .run_typed(&spec_attach)
        .await
        .expect("run_typed attach");
    assert!(attach_out.success());
    assert_eq!(attach_out.stdout_str().unwrap(), "/dev/disk4s1\n");

    // 2. Typed error works
    let spec_detach = CommandSpec::new("hdiutil").arg("detach").arg("disk.dmg");
    let detach_out = mock
        .run_typed(&spec_detach)
        .await
        .expect("run_typed detach");
    assert!(!detach_out.success());
    assert_eq!(detach_out.code(), Some(1));
    assert_eq!(detach_out.stderr_str().unwrap(), "resource busy\n");

    // 3. No fallback to legacy response for typed runner: must return Err
    let spec_uname = CommandSpec::new("uname").arg("-m");
    let uname_res = mock.run_typed(&spec_uname).await;
    assert!(
        uname_res.is_err(),
        "run_typed must NOT fall back to legacy untyped mock responses"
    );
}

#[tokio::test]
async fn test_synthetic_handles_never_signal_os() {
    let self_pid = std::process::id();
    let mock = MockCommandExecutor::new().with_synthetic_process(
        "dummy-guest",
        &[],
        SyntheticProcessSpec::new(self_pid, exit_status_from_code(0)),
    );

    let spec = CommandSpec::new("dummy-guest");
    let handle = mock.spawn_typed(&spec).await.expect("spawn_typed");

    // Drop handle without reaping. If mock drop sent real SIGTERM, this test process would die!
    drop(handle);

    // Verify self process is still alive and running
    #[cfg(unix)]
    {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;
        let res = kill(Pid::from_raw(self_pid as i32), None);
        assert!(
            res.is_ok(),
            "Synthetic handle drop must never signal host process"
        );
    }
}
