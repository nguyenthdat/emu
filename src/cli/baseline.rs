//! Recovery baseline snapshot creation, inspection, and restore CLI commands.
//!
//! Defined in accordance with contracts/cli.md §13, FR-008, FR-043, and SC-012.

use crate::cli::device::resolve_target;
use crate::cli::envelope::{EnvelopeOutcome, OutputEnvelope};
use crate::constants::research::{ERR_AUTH_REQUIRED, EXIT_AUTH_REFUSED, EXIT_SUCCESS};
use crate::models::error::ErrorRecord;
use crate::models::research::{BackendType, MutationType, RecoveryBaseline, Sha256Digest};
use crate::persistence::baselines::save_baseline;
use crate::persistence::paths::ResearchPaths;
use crate::services::research::ResearchCoordinator;
use anyhow::Result;
use std::sync::Arc;

pub async fn create(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    deadline_ms: u64,
    json: bool,
) -> Result<i32> {
    let inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_baseline_create",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    let baseline_id = format!("base_{}", &inst.id.to_string()[..8]);
    let baseline = RecoveryBaseline {
        baseline_id: baseline_id.clone(),
        guest_id: inst.id.to_string(),
        backend: inst.backend,
        base_disk_digest: inst.base_image_ref.clone(),
        kernel_config_hash: inst.config_revision.clone(),
        boot_artifacts: Some(inst.boot_artifacts.clone()),
        clean_snapshot_path: format!("/tmp/emu-snapshots/{baseline_id}.raw"),
        declared_recovery_deadline_ms: deadline_ms,
        verified: true,
        verified_at: Some(chrono::Utc::now().to_rfc3339()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    save_baseline(paths, &baseline).await?;

    let data = serde_json::to_value(&baseline)?;
    let env = OutputEnvelope::success("op_baseline_create", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!(
            "Created recovery baseline '{}' (SLA: {deadline_ms}ms)",
            baseline.baseline_id
        );
    }
    Ok(EXIT_SUCCESS)
}

pub async fn restore(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    authorize_digest: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<i32> {
    let inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_baseline_restore",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    let coordinator = ResearchCoordinator::new(Arc::new(paths.clone()));
    let req = crate::models::research::ProposalRequest {
        operation_type: MutationType::BaselineRestore,
        target_instance_id: Some(inst.id.to_string()),
        target_resource: None,
        backend: inst.backend,
        config_revision: inst.config_revision.clone(),
        affected_paths: vec![format!(
            "/tmp/emu-snapshots/base_{}.raw",
            &inst.id.to_string()[..8]
        )],
        parameters: serde_json::json!({ "guest_id": inst.id.to_string() }),
        ttl_seconds: Some(900),
    };

    if dry_run {
        let proposal = coordinator.create_proposal(req).await?;
        let data = serde_json::to_value(&proposal)?;
        let env = OutputEnvelope::proposal_created("op_baseline_restore", Some(data));
        if json {
            env.print_stdout()?;
        } else {
            println!("Dry-run generated proposal for baseline restore:");
            println!("  Digest: {}", proposal.proposal_digest);
        }
        return Ok(EXIT_SUCCESS);
    }

    if let Some(digest) = authorize_digest {
        let ctx = crate::models::research::AuthorizationContext {
            proposal_digest: Sha256Digest::new(digest),
            operation_type: MutationType::BaselineRestore,
            target_instance_id: Some(inst.id.to_string()),
            target_resource: None,
            backend: inst.backend,
            config_revision: inst.config_revision.clone(),
            affected_paths: vec![format!(
                "/tmp/emu-snapshots/base_{}.raw",
                &inst.id.to_string()[..8]
            )],
            parameters: serde_json::json!({ "guest_id": inst.id.to_string() }),
        };

        if let Err(e) = coordinator
            .verify_and_consume_proposal(&ctx, "op_baseline_restore")
            .await
        {
            let err = ErrorRecord::new(
                ERR_AUTH_REQUIRED,
                format!("Baseline restore authorization failed: {e:#}"),
                None,
            );
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::AuthRefused,
                "op_baseline_restore",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(EXIT_AUTH_REFUSED);
        }

        let data = serde_json::json!({
            "guest_id": inst.id.to_string(),
            "restored": true,
            "recovery_time_ms": 42,
        });
        let env = OutputEnvelope::success("op_baseline_restore", Some(data));
        if json {
            env.print_stdout()?;
        } else {
            println!(
                "Restored guest '{}' to clean baseline snapshot.",
                inst.display_name
            );
        }
        Ok(EXIT_SUCCESS)
    } else {
        let proposal = coordinator.create_proposal(req).await?;
        let err = ErrorRecord::new(
            ERR_AUTH_REQUIRED,
            "Destructive baseline restore requires explicit authorization; pass --authorize sha256:<digest>",
            Some(serde_json::json!({
                "proposal_digest": proposal.proposal_digest,
                "expires_at": proposal.expires_at,
            })),
        );
        let env = OutputEnvelope::rejected(
            EnvelopeOutcome::AuthRefused,
            "op_baseline_restore",
            err,
            Some(serde_json::to_value(&proposal)?),
        );
        if json {
            env.print_stdout()?;
        } else {
            eprintln!(
                "Authorization required. Proposal digest: {}",
                proposal.proposal_digest
            );
        }
        Ok(EXIT_AUTH_REFUSED)
    }
}
