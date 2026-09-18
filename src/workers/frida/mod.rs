//! Frida dynamic instrumentation worker bridge and protocol.

pub mod bridge;
pub mod protocol;

pub use bridge::FridaWorkerBridge;
pub use protocol::{FridaWorkerRequest, FridaWorkerResponse};
