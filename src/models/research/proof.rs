//! Root proof evidence domain data model.
//!
//! Defined in accordance with contracts/research.schema.json.

use super::types::{BackendType, ProbeOutcome, RootVerificationState, Sha256Digest};
use serde::{Deserialize, Serialize};

/// Empirical proof evidence of root privilege execution in an iOS-derived guest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootProofEvidence {
    pub evidence_id: String,
    pub guest_id: String,
    pub boot_session_id: String,
    pub backend: BackendType,
    pub guest_build_identity: String,
    pub image_artifact_digest: Sha256Digest,
    pub config_revision_hash: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_uid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub positive_probe_outcome: Option<ProbeOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_control_outcome: Option<ProbeOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_kernel_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_boot_args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benign_binary_digest: Option<Sha256Digest>,
    pub verification_state: RootVerificationState,
    pub verified_at: String,
    pub diagnostics: Vec<String>,
}

impl RootProofEvidence {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        if uuid::Uuid::parse_str(&self.evidence_id).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidIdentifier {
                field: "evidence_id".to_string(),
                value: self.evidence_id.clone(),
                reason: "must be a valid UUID".to_string(),
            });
        }
        if uuid::Uuid::parse_str(&self.guest_id).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidIdentifier {
                field: "guest_id".to_string(),
                value: self.guest_id.clone(),
                reason: "must be a valid UUID".to_string(),
            });
        }
        if uuid::Uuid::parse_str(&self.boot_session_id).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidIdentifier {
                field: "boot_session_id".to_string(),
                value: self.boot_session_id.clone(),
                reason: "must be a valid UUID".to_string(),
            });
        }
        self.image_artifact_digest.validate()?;
        self.config_revision_hash.validate()?;
        if chrono::DateTime::parse_from_rfc3339(&self.verified_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "verified_at".to_string(),
                value: self.verified_at.clone(),
            });
        }

        if self.verification_state == RootVerificationState::Verified {
            match self.verified_uid {
                Some(0) => {}
                Some(other) => {
                    return Err(crate::models::error::ContractViolation::InvalidRootProof(
                        format!("verified_uid must be 0 for verified state, got {other}"),
                    ));
                }
                None => {
                    return Err(crate::models::error::ContractViolation::InvalidRootProof(
                        "verified_uid is required for verified state".to_string(),
                    ));
                }
            }

            let pos = self.positive_probe_outcome.as_ref().ok_or_else(|| {
                crate::models::error::ContractViolation::InvalidRootProof(
                    "positive_probe_outcome is required for verified state".to_string(),
                )
            })?;
            pos.validate()?;
            let expected_payload = "emu_root_probe_v1\n";
            if pos.status != "success"
                || pos.executed_euid != 0
                || pos.content_verified != Some(true)
                || !pos.helper_executed
                || (pos.output != expected_payload && pos.output.trim() != "emu_root_probe_v1")
            {
                return Err(crate::models::error::ContractViolation::InvalidRootProof(
                    "positive_probe_outcome failed: status must be 'success', executed_euid must be 0, content_verified must be true, helper_executed must be true, output must match canonical probe payload bytes ('emu_root_probe_v1\\n')".to_string(),
                ));
            }

            let neg = self.negative_control_outcome.as_ref().ok_or_else(|| {
                crate::models::error::ContractViolation::InvalidRootProof(
                    "negative_control_outcome is required for verified state".to_string(),
                )
            })?;
            neg.validate()?;
            if neg.status != "denied"
                || neg.executed_euid < 1
                || !neg.expected_denial
                || !neg.helper_executed
            {
                return Err(crate::models::error::ContractViolation::InvalidRootProof(
                    "negative_control_outcome failed: status must be 'denied', executed_euid must be >= 1, expected_denial must be true, helper_executed must be true".to_string(),
                ));
            }
            match neg.syscall_errno {
                Some(1) | Some(13) => {}
                other => {
                    return Err(crate::models::error::ContractViolation::InvalidRootProof(
                        format!(
                            "negative_control_outcome syscall_errno must be 1 (EPERM) or 13 (EACCES), got {other:?}"
                        ),
                    ));
                }
            }
            match neg.syscall_error_name.as_deref() {
                Some("EPERM") | Some("EACCES") => {}
                other => {
                    return Err(crate::models::error::ContractViolation::InvalidRootProof(
                        format!(
                            "negative_control_outcome syscall_error_name must be 'EPERM' or 'EACCES', got {other:?}"
                        ),
                    ));
                }
            }

            let kver = self.observed_kernel_version.as_deref().ok_or_else(|| {
                crate::models::error::ContractViolation::InvalidRootProof(
                    "observed_kernel_version is required for verified state".to_string(),
                )
            })?;
            if kver.trim().is_empty() {
                return Err(crate::models::error::ContractViolation::InvalidRootProof(
                    "observed_kernel_version cannot be empty".to_string(),
                ));
            }

            if self.observed_boot_args.is_none() {
                return Err(crate::models::error::ContractViolation::InvalidRootProof(
                    "observed_boot_args is required for verified state".to_string(),
                ));
            }

            let benign = self.benign_binary_digest.as_ref().ok_or_else(|| {
                crate::models::error::ContractViolation::InvalidRootProof(
                    "benign_binary_digest is required for verified state".to_string(),
                )
            })?;
            benign.validate()?;
        }

        Ok(())
    }
}
