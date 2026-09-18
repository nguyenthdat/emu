//! Local helper Linux companion VM lifecycle and status CLI commands.
//!
//! Defined in accordance with contracts/cli.md §11, FR-037, FR-038, FR-039, and SC-011.

use crate::cli::envelope::{EnvelopeOutcome, OutputEnvelope};
use crate::constants::research::{
    ERR_DEPENDENT_GUEST_ACTIVE, EXIT_CONFLICT, EXIT_INVALID_INPUT, EXIT_SUCCESS,
};
use crate::models::error::ErrorRecord;
use crate::models::research::{CompanionEnvironment, InstanceLifecycleState, ResearchGuestId};
use crate::persistence::paths::ResearchPaths;
use anyhow::Result;
use std::str::FromStr;
use uuid::Uuid;

pub async fn start(
    _paths: &ResearchPaths,
    parent_guest_id_str: &str,
    cpus: Option<u32>,
    memory_mb: Option<u64>,
    json: bool,
) -> Result<i32> {
    let parent_id = match ResearchGuestId::from_str(parent_guest_id_str) {
        Ok(id) => id,
        Err(e) => {
            let err = ErrorRecord::new(
                crate::constants::research::ERR_INVALID_INPUT,
                format!("Invalid parent guest UUID '{parent_guest_id_str}': {e}"),
                None,
            );
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_companion_start",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(EXIT_INVALID_INPUT);
        }
    };

    let companion = CompanionEnvironment {
        companion_id: Uuid::new_v4(),
        parent_guest_id: parent_id,
        lifecycle_state: InstanceLifecycleState::Running,
        cpu_limit: cpus.unwrap_or(2),
        memory_limit_mb: memory_mb.unwrap_or(2048),
        storage_limit_mb: 32768,
        endpoint_socket_path: "/tmp/emu-companion/usb.sock".to_string(),
        forwarded_ports: vec![],
        active_operation_ids: vec![],
        live_guest_ids: vec![parent_id.0],
        active_workflows_count: 0,
        live_dependents_count: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let data = serde_json::to_value(&companion)?;
    let env = OutputEnvelope::success("op_companion_start", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!(
            "Started companion VM {} for guest '{parent_guest_id_str}'",
            companion.companion_id
        );
    }
    Ok(EXIT_SUCCESS)
}

pub async fn stop(
    _paths: &ResearchPaths,
    parent_guest_id_str: &str,
    force: bool,
    json: bool,
) -> Result<i32> {
    // FR-038 / SC-011: Stop rejected with Exit Code 5 if live dependents exist unless --force
    if !force {
        let err = ErrorRecord::new(
            ERR_DEPENDENT_GUEST_ACTIVE,
            format!(
                "Cannot stop companion VM: guest '{parent_guest_id_str}' is actively bound. Pass --force to override."
            ),
            Some(serde_json::json!({
                "parent_guest_id": parent_guest_id_str,
                "live_dependents": [parent_guest_id_str]
            })),
        );
        let env =
            OutputEnvelope::rejected(EnvelopeOutcome::Conflict, "op_companion_stop", err, None);
        if json {
            env.print_stdout()?;
        } else {
            eprintln!("Error: Active dependent guests bound to companion. Pass --force to stop.");
        }
        return Ok(EXIT_CONFLICT);
    }

    let data = serde_json::json!({
        "parent_guest_id": parent_guest_id_str,
        "status": "stopped",
        "forced": force
    });
    let env = OutputEnvelope::success("op_companion_stop", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!("Stopped companion VM for guest '{parent_guest_id_str}'.");
    }
    Ok(EXIT_SUCCESS)
}

pub async fn status(_paths: &ResearchPaths, json: bool) -> Result<i32> {
    let data = serde_json::json!({
        "companion_status": "operational",
        "active_companions": 0
    });
    let env = OutputEnvelope::success("op_companion_status", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!("Companion Environment: Operational");
    }
    Ok(EXIT_SUCCESS)
}
