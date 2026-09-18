//! Recovery baseline domain data model.
//!
//! Defined in accordance with contracts/research.schema.json.

use super::artifact::BootArtifactMap;
use super::types::{BackendType, Sha256Digest};
use serde::{Deserialize, Serialize};

/// Clean baseline state snapshot for fast rollback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryBaseline {
    pub baseline_id: String,
    pub guest_id: String,
    pub backend: BackendType,
    pub base_disk_digest: Sha256Digest,
    pub kernel_config_hash: Sha256Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_artifacts: Option<BootArtifactMap>,
    pub clean_snapshot_path: String,
    pub declared_recovery_deadline_ms: u64,
    pub verified: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    pub created_at: String,
}

impl RecoveryBaseline {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        if uuid::Uuid::parse_str(&self.guest_id).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidIdentifier {
                field: "guest_id".to_string(),
                value: self.guest_id.clone(),
                reason: "must be a valid UUID".to_string(),
            });
        }
        self.base_disk_digest.validate()?;
        self.kernel_config_hash.validate()?;
        if let Some(ba) = &self.boot_artifacts {
            ba.validate()?;
        }
        if self.declared_recovery_deadline_ms < 1000 {
            return Err(crate::models::error::ContractViolation::Constraint(
                "declared_recovery_deadline_ms must be >= 1000".to_string(),
            ));
        }
        if chrono::DateTime::parse_from_rfc3339(&self.created_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "created_at".to_string(),
                value: self.created_at.clone(),
            });
        }
        if let Some(va) = &self.verified_at {
            if chrono::DateTime::parse_from_rfc3339(va).is_err() {
                return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                    field: "verified_at".to_string(),
                    value: va.clone(),
                });
            }
        }
        Ok(())
    }
}
