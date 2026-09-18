//! Centralized constants for iOS Root VM and Darwin Security Research Backends.

use std::time::Duration;

pub mod contracts;
pub mod persistence;
pub mod process;
pub mod protocol;
pub mod storage;

// ============================================================================
// Process Exit Codes (contracts/cli.md §2, FR-045)
// ============================================================================

/// Exit code 0: Operation completed successfully, idempotent no-op satisfied,
/// or dry-run mutation proposal created.
pub const EXIT_SUCCESS: i32 = 0;

/// Exit code 1: Runtime failure (hypervisor crash, probe failure, script error).
pub const EXIT_RUNTIME_FAILURE: i32 = 1;

/// Exit code 2: Invalid input (missing parameter, ambiguous name, non-ARM64 Mach-O).
pub const EXIT_INVALID_INPUT: i32 = 2;

/// Exit code 3: Capability unsupported (non-Darwin host, missing app frameworks).
pub const EXIT_UNSUPPORTED: i32 = 3;

/// Exit code 4: Authorization refused (missing two-step auth or unauthenticated sudo).
pub const EXIT_AUTH_REFUSED: i32 = 4;

/// Exit code 5: Concurrency or state conflict (exclusive lock held, dependent guest active).
pub const EXIT_CONFLICT: i32 = 5;

/// Exit code 124: Operation timed out or caller wait deadline elapsed.
pub const EXIT_TIMEOUT: i32 = 124;

/// Exit code 130: Explicitly cancelled at verified safe transaction boundary.
pub const EXIT_CANCELLED: i32 = 130;

// ============================================================================
// Performance & Latency Budgets (Constitution §IV, SC-015)
// ============================================================================

/// Terminal input polling interval (8ms ~ 120fps).
pub const RESEARCH_INPUT_POLL_INTERVAL: Duration = crate::constants::timeouts::EVENT_POLL_TIMEOUT;

/// Cold startup deadline (<150ms).
pub const RESEARCH_STARTUP_TIMEOUT: Duration = Duration::from_millis(150);

/// Real-time log streaming latency ceiling (10ms).
pub const RESEARCH_LOG_STREAMING_TIMEOUT: Duration =
    crate::constants::performance::EVENT_DEBOUNCE_TIMEOUT;

/// User interface navigation latency ceiling (100ms).
pub const RESEARCH_UI_ACK_TIMEOUT: Duration = crate::constants::performance::DETAIL_UPDATE_DEBOUNCE;

/// Cancellation request acknowledgment ceiling (200ms).
pub const RESEARCH_CANCEL_ACK_TIMEOUT: Duration = crate::constants::performance::KEY_REPEAT_DELAY;

/// Default operation wait timeout (30 seconds).
pub const RESEARCH_DEFAULT_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Default proposal validity expiration (15 minutes).
pub const RESEARCH_PROPOSAL_TTL: Duration = Duration::from_secs(900);

// ============================================================================
// IPC, Socket & Buffer Limits (contracts/cli.md §1.2, D-04)
// ============================================================================

/// Maximum Unix Domain Socket path length on Darwin (`sizeof(sockaddr_un.sun_path)` = 104).
pub const MAX_DARWIN_SUN_PATH: usize = 104;

/// Maximum IPC message frame size (1 MiB = 1,048,576 bytes).
pub const MAX_IPC_FRAME_SIZE: usize = 1_048_576;

/// Maximum IPC pending message queue depth.
pub const MAX_IPC_QUEUE_DEPTH: usize = 256;

/// Default temporary socket directory prefix in `/tmp`.
pub const RESEARCH_TEMP_DIR_PREFIX: &str = "emu-";

/// Subdirectory permissions for private runtime directories (0700).
pub const RESEARCH_SECURE_DIR_MODE: u32 = 0o700;

// ============================================================================
// Standardized Error Codes (16 standardized codes from FR-045 & T009)
// ============================================================================

pub const ERR_AUTH_REQUIRED: &str = "AUTH_REQUIRED";
pub const ERR_INVALID_INPUT: &str = "INVALID_INPUT";
pub const ERR_UNSUPPORTED_HOST: &str = "UNSUPPORTED_HOST";
pub const ERR_APP_FRAMEWORKS_UNAVAILABLE: &str = "APP_FRAMEWORKS_UNAVAILABLE";
pub const ERR_AMBIGUOUS_INSTANCE_NAME: &str = "AMBIGUOUS_INSTANCE_NAME";
pub const ERR_ARTIFACT_CORRUPTED: &str = "ARTIFACT_CORRUPTED";
pub const ERR_EXPERIMENTAL_OPT_IN_REQUIRED: &str = "EXPERIMENTAL_OPT_IN_REQUIRED";
pub const ERR_MISSING_BASELINE: &str = "MISSING_BASELINE";
pub const ERR_CONCURRENCY_CONFLICT: &str = "CONCURRENCY_CONFLICT";
pub const ERR_DEPENDENT_GUEST_ACTIVE: &str = "DEPENDENT_GUEST_ACTIVE";
pub const ERR_TIMEOUT: &str = "TIMEOUT";
pub const ERR_CANCELLED: &str = "CANCELLED";
pub const ERR_PROBE_VERIFICATION_FAILED: &str = "PROBE_VERIFICATION_FAILED";
pub const ERR_CANCELLATION_PENDING: &str = "CANCELLATION_PENDING";
pub const ERR_DEBUG_LEASE_CONFLICT: &str = "DEBUG_LEASE_CONFLICT";
pub const ERR_UNSAFE_MOUNT_DETECTED: &str = "UNSAFE_MOUNT_DETECTED";
pub const ERR_RUNTIME_EXECUTION_ERROR: &str = "RUNTIME_EXECUTION_ERROR";

// ============================================================================
// Storage Paths & Directories (plan.md §Storage, data-model.md §6)
// ============================================================================

pub const STORAGE_ROOT_DIR: &str = "research";
pub const STORAGE_INSTANCES_DIR: &str = "instances";
pub const STORAGE_INSTANCES_LOCKS_DIR: &str = "instances/locks";
pub const STORAGE_PROFILES_DIR: &str = "profiles";
pub const STORAGE_ARTIFACTS_DIR: &str = "artifacts";
pub const STORAGE_BASELINES_DIR: &str = "baselines";
pub const STORAGE_RECORDS_DIR: &str = "records";
pub const STORAGE_OPERATIONS_DIR: &str = "operations";
pub const STORAGE_OPERATIONS_LOCKS_DIR: &str = "operations/locks";
pub const STORAGE_PROPOSALS_DIR: &str = "proposals";
pub const STORAGE_SECURITY_PROFILES_DIR: &str = "security_profiles";
pub const STORAGE_LOCKS_DIR: &str = "locks";

// File extensions
pub const EXT_JSON: &str = "json";
pub const EXT_EVENTS_JSONL: &str = "events.jsonl";
pub const EXT_LOCK: &str = "lock";
pub const EXT_RUN_LOCK: &str = "run.lock";
pub const EXT_OP_LOCK: &str = "op.lock";
pub const EXT_DEVICE_LOCK: &str = "device.lock";

// Socket file names
pub const SOCK_QMP: &str = "qmp.sock";
pub const SOCK_GDB: &str = "gdb.sock";
pub const SOCK_CONSOLE: &str = "console.sock";
pub const SOCK_SUPERVISOR: &str = "supervisor.sock";
pub const SOCK_INFERNO_USB: &str = "inferno-usb.sock";

// ============================================================================
// Subcommands, Hidden Modes & Family Names
// ============================================================================

pub const CMD_RESEARCH: &str = "research";
pub const CMD_SUPERVISE_HIDDEN: &str = "__supervise";
pub const CMD_WORKER_HIDDEN: &str = "__worker";

pub const FAMILY_BACKEND: &str = "backend";
pub const FAMILY_GUEST: &str = "guest";
pub const FAMILY_ROOT: &str = "root";
pub const FAMILY_APP: &str = "app";
pub const FAMILY_FRIDA: &str = "frida";
pub const FAMILY_DEBUG: &str = "debug";
pub const FAMILY_IMAGE: &str = "image";
pub const FAMILY_COMPANION: &str = "companion";
pub const FAMILY_PROFILE: &str = "profile";
pub const FAMILY_BASELINE: &str = "baseline";
pub const FAMILY_OPERATION: &str = "operation";
pub const FAMILY_RECORD: &str = "record";

// Root Probe Constants
pub const PROBE_GUEST_PATH: &str = "/private/var/root/.emu_probe";
pub const PROBE_PAYLOAD: &[u8] = b"emu_root_probe_v1\n";
pub const PROBE_UNPRIVILEGED_UID: u32 = 501;
pub const PROBE_UNPRIVILEGED_GID: u32 = 501;

// Canonical Schema URL
pub const CANONICAL_SCHEMA_URL: &str = "https://emu.rs/schemas/v1/research-ios.schema.json";
pub const CANONICAL_ENVELOPE_SCHEMA: &str =
    "https://emu.rs/schemas/v1/research-ios.schema.json#/definitions/OutputEnvelope";
pub const CANONICAL_LOG_SCHEMA: &str =
    "https://emu.rs/schemas/v1/research-ios.schema.json#/definitions/StreamLogEnvelope";
