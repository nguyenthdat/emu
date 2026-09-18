//! Persistence, storage hierarchy, and filesystem constants for research backends.

/// Primary research data storage root directory name under local app data.
pub const DEFAULT_RESEARCH_DIR_NAME: &str = "research";

/// Subdirectory name for guest instance descriptors.
pub const DIR_INSTANCES: &str = "instances";

/// Subdirectory name for dedicated advisory lock files.
pub const DIR_LOCKS: &str = "locks";

/// Subdirectory name for research experiment profiles.
pub const DIR_PROFILES: &str = "profiles";

/// Subdirectory name for image and firmware artifact metadata.
pub const DIR_ARTIFACTS: &str = "artifacts";

/// Subdirectory name for recovery baseline state records.
pub const DIR_BASELINES: &str = "baselines";

/// Subdirectory name for immutable experiment audit records.
pub const DIR_RECORDS: &str = "records";

/// Subdirectory name for operation journals and streamed event logs.
pub const DIR_OPERATIONS: &str = "operations";

/// Subdirectory name for mutation proposals pending authorization.
pub const DIR_PROPOSALS: &str = "proposals";

/// Subdirectory name for guest security profiles.
pub const DIR_SECURITY_PROFILES: &str = "security_profiles";

/// Standard file extension for serialized JSON records.
pub const EXT_JSON: &str = "json";

/// Standard file extension for streamed JSONL event logs.
pub const EXT_EVENTS_JSONL: &str = "events.jsonl";

/// Suffix for instance advisory run lock files.
pub const SUFFIX_RUN_LOCK: &str = ".run.lock";

/// Suffix for instance target device lock files.
pub const SUFFIX_DEVICE_LOCK: &str = ".device.lock";

/// Suffix for operation worker lock files.
pub const SUFFIX_OP_LOCK: &str = ".op.lock";

/// Extension for temporary staged atomic write files.
pub const EXT_TMP: &str = "tmp";

/// Default prefix for private temporary runtime directories in `/tmp`.
pub const RUNTIME_DIR_PREFIX: &str = "emu-";

/// Strict Darwin Unix Domain Socket path length ceiling (`sizeof(sockaddr_un.sun_path)` = 104).
/// Includes the required terminating NUL byte.
pub const MAX_DARWIN_SUN_PATH: usize = 104;

/// Restrictive POSIX directory permissions mode (rwx------ / 0700).
pub const SECURE_DIR_MODE: u32 = 0o700;

/// Restrictive POSIX file permissions mode (rw------- / 0600).
pub const SECURE_FILE_MODE: u32 = 0o600;

/// Standard exit code returned upon advisory lock contention (Exit Code 5 / conflict).
pub const LOCK_CONTENTION_EXIT_CODE: i32 = 5;

/// Socket file name for QEMU Machine Protocol (QMP) control.
pub const SOCK_QMP: &str = "qmp.sock";

/// Socket file name for GDB RSP chardev kernel debugging.
pub const SOCK_GDB: &str = "gdb.sock";

/// Socket file name for virtio guest console bridge.
pub const SOCK_CONSOLE: &str = "console.sock";

/// Socket file name for supervisor IPC commands.
pub const SOCK_SUPERVISOR: &str = "supervisor.sock";

/// Socket file name for companion VM USB-over-IP tunneling.
pub const SOCK_INFERNO_USB: &str = "inferno-usb.sock";
