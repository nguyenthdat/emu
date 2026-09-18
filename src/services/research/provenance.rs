//! Provenance tracking and immutable experiment record management.
//!
//! Defined in accordance with contracts/research.schema.json, data-model.md §1.2 (Rule 9),
//! and FR-042/SC-018: records an executed research trial into an immutable historical
//! audit record using observed typed inputs with zero hardcoded build identities.

use crate::models::research::{
    BackendType, DebugTelemetryRecord, ExperimentRecord, HookStatus, OperationStatus,
    RootProofEvidence, Sha256Digest,
};
use crate::persistence::paths::ResearchPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Cohesive input record for an observed research trial execution.
///
/// Encapsulates all observed trial metadata, telemetry, and execution status
/// into a single cohesive structure, eliminating argument bloat and clippy suppression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialExecutionInput {
    pub profile_id: String,
    pub backend: BackendType,
    pub upstream_build_identity: String,
    pub artifact_checksums: BTreeMap<String, Sha256Digest>,
    pub session_parameters: BTreeMap<String, serde_json::Value>,
    pub observed_root_proof: Option<RootProofEvidence>,
    pub instrumentation_summary: Option<HookStatus>,
    pub kernel_debug_telemetry: Option<DebugTelemetryRecord>,
    pub execution_status: OperationStatus,
    pub started_at: String,
    pub errors: Vec<String>,
}

pub struct ProvenanceTracker<'a> {
    paths: &'a ResearchPaths,
}

impl<'a> ProvenanceTracker<'a> {
    pub fn new(paths: &'a ResearchPaths) -> Self {
        Self { paths }
    }

    /// Records an observed trial into an immutable ExperimentRecord file.
    ///
    /// CRITICAL INVARIANTS:
    /// 1. Uses observed `upstream_build_identity` directly; never hardcodes build identities.
    /// 2. Saves to `records/<record_id>.json` via atomic create-new semantics.
    /// 3. Returns the persisted immutable `ExperimentRecord`.
    pub async fn record_trial(&self, input: TrialExecutionInput) -> Result<ExperimentRecord> {
        let record_id = uuid::Uuid::new_v4().to_string();
        let completed_at = chrono::Utc::now().to_rfc3339();

        let record = ExperimentRecord {
            record_id,
            profile_id: input.profile_id,
            backend: input.backend,
            upstream_build_identity: input.upstream_build_identity,
            artifact_checksums: input.artifact_checksums,
            session_parameters: input.session_parameters,
            observed_root_proof: input.observed_root_proof,
            instrumentation_summary: input.instrumentation_summary,
            kernel_debug_telemetry: input.kernel_debug_telemetry,
            execution_status: input.execution_status,
            started_at: input.started_at,
            completed_at,
            errors: input.errors,
        };

        crate::persistence::records::save_record(self.paths, &record)
            .await
            .context("Failed to save immutable trial experiment record")?;

        Ok(record)
    }
}
