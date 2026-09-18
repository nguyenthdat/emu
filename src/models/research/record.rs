//! Experiment record domain data model.
//!
//! Defined in accordance with contracts/research.schema.json.

use super::proof::RootProofEvidence;
use super::types::{BackendType, DebugTelemetryRecord, HookStatus, OperationStatus, Sha256Digest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Immutable historical audit record of an executed research trial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentRecord {
    pub record_id: String,
    pub profile_id: String,
    pub backend: BackendType,
    pub upstream_build_identity: String,
    pub artifact_checksums: BTreeMap<String, Sha256Digest>,
    pub session_parameters: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_root_proof: Option<RootProofEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrumentation_summary: Option<HookStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_debug_telemetry: Option<DebugTelemetryRecord>,
    pub execution_status: OperationStatus,
    pub started_at: String,
    pub completed_at: String,
    pub errors: Vec<String>,
}

impl ExperimentRecord {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        if uuid::Uuid::parse_str(&self.record_id).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidIdentifier {
                field: "record_id".to_string(),
                value: self.record_id.clone(),
                reason: "must be a valid UUID".to_string(),
            });
        }
        for digest in self.artifact_checksums.values() {
            digest.validate()?;
        }
        if let Some(proof) = &self.observed_root_proof {
            proof.validate()?;
        }
        if chrono::DateTime::parse_from_rfc3339(&self.started_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "started_at".to_string(),
                value: self.started_at.clone(),
            });
        }
        if chrono::DateTime::parse_from_rfc3339(&self.completed_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "completed_at".to_string(),
                value: self.completed_at.clone(),
            });
        }
        Ok(())
    }
}
