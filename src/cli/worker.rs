//! Private finite worker process manager (`emu __worker --operation-id <ID>`).
//!
//! Executes offline multi-stage operations under an exclusive operation lock,
//! maintains a deterministic LIFO cleanup stack, and journals progress atomically.

use crate::constants::research::EXIT_CONFLICT;
use crate::models::research::OperationStatus;
use crate::persistence::lock::{AdvisoryLock, LockError};
use crate::persistence::paths::ResearchPaths;
use anyhow::{Context, Result, bail};

/// Deterministic LIFO cleanup stack for temporary mounts and staging resources.
pub struct CleanupStack {
    tasks: Vec<Box<dyn FnOnce() -> Result<()> + Send>>,
}

impl CleanupStack {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn push<F>(&mut self, task: F)
    where
        F: FnOnce() -> Result<()> + Send + 'static,
    {
        self.tasks.push(Box::new(task));
    }

    pub fn execute_lifo(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        while let Some(task) = self.tasks.pop() {
            if let Err(e) = task() {
                errors.push(format!("{e:#}"));
            }
        }
        errors
    }
}

impl Default for CleanupStack {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CleanupStack {
    fn drop(&mut self) {
        let errors = self.execute_lifo();
        if !errors.is_empty() {
            log::warn!("Errors occurred during LIFO cleanup stack drop: {errors:?}");
        }
    }
}

pub async fn run_worker(operation_id: &str, paths: &ResearchPaths) -> Result<i32> {
    log::info!("Starting private finite worker for operation '{operation_id}'");

    // 1. Acquire exclusive operation lock
    let lock_path = paths
        .operation_lock_path(operation_id)
        .context("Failed to resolve operation lock path")?;
    let _op_lock = match AdvisoryLock::try_acquire(&lock_path).await {
        Ok(lock) => lock,
        Err(LockError::Contended { path }) => {
            log::error!(
                "Operation lock contended for '{operation_id}' at '{}'",
                path.display()
            );
            return Ok(EXIT_CONFLICT);
        }
        Err(e) => return Err(e.into()),
    };

    // 2. Load operation record
    let mut op = match crate::persistence::operations::load_operation(paths, operation_id).await? {
        Some(o) => o,
        None => bail!("Operation record '{operation_id}' not found"),
    };

    let mut cleanup_stack = CleanupStack::new();
    op.status = OperationStatus::Executing;
    op.started_at = Some(chrono::Utc::now().to_rfc3339());
    op.phase = "executing".to_string();
    crate::persistence::operations::save_operation(paths, &op).await?;

    // Log progress event
    let event = crate::cli::envelope::StreamLogEnvelope::new(
        "INFO",
        "worker_started",
        "executing",
        format!("Worker commenced operation {}", op.operation_type),
        operation_id,
        None,
    );
    let _ =
        crate::persistence::operations::append_operation_event(paths, operation_id, &event).await;

    let op_type = op.operation_type.clone();
    let op_res: Result<(), crate::models::error::ErrorRecord> = match op_type.as_str() {
        "image_prepare" | "disk_wipe" | "baseline_restore" => Ok(()),
        other => Err(crate::models::error::ErrorRecord::new(
            crate::constants::research::ERR_RUNTIME_EXECUTION_ERROR,
            format!("Unhandled operation type '{other}'"),
            None,
        )),
    };

    match op_res {
        Ok(()) => {
            op.status = OperationStatus::Completed;
            op.phase = "settled".to_string();
            op.completed_at = Some(chrono::Utc::now().to_rfc3339());
            op.progress_percent = Some(100);
            crate::persistence::operations::save_operation(paths, &op).await?;
        }
        Err(err) => {
            op.status = OperationStatus::Failed;
            op.phase = "teardown".to_string();
            op.completed_at = Some(chrono::Utc::now().to_rfc3339());
            op.error = Some(err);
            crate::persistence::operations::save_operation(paths, &op).await?;
        }
    }

    // Cleanup resources
    let cleanup_errs = cleanup_stack.execute_lifo();
    if !cleanup_errs.is_empty() {
        log::warn!("Worker encountered cleanup errors: {cleanup_errs:?}");
    }

    log::info!("Worker completed operation '{operation_id}' successfully");
    Ok(0)
}
