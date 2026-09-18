//! Typed QMP command specifications and status queries.
//!
//! Defined in accordance with RFC 9003 and plan.md §Structure.

use super::codec::encode_command;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Response payload for QMP `query-status` command.
///
/// Invariant: `running == true` indicates only that the hypervisor vCPU loop is active.
/// It does NOT constitute proof of guest OS boot completion or root access readiness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QmpStatusResponse {
    pub running: bool,
    pub singlestep: bool,
    pub status: String,
}

impl QmpStatusResponse {
    /// Returns `true` if the hypervisor vCPU execution loop is running.
    ///
    /// Note: This is an architectural hardware runstate indicator, not evidence of
    /// guest OS boot readiness or in-guest daemon responsiveness.
    pub fn is_vcpu_running(&self) -> bool {
        self.running && self.status == "running"
    }

    /// Returns `true` if the virtual CPU is in paused / debug-halted state.
    pub fn is_paused(&self) -> bool {
        !self.running || self.status == "paused"
    }
}

/// Encodes `qmp_capabilities` handshake negotiation command.
pub fn qmp_capabilities(id: Option<&str>) -> Result<Vec<u8>> {
    encode_command("qmp_capabilities", None, id)
}

/// Encodes `stop` command to pause virtual CPU execution.
pub fn stop(id: Option<&str>) -> Result<Vec<u8>> {
    encode_command("stop", None, id)
}

/// Encodes `cont` command to resume virtual CPU execution.
pub fn cont(id: Option<&str>) -> Result<Vec<u8>> {
    encode_command("cont", None, id)
}

/// Encodes `system_powerdown` ACPI shutdown command.
pub fn system_powerdown(id: Option<&str>) -> Result<Vec<u8>> {
    encode_command("system_powerdown", None, id)
}

/// Encodes `system_reset` ACPI reset command.
pub fn system_reset(id: Option<&str>) -> Result<Vec<u8>> {
    encode_command("system_reset", None, id)
}

/// Encodes `quit` command to cleanly terminate hypervisor.
pub fn quit(id: Option<&str>) -> Result<Vec<u8>> {
    encode_command("quit", None, id)
}

/// Encodes `query-status` command to query vCPU runstate.
pub fn query_status(id: Option<&str>) -> Result<Vec<u8>> {
    encode_command("query-status", None, id)
}
