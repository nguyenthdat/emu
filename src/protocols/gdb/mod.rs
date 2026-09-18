//! GDB Remote Serial Protocol (RSP) client, packet codec, and register mappings.

pub mod client;
pub mod lease;
pub mod packet;
pub mod registers;

pub use client::GdbClient;
pub use lease::{KernelDebugLease, KernelDebugLeaseGuard, KernelDebugLeaseRegistry};
pub use packet::{GdbStopReply, calculate_checksum, frame_packet, unframe_packet};
pub use registers::Arm64Registers;
