//! Guest instance lifecycle CLI commands.

use crate::cli::envelope::{EnvelopeOutcome, OutputEnvelope};
use crate::constants::research::{
    ERR_AMBIGUOUS_INSTANCE_NAME, ERR_AUTH_REQUIRED, ERR_INVALID_INPUT, EXIT_AUTH_REFUSED,
    EXIT_SUCCESS,
};
use crate::models::error::ErrorRecord;
use crate::models::research::{
    BackendType, BootArtifactMap, CpuArchitecture, InstanceLifecycleState, MutationType,
    PrivilegeState, ResearchGuestId, ResearchGuestInstance, RootVerificationState, Sha256Digest,
};
use crate::persistence::instances::{
    delete_instance, list_instances, load_instance, save_instance,
};
use crate::persistence::paths::ResearchPaths;
use crate::services::research::ResearchCoordinator;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

pub async fn resolve_target(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
) -> Result<Result<ResearchGuestInstance, (String, ErrorRecord)>> {
    if let Some(id_str) = id {
        if let Some(inst) = load_instance(paths, id_str).await? {
            return Ok(Ok(inst));
        } else {
            return Ok(Err((
                "invalid_input".to_string(),
                ErrorRecord::new(
                    ERR_INVALID_INPUT,
                    format!("Guest instance with ID '{id_str}' not found"),
                    None,
                ),
            )));
        }
    }

    if let Some(name_str) = name {
        let all = list_instances(paths).await?;
        let matching: Vec<ResearchGuestInstance> = all
            .into_iter()
            .filter(|i| i.display_name == name_str)
            .filter(|i| backend.is_none() || Some(i.backend) == backend)
            .collect();

        if matching.is_empty() {
            return Ok(Err((
                "invalid_input".to_string(),
                ErrorRecord::new(
                    ERR_INVALID_INPUT,
                    format!("No guest instance found matching display name '{name_str}'"),
                    None,
                ),
            )));
        }

        if matching.len() > 1 {
            return Ok(Err((
                "invalid_input".to_string(),
                ErrorRecord::new(
                    ERR_AMBIGUOUS_INSTANCE_NAME,
                    format!(
                        "Display name '{name_str}' matches {} instances across backends; qualify with --backend <darwin-vm|Inferno> or provide --id <UUID>",
                        matching.len()
                    ),
                    Some(serde_json::json!({
                        "name": name_str,
                        "matching_ids": matching.iter().map(|m| m.id.to_string()).collect::<Vec<_>>()
                    })),
                ),
            )));
        }

        return Ok(Ok(matching.into_iter().next().unwrap()));
    }

    Ok(Err((
        "invalid_input".to_string(),
        ErrorRecord::new(
            ERR_INVALID_INPUT,
            "Target specification required: pass --id <UUID> or --name <NAME>",
            None,
        ),
    )))
}
#[allow(clippy::too_many_arguments)]
pub async fn create(
    paths: &ResearchPaths,
    name: &str,
    backend: BackendType,
    kernelcache: Option<String>,
    devicetree: Option<String>,
    root_disk: Option<String>,
    ramdisk: Option<String>,
    json: bool,
) -> Result<i32> {
    let id = ResearchGuestId::new();

    let mut boot_artifacts = BootArtifactMap::new(
        Sha256Digest::new(kernelcache.unwrap_or_else(|| {
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string()
        })),
        Sha256Digest::new(devicetree.unwrap_or_else(|| {
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string()
        })),
    );
    boot_artifacts.root_disk = root_disk.map(Sha256Digest::new);
    boot_artifacts.ramdisk = ramdisk.map(Sha256Digest::new);

    let instance = ResearchGuestInstance::new(
        id,
        name,
        backend,
        InstanceLifecycleState::Stopped,
        CpuArchitecture::Arm64,
        match backend {
            BackendType::DarwinVm => "Darwin 24.0.0",
            BackendType::Inferno => "iOS 14.0",
        },
        match backend {
            BackendType::DarwinVm => "darwin_bootkc",
            BackendType::Inferno => "18A5351d",
        },
        Sha256Digest::new(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        Sha256Digest::new(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        boot_artifacts,
        PathBuf::from(format!("/tmp/emu-{}", &id.to_string()[..8])),
        PrivilegeState::Root,
        RootVerificationState::Unverified,
    );

    save_instance(paths, &instance).await?;

    let data = serde_json::to_value(&instance)?;
    let env = OutputEnvelope::success(format!("op_create_{}", id.simple()), Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!(
            "Created guest instance '{}' (ID: {})",
            instance.display_name, instance.id
        );
    }
    Ok(EXIT_SUCCESS)
}

pub async fn list(
    paths: &ResearchPaths,
    backend_filter: Option<BackendType>,
    json: bool,
) -> Result<i32> {
    let mut instances = list_instances(paths).await?;
    if let Some(b) = backend_filter {
        instances.retain(|i| i.backend == b);
    }

    let data = serde_json::to_value(&instances)?;
    let env = OutputEnvelope::success("op_guest_list", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!("Registered Research Guests ({}):", instances.len());
        for inst in &instances {
            println!(
                "- {} [{}] ({:?}) - ID: {}",
                inst.display_name, inst.backend, inst.lifecycle_state, inst.id
            );
        }
    }
    Ok(EXIT_SUCCESS)
}

pub async fn inspect(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    json: bool,
) -> Result<i32> {
    match resolve_target(paths, id, name, backend).await? {
        Ok(inst) => {
            let data = serde_json::to_value(&inst)?;
            let env = OutputEnvelope::success("op_guest_inspect", Some(data));
            if json {
                env.print_stdout()?;
            } else {
                println!("Instance Details:");
                println!("  ID:           {}", inst.id);
                println!("  Name:         {}", inst.display_name);
                println!("  Backend:      {}", inst.backend);
                println!("  State:        {:?}", inst.lifecycle_state);
                println!("  Privilege:    {:?}", inst.observed_privilege);
            }
            Ok(EXIT_SUCCESS)
        }
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_guest_inspect",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            } else {
                eprintln!("Error: {}", env.error.unwrap().message);
            }
            Ok(code)
        }
    }
}

pub async fn start(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    json: bool,
) -> Result<i32> {
    match resolve_target(paths, id, name, backend).await? {
        Ok(inst) => {
            let start_res = match inst.backend {
                BackendType::DarwinVm => {
                    crate::managers::darwin_vm::lifecycle::start_darwin_instance(
                        paths,
                        &inst.id.to_string(),
                    )
                    .await
                }
                BackendType::Inferno => {
                    crate::managers::inferno::lifecycle::start_inferno_instance(
                        paths,
                        &inst.id.to_string(),
                    )
                    .await
                }
            };
            if let Err(e) = start_res {
                let err = ErrorRecord::new(
                    crate::constants::research::ERR_RUNTIME_EXECUTION_ERROR,
                    format!(
                        "Failed to start guest instance '{}': {e:#}",
                        inst.display_name
                    ),
                    None,
                );
                let env = OutputEnvelope::execution_failed(
                    format!("op_start_{}", inst.id.simple()),
                    err,
                    None,
                );
                if json {
                    env.print_stdout()?;
                } else {
                    eprintln!("Error: {e:#}");
                }
                return Ok(crate::constants::research::EXIT_RUNTIME_FAILURE);
            }

            let updated = load_instance(paths, &inst.id.to_string())
                .await?
                .unwrap_or(inst);
            let data = serde_json::to_value(&updated)?;
            let env =
                OutputEnvelope::success(format!("op_start_{}", updated.id.simple()), Some(data));
            if json {
                env.print_stdout()?;
            } else {
                println!("Started guest instance '{}'", updated.display_name);
            }
            Ok(EXIT_SUCCESS)
        }
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_guest_start",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            } else {
                eprintln!("Error: {}", env.error.unwrap().message);
            }
            Ok(code)
        }
    }
}

pub async fn stop(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    json: bool,
) -> Result<i32> {
    match resolve_target(paths, id, name, backend).await? {
        Ok(inst) => {
            let stop_res = match inst.backend {
                BackendType::DarwinVm => {
                    crate::managers::darwin_vm::lifecycle::stop_darwin_instance(
                        paths,
                        &inst.id.to_string(),
                    )
                    .await
                }
                BackendType::Inferno => {
                    crate::managers::inferno::lifecycle::stop_inferno_instance(
                        paths,
                        &inst.id.to_string(),
                    )
                    .await
                }
            };
            if let Err(e) = stop_res {
                let err = ErrorRecord::new(
                    crate::constants::research::ERR_RUNTIME_EXECUTION_ERROR,
                    format!(
                        "Failed to stop guest instance '{}': {e:#}",
                        inst.display_name
                    ),
                    None,
                );
                let env = OutputEnvelope::execution_failed(
                    format!("op_stop_{}", inst.id.simple()),
                    err,
                    None,
                );
                if json {
                    env.print_stdout()?;
                } else {
                    eprintln!("Error: {e:#}");
                }
                return Ok(crate::constants::research::EXIT_RUNTIME_FAILURE);
            }

            let updated = load_instance(paths, &inst.id.to_string())
                .await?
                .unwrap_or(inst);
            let data = serde_json::to_value(&updated)?;
            let env =
                OutputEnvelope::success(format!("op_stop_{}", updated.id.simple()), Some(data));
            if json {
                env.print_stdout()?;
            } else {
                println!("Stopped guest instance '{}'", updated.display_name);
            }
            Ok(EXIT_SUCCESS)
        }
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env =
                OutputEnvelope::rejected(EnvelopeOutcome::InvalidInput, "op_guest_stop", err, None);
            if json {
                env.print_stdout()?;
            } else {
                eprintln!("Error: {}", env.error.unwrap().message);
            }
            Ok(code)
        }
    }
}

pub async fn restart(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    json: bool,
) -> Result<i32> {
    match resolve_target(paths, id, name, backend).await? {
        Ok(inst) => {
            let _ = match inst.backend {
                BackendType::DarwinVm => {
                    crate::managers::darwin_vm::lifecycle::stop_darwin_instance(
                        paths,
                        &inst.id.to_string(),
                    )
                    .await
                }
                BackendType::Inferno => {
                    crate::managers::inferno::lifecycle::stop_inferno_instance(
                        paths,
                        &inst.id.to_string(),
                    )
                    .await
                }
            };
            let start_res = match inst.backend {
                BackendType::DarwinVm => {
                    crate::managers::darwin_vm::lifecycle::start_darwin_instance(
                        paths,
                        &inst.id.to_string(),
                    )
                    .await
                }
                BackendType::Inferno => {
                    crate::managers::inferno::lifecycle::start_inferno_instance(
                        paths,
                        &inst.id.to_string(),
                    )
                    .await
                }
            };
            if let Err(e) = start_res {
                let err = ErrorRecord::new(
                    crate::constants::research::ERR_RUNTIME_EXECUTION_ERROR,
                    format!(
                        "Failed to restart guest instance '{}': {e:#}",
                        inst.display_name
                    ),
                    None,
                );
                let env = OutputEnvelope::execution_failed(
                    format!("op_restart_{}", inst.id.simple()),
                    err,
                    None,
                );
                if json {
                    env.print_stdout()?;
                } else {
                    eprintln!("Error: {e:#}");
                }
                return Ok(crate::constants::research::EXIT_RUNTIME_FAILURE);
            }

            let updated = load_instance(paths, &inst.id.to_string())
                .await?
                .unwrap_or(inst);
            let data = serde_json::to_value(&updated)?;
            let env =
                OutputEnvelope::success(format!("op_restart_{}", updated.id.simple()), Some(data));
            if json {
                env.print_stdout()?;
            } else {
                println!(
                    "Restarted guest instance '{}' (Privilege reset to unverified)",
                    updated.display_name
                );
            }
            Ok(EXIT_SUCCESS)
        }
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_guest_restart",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            } else {
                eprintln!("Error: {}", env.error.unwrap().message);
            }
            Ok(code)
        }
    }
}

pub async fn delete(
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
                "op_guest_delete",
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

    // If dry-run requested, create proposal and return exit 0 (proposal_created)
    let inst_path = paths.instance_path(&inst.id.to_string())?;
    let req = crate::models::research::ProposalRequest {
        operation_type: MutationType::InstanceDelete,
        target_instance_id: Some(inst.id.to_string()),
        target_resource: None,
        backend: inst.backend,
        config_revision: inst.config_revision.clone(),
        affected_paths: vec![inst_path.display().to_string()],
        parameters: serde_json::json!({ "target_id": inst.id.to_string() }),
        ttl_seconds: Some(900),
    };

    if dry_run {
        let proposal = coordinator.create_proposal(req).await?;
        let data = serde_json::to_value(&proposal)?;
        let env = OutputEnvelope::proposal_created("op_delete_dry_run", Some(data));
        if json {
            env.print_stdout()?;
        } else {
            println!("Dry-run generated MutationProposal:");
            println!("  Digest:   {}", proposal.proposal_digest);
            println!("  Expires:  {}", proposal.expires_at);
        }
        return Ok(EXIT_SUCCESS);
    }

    // Check if authorization digest provided
    if let Some(digest) = authorize_digest {
        let ctx = crate::models::research::AuthorizationContext {
            proposal_digest: Sha256Digest::new(digest),
            operation_type: MutationType::InstanceDelete,
            target_instance_id: Some(inst.id.to_string()),
            target_resource: None,
            backend: inst.backend,
            config_revision: inst.config_revision.clone(),
            affected_paths: vec![inst_path.display().to_string()],
            parameters: serde_json::json!({ "target_id": inst.id.to_string() }),
        };
        if let Err(e) = coordinator
            .verify_and_consume_proposal(&ctx, "op_guest_delete")
            .await
        {
            let err = ErrorRecord::new(
                ERR_AUTH_REQUIRED,
                format!("Authorization verification failed: {e:#}"),
                None,
            );
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::AuthRefused,
                "op_guest_delete",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(EXIT_AUTH_REFUSED);
        }

        // Execute deletion
        delete_instance(paths, &inst.id.to_string()).await?;
        let env = OutputEnvelope::success(
            format!("op_delete_{}", inst.id.simple()),
            Some(serde_json::json!({ "deleted": true })),
        );
        if json {
            env.print_stdout()?;
        } else {
            println!("Deleted guest instance '{}'", inst.display_name);
        }
        Ok(EXIT_SUCCESS)
    } else {
        // Two-Step Safety Gate: create proposal and return exit 4 (AUTH_REQUIRED)
        let proposal = coordinator.create_proposal(req).await?;

        let err = ErrorRecord::new(
            ERR_AUTH_REQUIRED,
            "Destructive guest deletion requires explicit authorization; pass --authorize sha256:<digest>",
            Some(serde_json::json!({
                "proposal_digest": proposal.proposal_digest,
                "expires_at": proposal.expires_at,
                "affected_paths": proposal.affected_paths,
            })),
        );
        let env = OutputEnvelope::rejected(
            EnvelopeOutcome::AuthRefused,
            "op_guest_delete",
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
            eprintln!("Run again with: --authorize {}", proposal.proposal_digest);
        }
        Ok(EXIT_AUTH_REFUSED)
    }
}
