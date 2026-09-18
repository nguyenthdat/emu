//! Research experiment profile domain data model.
//!
//! Defined in accordance with contracts/research.schema.json.

use super::artifact::BootArtifactMap;
use super::security_profile::GuestSecurityProfile;
use super::types::{BackendType, Sha256Digest};
use serde::{Deserialize, Serialize};

/// Versioned, portable research experiment environment configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchExperimentProfile {
    pub profile_id: String,
    pub name: String,
    pub target_backend: BackendType,
    pub base_image_digest: Sha256Digest,
    pub boot_artifacts: BootArtifactMap,
    pub kernel_boot_args: String,
    pub applied_patches: Vec<String>,
    pub security_profile: GuestSecurityProfile,
    pub app_artifacts: Vec<Sha256Digest>,
    pub instrumentation_scripts: Vec<Sha256Digest>,
    pub guest_fixtures: Vec<String>,
    pub exported_at: String,
    pub version: String,
}

impl ResearchExperimentProfile {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        self.base_image_digest.validate()?;
        self.boot_artifacts.validate()?;
        self.security_profile.validate()?;
        for a in &self.app_artifacts {
            a.validate()?;
        }
        for s in &self.instrumentation_scripts {
            s.validate()?;
        }
        if chrono::DateTime::parse_from_rfc3339(&self.exported_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "exported_at".to_string(),
                value: self.exported_at.clone(),
            });
        }
        Ok(())
    }
}
