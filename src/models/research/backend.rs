//! Backend capability profile and diagnostic models.
//!
//! Defined in accordance with contracts/research.schema.json and data-model.md §2.3.

use super::types::{
    BackendType, BinaryPrerequisite, CapabilityDetail, EntitlementPrerequisite, SupportStatus,
};
use serde::{Deserialize, Serialize};

/// Detailed host and hypervisor capability diagnostics for a research backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendCapabilityProfile {
    pub backend: BackendType,
    pub host_os: String,
    pub host_arch: String,
    pub hypervisor: String,
    pub support_status: SupportStatus,
    pub supported_guest_families: Vec<String>,
    pub headless_console_support: bool,
    pub graphical_display_support: bool,
    pub companion_vm_required: bool,
    pub app_frameworks_supported: bool,
    pub supported_debug_interfaces: Vec<String>,
    pub capabilities: CapabilityDetail,
    pub required_binaries: Vec<BinaryPrerequisite>,
    pub required_entitlements: Vec<EntitlementPrerequisite>,
    pub remediation_steps: Vec<String>,
}

impl BackendCapabilityProfile {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        if self.host_os != "macos" {
            return Err(crate::models::error::ContractViolation::FieldViolation {
                field: "host_os".to_string(),
                reason: format!("must be 'macos', got '{}'", self.host_os),
            });
        }
        if self.host_arch != "aarch64" {
            return Err(crate::models::error::ContractViolation::FieldViolation {
                field: "host_arch".to_string(),
                reason: format!("must be 'aarch64', got '{}'", self.host_arch),
            });
        }
        if self.hypervisor != "hypervisor_framework" && self.hypervisor != "tcg_emulation" {
            return Err(crate::models::error::ContractViolation::FieldViolation {
                field: "hypervisor".to_string(),
                reason: format!(
                    "must be 'hypervisor_framework' or 'tcg_emulation', got '{}'",
                    self.hypervisor
                ),
            });
        }
        for iface in &self.supported_debug_interfaces {
            if iface != "gdb_rsp" && iface != "qmp_monitor" {
                return Err(crate::models::error::ContractViolation::FieldViolation {
                    field: "supported_debug_interfaces".to_string(),
                    reason: format!("unknown interface '{iface}': must be gdb_rsp or qmp_monitor"),
                });
            }
        }
        Ok(())
    }
}
