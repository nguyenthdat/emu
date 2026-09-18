//! Canonical schema contract constants, exit codes, and validation specifications
//! for iOS Root VM and Darwin Security Research Backends.
//!
//! Defined in accordance with contracts/research.schema.json and contracts/cli.md.

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
// Canonical Error Codes (17 standardized codes from research.schema.json#/definitions/ErrorRecord)
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
pub const ERR_DEBUG_LEASE_CONFLICT: &str = "DEBUG_LEASE_CONFLICT";
pub const ERR_UNSAFE_MOUNT_DETECTED: &str = "UNSAFE_MOUNT_DETECTED";
pub const ERR_RUNTIME_EXECUTION_ERROR: &str = "RUNTIME_EXECUTION_ERROR";
pub const ERR_CANCELLATION_PENDING: &str = "CANCELLATION_PENDING";

/// Complete slice of all 17 canonical error codes in research.schema.json.
pub const CANONICAL_ERROR_CODES: &[&str] = &[
    ERR_AUTH_REQUIRED,
    ERR_INVALID_INPUT,
    ERR_UNSUPPORTED_HOST,
    ERR_APP_FRAMEWORKS_UNAVAILABLE,
    ERR_AMBIGUOUS_INSTANCE_NAME,
    ERR_ARTIFACT_CORRUPTED,
    ERR_EXPERIMENTAL_OPT_IN_REQUIRED,
    ERR_MISSING_BASELINE,
    ERR_CONCURRENCY_CONFLICT,
    ERR_DEPENDENT_GUEST_ACTIVE,
    ERR_TIMEOUT,
    ERR_CANCELLED,
    ERR_PROBE_VERIFICATION_FAILED,
    ERR_DEBUG_LEASE_CONFLICT,
    ERR_UNSAFE_MOUNT_DETECTED,
    ERR_RUNTIME_EXECUTION_ERROR,
    ERR_CANCELLATION_PENDING,
];

// ============================================================================
// Canonical Schema URIs & Identifiers
// ============================================================================

pub const CANONICAL_SCHEMA_URL: &str = "https://emu.rs/schemas/v1/research-ios.schema.json";
pub const CANONICAL_ENVELOPE_SCHEMA: &str =
    "https://emu.rs/schemas/v1/research-ios.schema.json#/definitions/OutputEnvelope";

// ============================================================================
// Validation Patterns & Bounds
// ============================================================================

/// Operation ID pattern: `^op_[0-9A-Za-z]+$`
pub const OPERATION_ID_PREFIX: &str = "op_";

/// SHA-256 digest pattern: `^sha256:[a-f0-9]{64}$`
pub const SHA256_PREFIX: &str = "sha256:";
pub const SHA256_HEX_LEN: usize = 64;
pub const SHA256_TOTAL_LEN: usize = 71; // "sha256:".len() + 64

/// Canonical Frida agent version pinned by research schema
pub const CANONICAL_FRIDA_VERSION: &str = "17.18.0";

/// Canonical host OS and architecture for research backends
pub const CANONICAL_HOST_OS: &str = "macos";
pub const CANONICAL_HOST_ARCH: &str = "aarch64";

/// Supported hypervisor types
pub const HYPERVISOR_FRAMEWORK: &str = "hypervisor_framework";
pub const TCG_EMULATION: &str = "tcg_emulation";

/// Supported debug interfaces
pub const DEBUG_INTERFACE_GDB_RSP: &str = "gdb_rsp";
pub const DEBUG_INTERFACE_QMP_MONITOR: &str = "qmp_monitor";

/// Minimum declared recovery deadline in milliseconds
pub const MIN_RECOVERY_DEADLINE_MS: u64 = 1000;

/// Root Probe Constants
pub const PROBE_GUEST_PATH: &str = "/private/var/root/.emu_probe";
pub const PROBE_PAYLOAD: &[u8] = b"emu_root_probe_v1\n";
pub const PROBE_UNPRIVILEGED_UID: u32 = 501;
pub const PROBE_UNPRIVILEGED_GID: u32 = 501;
