//! Protocol constants for QMP, GDB Remote Serial Protocol (RSP), and KernelDebugLease.
//!
//! Defined in accordance with RFC 9003, plan.md §Structure, and contracts/cli.md §9.

use std::time::Duration;

// ============================================================================
// QMP (QEMU Machine Protocol) Framing & Limits
// ============================================================================

/// Maximum JSON-lines frame size (1 MiB = 1,048,576 bytes).
/// Protects against unbounded buffering and memory exhaustion on malformed streams.
pub const MAX_JSONL_FRAME_SIZE: usize = 1_048_576;

/// Default timeout for QMP request/response synchronization.
pub const DEFAULT_QMP_TIMEOUT: Duration = Duration::from_secs(10);

/// Default queue depth for unsolicited lifecycle events.
pub const DEFAULT_QMP_EVENT_CAPACITY: usize = 256;

// ============================================================================
// GDB Remote Serial Protocol (RSP) Constants
// ============================================================================

/// Maximum GDB RSP packet buffer size (1 MiB = 1,048,576 bytes).
pub const MAX_GDB_PACKET_SIZE: usize = 1_048_576;

/// Default timeout for GDB packet exchanges and ACK receipt.
pub const DEFAULT_GDB_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum retransmission attempts upon receiving RSP NAK ('-').
pub const GDB_MAX_NAK_RETRIES: usize = 3;

/// Out-of-band execution halt interrupt byte (ASCII 0x03, Ctrl+C / ETX).
pub const GDB_BREAK_BYTE: u8 = 0x03;

/// RSP packet start delimiter ('$').
pub const GDB_PACKET_START: u8 = b'$';

/// RSP checksum delimiter ('#').
pub const GDB_PACKET_END: u8 = b'#';

/// RSP positive acknowledgment ('+').
pub const GDB_ACK: u8 = b'+';

/// RSP negative acknowledgment ('-').
pub const GDB_NAK: u8 = b'-';

/// RSP binary escape character ('}' / 0x7d).
pub const GDB_ESCAPE_CHAR: u8 = 0x7d;

/// RSP escape XOR mask (0x20).
pub const GDB_ESCAPE_XOR: u8 = 0x20;

// ============================================================================
// ARM64 Register Architectural Geometry
// ============================================================================

/// Number of 64-bit general-purpose registers (x0 - x30) in ARM64.
pub const ARM64_GP_REGISTER_COUNT: usize = 31;

/// Total payload length in bytes for ARM64 'g'/'G' packet (268 bytes).
/// Layout: 31 x 8 bytes (x0..x30) + 8 bytes (sp) + 8 bytes (pc) + 4 bytes (pstate).
pub const ARM64_GDB_REGISTER_BYTES: usize = 268;

// ============================================================================
// KernelDebugLease Defaults & Deadlines
// ============================================================================

/// Default expiration TTL for newly acquired KernelDebugLease (30 minutes).
pub const DEFAULT_LEASE_TTL: Duration = Duration::from_secs(1800);

/// Default heartbeat interval for active kernel debugging sessions.
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
