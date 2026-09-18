//! Reconciler and crash recovery for research supervisors and operations.
//!
//! Defined in accordance with data-model.md §1.2 (Rule 7) and FR-047:
//! The reconciler never performs automatic retries on failed mutating steps.
//! Lost worker processes are marked as failed or cancelled, and supervisors that
//! exited while running are transitioned to error without guessing or killing unverified PIDs.

use crate::constants::research::ERR_RUNTIME_EXECUTION_ERROR;
use crate::models::error::ErrorRecord;
use crate::models::research::{InstanceLifecycleState, OperationStatus};
use crate::persistence::paths::ResearchPaths;
use anyhow::{Context, Result};

pub struct OperationReconciler;

impl Default for OperationReconciler {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationReconciler {
    pub fn new() -> Self {
        Self
    }

    /// Inspects registered instances and reconciles orphaned run locks from lost supervisors.
    ///
    /// CRITICAL INVARIANT:
    /// Does not guess or kill unverified PIDs. Truthfully records error state.
    pub async fn reconcile_instances(&self, paths: &ResearchPaths) -> Result<usize> {
        let instances = crate::persistence::instances::list_instances(paths).await?;
        let mut reconciled = 0;

        for mut inst in instances {
            // Check active states where a supervisor process should hold the run lock
            let is_active = matches!(
                inst.lifecycle_state,
                InstanceLifecycleState::Running
                    | InstanceLifecycleState::Booting
                    | InstanceLifecycleState::Stopping
            );

            if is_active {
                let lock_path = paths.instance_run_lock_path(&inst.id.to_string())?;
                // If run lock can be acquired exclusively, supervisor process has terminated
                if let Ok(lock) =
                    crate::persistence::lock::AdvisoryLock::try_acquire(&lock_path).await
                {
                    inst.lifecycle_state = InstanceLifecycleState::Error;
                    inst.updated_at = chrono::Utc::now().to_rfc3339();
                    crate::persistence::instances::save_instance(paths, &inst)
                        .await
                        .context("Failed to persist reconciled instance error state")?;
                    drop(lock);
                    reconciled += 1;
                }
            }
        }

        Ok(reconciled)
    }

    /// Reconciles non-terminal operations where the worker process exited unexpectedly.
    ///
    /// CRITICAL INVARIANTS (FR-047):
    /// 1. Zero automatic retries: failed operations are marked `Failed`, never re-dispatched.
    /// 2. Never infers success: missing worker results are marked failed with `ERR_RUNTIME_EXECUTION_ERROR`.
    /// 3. A lost worker in `CancellationPending` provides NO safe-boundary acknowledgment.
    ///    It is recorded as `Failed`, NEVER `Cancelled`. Only explicit worker acknowledgment produces `Cancelled`.
    pub async fn reconcile_operations(&self, paths: &ResearchPaths) -> Result<usize> {
        let operations = crate::persistence::operations::list_operations(paths).await?;
        let mut reconciled = 0;

        for mut op in operations {
            if op.is_terminal() {
                continue;
            }

            let lock_path = paths.operation_lock_path(&op.operation_id)?;
            // If operation lock can be acquired exclusively, worker process has terminated
            if let Ok(lock) = crate::persistence::lock::AdvisoryLock::try_acquire(&lock_path).await
            {
                let error_msg = if op.status == OperationStatus::CancellationPending {
                    "Worker process terminated unexpectedly while cancellation was pending without acknowledging clean boundary cancellation; automatic retries prohibited per FR-047"
                } else {
                    "Worker process terminated unexpectedly before completing operation; automatic retries prohibited per FR-047"
                };

                op.fail(
                    ErrorRecord::new(ERR_RUNTIME_EXECUTION_ERROR, error_msg, None),
                    "teardown",
                )
                .context("Failed to apply guarded fail transition during reconciliation")?;

                crate::persistence::operations::save_operation_unlocked(paths, &op)
                    .await
                    .context("Failed to persist reconciled operation state")?;

                drop(lock);
                reconciled += 1;
            }
        }

        Ok(reconciled)
    }
}
