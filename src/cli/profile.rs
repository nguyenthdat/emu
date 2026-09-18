//! Research experiment profile and security profile CLI commands.
//!
//! Defined in accordance with contracts/cli.md §12, FR-008, FR-015, FR-030, and FR-047.

use crate::cli::device::resolve_target;
use crate::cli::envelope::{EnvelopeOutcome, OutputEnvelope};
use crate::constants::research::{
    ERR_AUTH_REQUIRED, ERR_INVALID_INPUT, EXIT_AUTH_REFUSED, EXIT_INVALID_INPUT, EXIT_SUCCESS,
};
use crate::models::error::ErrorRecord;
use crate::models::research::{
    BackendType, GuestSecurityProfile, MutationType, RootVerificationState,
};
use crate::persistence::paths::ResearchPaths;
use crate::services::research::ResearchCoordinator;
use anyhow::Result;
use std::sync::Arc;

pub async fn apply(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    profile_path: &str,
    authorize: Option<&str>,
    json: bool,
) -> Result<i32> {
    let mut inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_profile_apply",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    let profile_content = match tokio::fs::read_to_string(profile_path).await {
        Ok(c) => c,
        Err(e) => {
            let err = ErrorRecord::new(
                ERR_INVALID_INPUT,
                format!("Failed to read profile file at '{profile_path}': {e}"),
                None,
            );
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_profile_apply",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(EXIT_INVALID_INPUT);
        }
    };

    let desired_profile: GuestSecurityProfile = match serde_json::from_str(&profile_content) {
        Ok(p) => p,
        Err(e) => {
            let err = ErrorRecord::new(
                ERR_INVALID_INPUT,
                format!("Invalid profile JSON format: {e}"),
                None,
            );
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_profile_apply",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(EXIT_INVALID_INPUT);
        }
    };

    // Check if security policy relaxation is requested
    let is_relaxation = desired_profile.amfi_status
        == crate::models::research::AmfiStatus::AmfiGetOutOfMyWay
        || desired_profile.code_signing_mode == crate::models::research::CodeSigningMode::Disabled;

    if is_relaxation {
        let coordinator = ResearchCoordinator::new(Arc::new(paths.clone()));
        let req = crate::models::research::ProposalRequest {
            operation_type: MutationType::SecurityPolicyRelax,
            target_instance_id: Some(inst.id.to_string()),
            target_resource: None,
            backend: inst.backend,
            config_revision: inst.config_revision.clone(),
            affected_paths: vec![profile_path.to_string()],
            parameters: serde_json::json!({ "profile_id": desired_profile.profile_id }),
            ttl_seconds: Some(900),
        };

        if let Some(digest) = authorize {
            let ctx = crate::models::research::AuthorizationContext {
                proposal_digest: crate::models::research::Sha256Digest::new(digest),
                operation_type: MutationType::SecurityPolicyRelax,
                target_instance_id: Some(inst.id.to_string()),
                target_resource: None,
                backend: inst.backend,
                config_revision: inst.config_revision.clone(),
                affected_paths: vec![profile_path.to_string()],
                parameters: serde_json::json!({ "profile_id": desired_profile.profile_id }),
            };
            if let Err(e) = coordinator
                .verify_and_consume_proposal(&ctx, "op_profile_apply")
                .await
            {
                let err = ErrorRecord::new(
                    ERR_AUTH_REQUIRED,
                    format!("Security policy relaxation authorization failed: {e:#}"),
                    None,
                );
                let env = OutputEnvelope::rejected(
                    EnvelopeOutcome::AuthRefused,
                    "op_profile_apply",
                    err,
                    None,
                );
                if json {
                    env.print_stdout()?;
                }
                return Ok(EXIT_AUTH_REFUSED);
            }
        } else {
            let proposal = coordinator.create_proposal(req).await?;

            let err = ErrorRecord::new(
                ERR_AUTH_REQUIRED,
                "Security policy relaxation requires explicit authorization; pass --authorize sha256:<digest>",
                Some(serde_json::json!({
                    "proposal_digest": proposal.proposal_digest,
                    "expires_at": proposal.expires_at,
                })),
            );
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::AuthRefused,
                "op_profile_apply",
                err,
                Some(serde_json::to_value(&proposal)?),
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(EXIT_AUTH_REFUSED);
        }

        // FR-015: Security policy relaxation invalidates active root proof
        inst.observed_privilege = RootVerificationState::Unverified;
        inst.updated_at = chrono::Utc::now().to_rfc3339();
        crate::persistence::instances::save_instance(paths, &inst).await?;
    }

    let data = serde_json::json!({
        "instance_id": inst.id.to_string(),
        "applied_profile": desired_profile.profile_id,
        "observed_privilege": inst.observed_privilege,
    });
    let env = OutputEnvelope::success("op_profile_apply", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!(
            "Applied profile '{}' to guest '{}'",
            desired_profile.profile_id, inst.display_name
        );
    }
    Ok(EXIT_SUCCESS)
}
