//! Guest security profile domain data model.
//!
//! Defined in accordance with contracts/research.schema.json.

use super::types::{AmfiStatus, CodeSigningMode, KernelPrivilegeLevel, MountMode, SandboxMode};
use serde::{Deserialize, Serialize};

/// Custom guest security policy relaxation and isolation configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestSecurityProfile {
    pub profile_id: String,
    pub name: String,
    pub code_signing_mode: CodeSigningMode,
    pub amfi_status: AmfiStatus,
    pub sandbox_mode: SandboxMode,
    pub root_filesystem_mount: MountMode,
    pub kernel_privilege_level: KernelPrivilegeLevel,
    pub guest_relaxations: Vec<String>,
    pub baseline_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl GuestSecurityProfile {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        if self.profile_id.trim().is_empty() {
            return Err(crate::models::error::ContractViolation::FieldViolation {
                field: "profile_id".to_string(),
                reason: "profile_id cannot be empty".to_string(),
            });
        }
        if self.name.trim().is_empty() {
            return Err(crate::models::error::ContractViolation::FieldViolation {
                field: "name".to_string(),
                reason: "name cannot be empty".to_string(),
            });
        }
        if chrono::DateTime::parse_from_rfc3339(&self.created_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "created_at".to_string(),
                value: self.created_at.clone(),
            });
        }
        if chrono::DateTime::parse_from_rfc3339(&self.updated_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "updated_at".to_string(),
                value: self.updated_at.clone(),
            });
        }
        Ok(())
    }
}

impl Default for GuestSecurityProfile {
    fn default() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            profile_id: "secprof_default".to_string(),
            name: "Default Enforced Profile".to_string(),
            code_signing_mode: CodeSigningMode::Enforced,
            amfi_status: AmfiStatus::Enforced,
            sandbox_mode: SandboxMode::Enforced,
            root_filesystem_mount: MountMode::ReadOnly,
            kernel_privilege_level: KernelPrivilegeLevel::Stock,
            guest_relaxations: Vec::new(),
            baseline_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
