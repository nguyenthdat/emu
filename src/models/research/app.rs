//! Application artifact domain data model.
//!
//! Defined in accordance with contracts/research.schema.json and data-model.md §2.6.

use super::types::{AppDeploymentStatus, ApplicationId, CpuArchitecture, Sha256Digest};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Owned compatible iOS application package or extracted bundle artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationArtifact {
    pub app_id: ApplicationId,
    pub bundle_identifier: String,
    pub bundle_name: String,
    pub package_path: PathBuf,
    pub sha256_digest: Sha256Digest,
    pub binary_architecture: CpuArchitecture,
    pub code_signature_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_container_path: Option<String>,
    pub entitlement_manifest: serde_json::Value,
    pub deployment_status: AppDeploymentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_guest_id: Option<uuid::Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

impl ApplicationArtifact {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        self.sha256_digest.validate()?;
        if !self.entitlement_manifest.is_object() {
            return Err(crate::models::error::ContractViolation::FieldViolation {
                field: "entitlement_manifest".to_string(),
                reason: "must be a JSON object".to_string(),
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
