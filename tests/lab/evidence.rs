//! Laboratory evidence gathering and trial record emission.

use anyhow::Result;
use emu::models::research::{BackendType, ExperimentRecord, OperationStatus};
use emu::persistence::paths::ResearchPaths;
use emu::services::research::ProvenanceTracker;
use std::collections::BTreeMap;

pub struct LabEvidenceCollector<'a> {
    tracker: ProvenanceTracker<'a>,
}

impl<'a> LabEvidenceCollector<'a> {
    pub fn new(paths: &'a ResearchPaths) -> Self {
        Self {
            tracker: ProvenanceTracker::new(paths),
        }
    }

    pub async fn record_gate_trial(
        &self,
        gate_name: &str,
        backend: BackendType,
        status: OperationStatus,
        errors: Vec<String>,
    ) -> Result<ExperimentRecord> {
        let started_at = chrono::Utc::now().to_rfc3339();
        let input = emu::services::research::provenance::TrialExecutionInput {
            profile_id: format!("lab_gate_{gate_name}"),
            backend,
            upstream_build_identity: "1.0.0".to_string(),
            artifact_checksums: BTreeMap::new(),
            session_parameters: BTreeMap::new(),
            observed_root_proof: None,
            instrumentation_summary: None,
            kernel_debug_telemetry: None,
            execution_status: status,
            started_at,
            errors,
        };
        self.tracker.record_trial(input).await
    }
}
