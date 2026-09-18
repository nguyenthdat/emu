//! Research guest instance domain data model.
//!
//! Defined in accordance with contracts/research.schema.json and data-model.md §2.2.

use super::artifact::BootArtifactMap;
use super::types::{
    BackendType, CpuArchitecture, InstanceLifecycleState, PrivilegeState, ResearchGuestId,
    RootVerificationState, Sha256Digest,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Registered research guest virtual machine instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchGuestInstance {
    pub id: ResearchGuestId,
    pub display_name: String,
    pub backend: BackendType,
    pub lifecycle_state: InstanceLifecycleState,
    pub guest_arch: CpuArchitecture,
    pub guest_os_version: String,
    pub build_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_session_id: Option<uuid::Uuid>,
    pub config_revision: Sha256Digest,
    pub base_image_ref: Sha256Digest,
    pub boot_artifacts: BootArtifactMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub runtime_dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qmp_socket_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gdb_socket_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console_socket_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_id: Option<String>,
    pub desired_privilege: PrivilegeState,
    pub observed_privilege: RootVerificationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_lock_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_lock_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_settled_at: String,
}

impl ResearchGuestInstance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ResearchGuestId,
        display_name: impl Into<String>,
        backend: BackendType,
        lifecycle_state: InstanceLifecycleState,
        guest_arch: CpuArchitecture,
        guest_os_version: impl Into<String>,
        build_identity: impl Into<String>,
        config_revision: Sha256Digest,
        base_image_ref: Sha256Digest,
        boot_artifacts: BootArtifactMap,
        runtime_dir: PathBuf,
        desired_privilege: PrivilegeState,
        observed_privilege: RootVerificationState,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            display_name: display_name.into(),
            backend,
            lifecycle_state,
            guest_arch,
            guest_os_version: guest_os_version.into(),
            build_identity: build_identity.into(),
            boot_session_id: None,
            config_revision,
            base_image_ref,
            boot_artifacts,
            profile_id: None,
            runtime_dir,
            qmp_socket_path: None,
            gdb_socket_path: None,
            console_socket_path: None,
            active_session_id: None,
            security_profile_id: None,
            baseline_id: None,
            desired_privilege,
            observed_privilege,
            run_lock_path: None,
            device_lock_path: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_settled_at: now,
        }
    }

    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        if self.display_name.trim().is_empty() {
            return Err(crate::models::error::ContractViolation::FieldViolation {
                field: "display_name".to_string(),
                reason: "display_name cannot be empty".to_string(),
            });
        }
        self.config_revision.validate()?;
        self.base_image_ref.validate()?;
        self.boot_artifacts.validate()?;

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
        if chrono::DateTime::parse_from_rfc3339(&self.last_settled_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "last_settled_at".to_string(),
                value: self.last_settled_at.clone(),
            });
        }
        Ok(())
    }
}
