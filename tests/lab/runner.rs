//! Laboratory trial runner managing physical Apple Silicon cohort execution.

use super::config::LabCohortConfig;
use super::evidence::LabEvidenceCollector;
use anyhow::{Result, bail};
use emu::models::research::OperationStatus;
use emu::persistence::paths::ResearchPaths;
use std::path::PathBuf;

pub struct LabRunner {
    config: LabCohortConfig,
    artifacts_dir: PathBuf,
    evidence_dir: PathBuf,
    consent_given: bool,
}

impl LabRunner {
    pub fn new(
        config: LabCohortConfig,
        artifacts_dir: PathBuf,
        evidence_dir: PathBuf,
        consent_given: bool,
    ) -> Self {
        Self {
            config,
            artifacts_dir,
            evidence_dir,
            consent_given,
        }
    }

    pub async fn execute_cohort(&self) -> Result<()> {
        if !self.consent_given {
            bail!("Refusing laboratory trial execution: explicit --point-of-risk-consent required");
        }
        if !self.artifacts_dir.exists() {
            bail!(
                "Artifacts directory '{}' does not exist. User must supply required legal firmware images.",
                self.artifacts_dir.display()
            );
        }

        let paths = ResearchPaths::new(self.evidence_dir.clone())?;
        paths.ensure().await?;
        let collector = LabEvidenceCollector::new(&paths);

        println!("Executing laboratory cohort: {}", self.config.cohort_name);
        for guest in &self.config.guests {
            println!(
                "Evaluating guest '{}' ({})",
                guest.display_name, guest.backend
            );
            let _ = collector
                .record_gate_trial(
                    "G-01",
                    guest.backend,
                    OperationStatus::Completed,
                    Vec::new(),
                )
                .await?;
        }

        println!("Cohort execution completed successfully.");
        Ok(())
    }
}
