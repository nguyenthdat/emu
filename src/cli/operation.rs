//! Operation tracking, waiting, and cooperative cancellation CLI commands.

use crate::cli::envelope::{EnvelopeOutcome, EnvelopeStatus, OutputEnvelope};
use crate::constants::research::{EXIT_CANCELLED, EXIT_SUCCESS, EXIT_TIMEOUT};
use crate::models::error::ErrorRecord;
use crate::models::research::OperationStatus;
use crate::persistence::paths::ResearchPaths;
use anyhow::{Result, bail};
use std::time::{Duration, Instant};

pub async fn status(paths: &ResearchPaths, operation_id: &str, json: bool) -> Result<i32> {
    let maybe_op = crate::persistence::operations::load_operation(paths, operation_id).await?;
    let op = match maybe_op {
        Some(o) => o,
        None => bail!("Operation '{operation_id}' not found"),
    };

    let data = serde_json::to_value(&op)?;
    let env = match op.status {
        OperationStatus::Completed => OutputEnvelope::success(operation_id, Some(data)),
        OperationStatus::Failed => {
            let err = op.error.unwrap_or_else(|| {
                ErrorRecord::new(
                    crate::constants::research::ERR_RUNTIME_EXECUTION_ERROR,
                    "Operation failed during execution",
                    None,
                )
            });
            OutputEnvelope::execution_failed(operation_id, err, Some(data))
        }
        OperationStatus::CancellationPending => {
            let err = ErrorRecord::new(
                crate::constants::research::ERR_CANCELLATION_PENDING,
                "Operation cancellation requested and pending safe transaction boundary",
                None,
            );
            OutputEnvelope::cancellation_pending(operation_id, err, Some(data))
        }
        OperationStatus::Cancelled => {
            let err = ErrorRecord::new(
                crate::constants::research::ERR_CANCELLED,
                "Operation cancelled at safe transaction boundary",
                None,
            );
            OutputEnvelope::cancelled(operation_id, err, Some(data))
        }
        _ => OutputEnvelope::new(
            EnvelopeStatus::Success,
            EnvelopeOutcome::Completed,
            operation_id,
            Some(data),
            None,
        ),
    };

    if json {
        env.print_stdout()?;
    } else {
        println!("Operation: {}", op.operation_id);
        println!("Type:      {}", op.operation_type);
        println!("Status:    {:?}", op.status);
        println!("Phase:     {}", op.phase);
        if let Some(pct) = op.progress_percent {
            println!("Progress:  {pct}%");
        }
    }
    Ok(env.exit_code())
}

pub async fn cancel(paths: &ResearchPaths, operation_id: &str, json: bool) -> Result<i32> {
    let maybe_op = crate::persistence::operations::load_operation(paths, operation_id).await?;
    let mut op = match maybe_op {
        Some(o) => o,
        None => bail!("Operation '{operation_id}' not found"),
    };

    // Transition to cancellation_pending and settle to cancelled
    let _ = op.transition_to(OperationStatus::CancellationPending, "cancelling");
    let _ = op.transition_to(OperationStatus::Cancelled, "cleanup");
    op.completed_at = Some(chrono::Utc::now().to_rfc3339());
    op.phase = "cleanup".to_string();
    op.completed_at = Some(chrono::Utc::now().to_rfc3339());
    crate::persistence::operations::save_operation(paths, &op).await?;

    let data = serde_json::to_value(&op)?;
    let err = ErrorRecord::new(
        crate::constants::research::ERR_CANCELLED,
        format!("Operation '{operation_id}' successfully cancelled at safe boundary"),
        None,
    );
    let env = OutputEnvelope::cancelled(operation_id, err, Some(data));

    if json {
        env.print_stdout()?;
    } else {
        println!("Operation '{operation_id}' cancelled.");
    }
    Ok(EXIT_CANCELLED)
}

pub async fn wait(
    paths: &ResearchPaths,
    operation_id: &str,
    timeout_secs: Option<u64>,
    json: bool,
) -> Result<i32> {
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(30));
    let start = Instant::now();

    loop {
        let maybe_op = crate::persistence::operations::load_operation(paths, operation_id).await?;
        let op = match maybe_op {
            Some(o) => o,
            None => bail!("Operation '{operation_id}' not found"),
        };

        match op.status {
            OperationStatus::Completed => {
                let data = serde_json::to_value(&op)?;
                let env = OutputEnvelope::success(operation_id, Some(data));
                if json {
                    env.print_stdout()?;
                } else {
                    println!("Operation '{operation_id}' completed successfully.");
                }
                return Ok(EXIT_SUCCESS);
            }
            OperationStatus::Failed => {
                let data = serde_json::to_value(&op)?;
                let err = op.error.unwrap_or_else(|| {
                    ErrorRecord::new(
                        crate::constants::research::ERR_RUNTIME_EXECUTION_ERROR,
                        "Operation execution failed",
                        None,
                    )
                });
                let env = OutputEnvelope::execution_failed(operation_id, err, Some(data));
                if json {
                    env.print_stdout()?;
                }
                return Ok(env.exit_code());
            }
            OperationStatus::Cancelled => {
                let data = serde_json::to_value(&op)?;
                let err = ErrorRecord::new(
                    crate::constants::research::ERR_CANCELLED,
                    "Operation was cancelled",
                    None,
                );
                let env = OutputEnvelope::cancelled(operation_id, err, Some(data));
                if json {
                    env.print_stdout()?;
                }
                return Ok(EXIT_CANCELLED);
            }
            _ => {
                if start.elapsed() >= timeout {
                    let mut data = serde_json::to_value(&op)?;
                    if let Some(obj) = data.as_object_mut() {
                        obj.insert(
                            "observer_status".to_string(),
                            serde_json::Value::String("continuing".to_string()),
                        );
                    }
                    let err = ErrorRecord::new(
                        crate::constants::research::ERR_TIMEOUT,
                        format!(
                            "Caller wait deadline of {}s elapsed while operation '{operation_id}' continues executing",
                            timeout.as_secs()
                        ),
                        Some(
                            serde_json::json!({ "operation_id": operation_id, "actual_state": "continuing" }),
                        ),
                    );
                    let env = OutputEnvelope::timed_out(operation_id, err, Some(data));
                    if json {
                        env.print_stdout()?;
                    } else {
                        eprintln!("Wait deadline elapsed; task continues in background.");
                    }
                    return Ok(EXIT_TIMEOUT);
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}
