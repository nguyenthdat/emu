//! Experiment record audit and inspection CLI commands.
//!
//! Defined in accordance with contracts/cli.md §5 (Family 11), FR-042, and SC-018.

use crate::cli::envelope::{EnvelopeOutcome, OutputEnvelope};
use crate::constants::research::{ERR_INVALID_INPUT, EXIT_INVALID_INPUT, EXIT_SUCCESS};
use crate::models::error::ErrorRecord;
use crate::models::research::BackendType;
use crate::persistence::paths::ResearchPaths;
use crate::persistence::records::{list_records, load_record};
use anyhow::Result;

pub async fn inspect(paths: &ResearchPaths, record_id: &str, json: bool) -> Result<i32> {
    let maybe_record = load_record(paths, record_id).await?;
    let record = match maybe_record {
        Some(r) => r,
        None => {
            let err = ErrorRecord::new(
                ERR_INVALID_INPUT,
                format!("Experiment record '{record_id}' not found"),
                None,
            );
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_record_inspect",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(EXIT_INVALID_INPUT);
        }
    };

    let data = serde_json::to_value(&record)?;
    let env = OutputEnvelope::success("op_record_inspect", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!("Experiment Record '{}':", record.record_id);
        println!("  Profile:   {}", record.profile_id);
        println!("  Backend:   {}", record.backend);
        println!("  Status:    {:?}", record.execution_status);
        println!("  Started:   {}", record.started_at);
        println!("  Completed: {}", record.completed_at);
    }
    Ok(EXIT_SUCCESS)
}

pub async fn list(paths: &ResearchPaths, backend: Option<BackendType>, json: bool) -> Result<i32> {
    let mut records = list_records(paths).await?;
    if let Some(b) = backend {
        records.retain(|r| r.backend == b);
    }

    let data = serde_json::to_value(&records)?;
    let env = OutputEnvelope::success("op_record_list", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!("Stored Historical Experiment Records ({}):", records.len());
        for r in &records {
            println!(
                "- {} [{}] ({:?})",
                r.record_id, r.backend, r.execution_status
            );
        }
    }
    Ok(EXIT_SUCCESS)
}
