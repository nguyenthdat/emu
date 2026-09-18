//! Research domain coordinator facade for operation dispatching and health checks.
//!
//! Defined in accordance with contracts/research.schema.json, data-model.md §1.2/§5.1/§5.2, and FR-008.

use super::reconciler::OperationReconciler;
use crate::models::research::{
    AuthorizationContext, BackendType, MutationProposal, OperationRecord, ProposalError,
    ProposalRequest,
};
use crate::persistence::lock::AdvisoryLock;
use crate::persistence::paths::{ResearchPaths, validate_path_component};
use anyhow::{Context, Result, bail};
use std::sync::Arc;

fn is_valid_sha256_digest(s: &str) -> bool {
    if !s.starts_with("sha256:") || s.len() != 71 {
        return false;
    }
    s[7..].chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

/// Active authorization guard holding exclusive target mutation lock and proposal lock.
///
/// CRITICAL INVARIANT (data-model.md §1.2 Rule 6, §5.2):
/// Retaining this guard keeps the target device/resource lock held continuously through
#[derive(Debug)]
pub struct AuthorizationGuard {
    pub proposal: MutationProposal,
    pub operation: OperationRecord,
    _target_lock: AdvisoryLock,
    _proposal_lock: AdvisoryLock,
}

impl AuthorizationGuard {
    pub fn proposal(&self) -> &MutationProposal {
        &self.proposal
    }

    pub fn operation(&self) -> &OperationRecord {
        &self.operation
    }
}

pub struct ResearchCoordinator {
    paths: Arc<ResearchPaths>,
    reconciler: OperationReconciler,
}

impl ResearchCoordinator {
    pub fn new(paths: Arc<ResearchPaths>) -> Self {
        Self {
            paths,
            reconciler: OperationReconciler::new(),
        }
    }

    pub fn paths(&self) -> &ResearchPaths {
        &self.paths
    }

    pub fn reconciler(&self) -> &OperationReconciler {
        &self.reconciler
    }

    /// Dispatches a new tracked asynchronous operation, saving its initial journal entry.
    pub async fn dispatch_operation(
        &self,
        op_type: impl Into<String>,
        target_instance_id: Option<String>,
        backend: Option<BackendType>,
    ) -> Result<OperationRecord> {
        let op_id = format!("op_{}", uuid::Uuid::new_v4().simple());
        let record = OperationRecord::new(op_id, op_type, target_instance_id, backend);
        crate::persistence::operations::save_operation(&self.paths, &record).await?;
        Ok(record)
    }

    /// Creates a mutation proposal with a cryptographically bound SHA-256 digest under the Two-Step Safety Gate.
    ///
    /// Requires non-zero config revision, valid target and paths, and persists the proposal to disk.
    pub async fn create_proposal(&self, req: ProposalRequest) -> Result<MutationProposal> {
        let now = chrono::Utc::now();
        let proposal = MutationProposal::create_from_request(req, now)
            .map_err(|e| anyhow::anyhow!("Proposal creation rejected: {e}"))?;

        crate::persistence::proposals::save_proposal(&self.paths, &proposal).await?;
        Ok(proposal)
    }

    /// Verifies and consumes a mutation proposal using its `AuthorizationContext` and required `operation_id`.
    ///
    /// CRITICAL INVARIANTS (data-model.md §1.2 Rule 6, §5.2):
    /// 1. Validates proposal digest format and path containment before constructing or acquiring locks.
    /// 2. Requires a matching OperationRecord and verifies matching target, type, and backend.
    /// 3. Acquires target mutation lock (`.device.lock` or resource lock) AND exclusive proposal lock.
    /// 4. Reloads live guest instance under lock to verify active config_revision has not changed.
    /// 5. Checks durable proposal tombstone to strictly prevent token resurrection/reuse.
    /// 6. Journals `consumed_proposal_digest` into OperationRecord BEFORE effects or deletion.
    /// 7. Deletes proposal file and publishes durable tombstone with parent directory fsync.
    /// 8. Returns an `AuthorizationGuard` retaining the target mutation lock through destructive effects.
    pub async fn verify_and_consume_proposal(
        &self,
        ctx: &AuthorizationContext,
        operation_id: &str,
    ) -> Result<AuthorizationGuard> {
        // 1. Strict validation of proposal digest format before touching paths or locks
        let digest_str = ctx.proposal_digest.as_str();
        if !is_valid_sha256_digest(digest_str) {
            return Err(ProposalError::InvalidDigestFormat(digest_str.to_string()).into());
        }
        validate_path_component(digest_str).context("Digest failed path component validation")?;

        // 2. Validate and load the tracking OperationRecord
        let op_record = match crate::persistence::operations::load_operation(
            &self.paths,
            operation_id,
        )
        .await?
        {
            Some(op) => op,
            None => bail!("Operation record '{operation_id}' not found for proposal consumption"),
        };

        if op_record.is_terminal() {
            bail!(
                "Cannot authorize proposal on terminal operation '{operation_id}' in state {:?}",
                op_record.status
            );
        }

        let expected_op_type = serde_json::to_string(&ctx.operation_type)
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_default();
        if op_record.operation_type != expected_op_type {
            bail!(
                "Operation type mismatch: operation record declares '{}', proposal requires '{}'",
                op_record.operation_type,
                expected_op_type
            );
        }

        if op_record.target_instance_id != ctx.target_instance_id {
            bail!(
                "Operation target instance mismatch: operation has '{:?}', proposal requires '{:?}'",
                op_record.target_instance_id,
                ctx.target_instance_id
            );
        }

        if op_record.backend != Some(ctx.backend) {
            bail!(
                "Operation backend mismatch: operation has '{:?}', proposal requires '{:?}'",
                op_record.backend,
                ctx.backend
            );
        }

        // 3. Acquire target mutation lock (instance device lock or resource lock)
        let target_lock = if let Some(inst_id) = &ctx.target_instance_id {
            let device_lock_path = self.paths.instance_device_lock_path(inst_id)?;
            let lock = AdvisoryLock::try_acquire(&device_lock_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to acquire target device lock for instance '{inst_id}' during proposal authorization"
                    )
                })?;

            // CRITICAL: Reload live instance state under lock to detect concurrent config drift
            let live_instance =
                match crate::persistence::instances::load_instance(&self.paths, inst_id).await? {
                    Some(inst) => inst,
                    None => bail!("Target instance '{inst_id}' does not exist on disk"),
                };

            if live_instance.config_revision != ctx.config_revision {
                bail!(
                    "Stale proposal authorization rejected: live instance config revision '{}' does not match proposal revision '{}'",
                    live_instance.config_revision.as_str(),
                    ctx.config_revision.as_str()
                );
            }

            if live_instance.backend != ctx.backend {
                bail!(
                    "Live instance backend '{:?}' does not match proposal backend '{:?}'",
                    live_instance.backend,
                    ctx.backend
                );
            }

            lock
        } else {
            let res_name = ctx.target_resource.as_deref().unwrap_or("pre_guest");
            let safe_res = validate_path_component(res_name)
                .context("Invalid target resource for lock path")?;
            let res_lock_path = self
                .paths
                .operation_locks()
                .join(format!("res_{safe_res}.lock"));
            AdvisoryLock::try_acquire(&res_lock_path)
                .await
                .with_context(|| format!("Failed to acquire resource lock for '{safe_res}'"))?
        };

        // 4. Acquire exclusive proposal lock
        let safe_digest = digest_str.replace(':', "_");
        validate_path_component(&safe_digest)
            .context("Sanitized digest failed path component validation")?;
        let prop_lock_path = self
            .paths
            .operation_locks()
            .join(format!("{safe_digest}.prop.lock"));
        let proposal_lock = AdvisoryLock::try_acquire(&prop_lock_path)
            .await
            .with_context(|| {
                format!("Failed to acquire exclusive proposal lock for '{digest_str}'")
            })?;

        // 5. Check durable tombstone before loading proposal
        if crate::persistence::proposals::is_proposal_consumed(&self.paths, digest_str)? {
            bail!(
                "Mutation proposal with digest '{digest_str}' was already consumed and cannot be reused"
            );
        }

        // 6. Load proposal from disk
        let prop =
            match crate::persistence::proposals::load_proposal(&self.paths, digest_str).await? {
                Some(p) => p,
                None => bail!("Mutation proposal with digest '{digest_str}' was not found on disk"),
            };

        // 7. Verify authorization context matches proposal exactly and has not expired
        let now = chrono::Utc::now();
        prop.verify_authorization(ctx, now)
            .map_err(|e| anyhow::anyhow!("Mutation authorization verification failed: {e}"))?;

        // 8. Journal consumed proposal digest into OperationRecord BEFORE effects or deletion
        let updated_op = crate::persistence::operations::update_operation_with_lock(
            &self.paths,
            operation_id,
            |op_rec| {
                op_rec.record_consumed_proposal(prop.proposal_digest.clone());
                Ok(())
            },
        )
        .await
        .context("Failed to journal consumed proposal digest into operation record")?;

        // 9. Mark proposal consumed on disk (deletes proposal file and creates durable tombstone with fsync)
        crate::persistence::proposals::mark_proposal_consumed(&self.paths, digest_str)
            .await
            .context("Failed to mark proposal as consumed on disk")?;

        Ok(AuthorizationGuard {
            proposal: prop,
            operation: updated_op,
            _target_lock: target_lock,
            _proposal_lock: proposal_lock,
        })
    }
}
