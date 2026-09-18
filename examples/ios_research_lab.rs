//! Opt-in physical laboratory trial runner for iOS Root VM and Darwin Security Research.
//!
//! Invoked via:
//! ```sh
//! cargo run --example ios_research_lab -- \
//!   --config <PATH> \
//!   --artifacts-dir ~/research_artifacts/ \
//!   --evidence-dir ~/.local/share/emu/research/records/ \
//!   --point-of-risk-consent
//! ```

use clap::Parser;
use std::path::PathBuf;

#[path = "../tests/lab/mod.rs"]
mod lab;

use lab::{LabCohortConfig, LabRunner};

#[derive(Parser, Debug)]
#[command(name = "ios_research_lab", about = "Opt-in laboratory trial runner")]
struct LabArgs {
    /// Path to cohort configuration JSON file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Path to user-supplied legal firmware and IPSW artifacts directory
    #[arg(long)]
    artifacts_dir: PathBuf,

    /// Directory where immutable ExperimentRecord files will be written
    #[arg(long)]
    evidence_dir: PathBuf,

    /// Explicit researcher consent to execute low-level hypervisor actions
    #[arg(long)]
    point_of_risk_consent: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = LabArgs::parse();

    let config = if let Some(path) = args.config {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str::<LabCohortConfig>(&content)?
    } else {
        LabCohortConfig::default()
    };

    let runner = LabRunner::new(
        config,
        args.artifacts_dir,
        args.evidence_dir,
        args.point_of_risk_consent,
    );

    runner.execute_cohort().await
}
