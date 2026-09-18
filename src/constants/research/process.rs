//! Process execution and lifecycle constants for research backends.

use std::time::Duration;

/// Default maximum buffer size for captured stdout and stderr (10 MiB).
/// Prevents host memory exhaustion from runaway guest/process logging.
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

/// Default graceful termination timeout before SIGKILL escalation.
pub const DEFAULT_SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Polling interval when awaiting child process reaping during termination.
pub const PROCESS_REAP_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Safe default PATH environment variable when executing commands in cleared environment.
pub const DEFAULT_SAFE_PATH: &str =
    "/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin:/opt/homebrew/bin";

/// Standard TERM setting for headless command execution.
pub const DEFAULT_TERM: &str = "dumb";

/// Maximum duration to await final stream draining after child process exit,
/// preventing indefinite hangs when grandchild background processes inherit stdout/stderr pipes.
pub const INHERITED_STREAM_DRAIN_TIMEOUT: Duration = Duration::from_millis(250);

/// Default buffer capacity for synthetic in-memory duplex streams (64 KiB).
/// Prevents blocking on sustained writes during synthetic protocol testing.
pub const SYNTHETIC_DUPLEX_BUFFER_BYTES: usize = 64 * 1024;
