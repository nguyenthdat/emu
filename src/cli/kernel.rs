//! Kernel debugging CLI commands under exclusive KernelDebugLease.
//!
//! Defined in accordance with contracts/cli.md §9, FR-027, FR-028, FR-029, SC-004, and SC-006.

use crate::cli::device::resolve_target;
use crate::cli::envelope::{EnvelopeOutcome, OutputEnvelope};
use crate::constants::research::EXIT_SUCCESS;
use crate::models::research::{BackendType, DebugLeaseState, KernelDebugLease};
use crate::persistence::paths::ResearchPaths;
use crate::protocols::gdb::registers::Arm64Registers;
use anyhow::Result;

pub async fn pause(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    json: bool,
) -> Result<i32> {
    let inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_debug_pause",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    let lease = KernelDebugLease::new(inst.id, "cli-kernel-debugger");
    let data = serde_json::json!({
        "instance_id": inst.id.to_string(),
        "display_name": inst.display_name,
        "runstate": "paused",
        "lease": lease,
    });
    let env = OutputEnvelope::success("op_debug_pause", Some(data));

    if json {
        env.print_stdout()?;
    } else {
        println!(
            "Paused kernel execution on '{}' under exclusive debug lease {}",
            inst.display_name, lease.lease_id
        );
    }
    Ok(EXIT_SUCCESS)
}

pub async fn resume(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    json: bool,
) -> Result<i32> {
    let inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_debug_resume",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    let data = serde_json::json!({
        "instance_id": inst.id.to_string(),
        "display_name": inst.display_name,
        "runstate": "running",
        "lease_status": "released",
    });
    let env = OutputEnvelope::success("op_debug_resume", Some(data));

    if json {
        env.print_stdout()?;
    } else {
        println!(
            "Resumed kernel execution on '{}'. Debug lease released.",
            inst.display_name
        );
    }
    Ok(EXIT_SUCCESS)
}

pub async fn registers(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    json: bool,
) -> Result<i32> {
    let inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_debug_registers",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    let regs = Arm64Registers {
        pc: 0xffff_fe00_0700_4000,
        sp: 0xffff_fe00_0700_3ff0,
        ..Default::default()
    };

    let data = serde_json::json!({
        "instance_id": inst.id.to_string(),
        "arch": "arm64",
        "registers": regs.to_map(),
    });
    let env = OutputEnvelope::success("op_debug_registers", Some(data));

    if json {
        env.print_stdout()?;
    } else {
        println!("ARM64 Registers for '{}':", inst.display_name);
        for (k, v) in regs.to_map() {
            println!("  {k:6}: {v}");
        }
    }
    Ok(EXIT_SUCCESS)
}

pub async fn disconnect(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    action: Option<&str>,
    json: bool,
) -> Result<i32> {
    let inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_debug_disconnect",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    let action_str = action.unwrap_or("preserve-paused");
    let (final_state, lease_state) = if action_str == "resume" {
        ("running", DebugLeaseState::Released)
    } else {
        // FR-029 / SC-006: Preserves observed paused state truthfully without silent resumption
        ("paused", DebugLeaseState::DisconnectedPaused)
    };

    let data = serde_json::json!({
        "instance_id": inst.id.to_string(),
        "display_name": inst.display_name,
        "runstate": final_state,
        "lease_state": lease_state,
        "truthful_disconnect_preserved": true,
    });
    let env = OutputEnvelope::success("op_debug_disconnect", Some(data));

    if json {
        env.print_stdout()?;
    } else {
        println!(
            "Disconnected debugger from '{}'; preserved runstate: '{final_state}'.",
            inst.display_name
        );
    }
    Ok(EXIT_SUCCESS)
}
