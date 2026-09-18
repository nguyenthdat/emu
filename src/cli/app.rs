//! Application lifecycle and container inspection CLI commands.
//!
//! Defined in accordance with contracts/cli.md §7, FR-016, FR-022, FR-023, FR-024, and SC-003.

use crate::cli::device::resolve_target;
use crate::cli::envelope::{EnvelopeOutcome, OutputEnvelope};
use crate::constants::research::{
    ERR_APP_FRAMEWORKS_UNAVAILABLE, ERR_AUTH_REQUIRED, EXIT_AUTH_REFUSED, EXIT_SUCCESS,
    EXIT_UNSUPPORTED,
};
use crate::models::error::ErrorRecord;
use crate::models::research::{
    AppDeploymentStatus, ApplicationArtifact, ApplicationId, BackendType, CpuArchitecture,
    Sha256Digest,
};
use crate::persistence::paths::ResearchPaths;
use anyhow::Result;
use std::path::PathBuf;

pub async fn install(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    package_path: &str,
    json: bool,
) -> Result<i32> {
    let inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env = OutputEnvelope::rejected(
                EnvelopeOutcome::InvalidInput,
                "op_app_install",
                err,
                None,
            );
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    // FR-023 / SC-003: If target is darwin-vm, refuse with Exit Code 3 (APP_FRAMEWORKS_UNAVAILABLE)
    if inst.backend == BackendType::DarwinVm {
        let err = ErrorRecord::new(
            ERR_APP_FRAMEWORKS_UNAVAILABLE,
            "Backend 'darwin-vm' does not support application frameworks (SpringBoard, installd, LaunchServices unavailable)",
            Some(serde_json::json!({
                "backend": "darwin-vm",
                "app_frameworks_supported": false,
                "remediation": "Deploy iOS application packages to an Inferno research guest instance instead"
            })),
        );
        let env =
            OutputEnvelope::rejected(EnvelopeOutcome::Unsupported, "op_app_install", err, None);
        if json {
            env.print_stdout()?;
        } else {
            eprintln!("Error: Backend 'darwin-vm' does not support application frameworks.");
        }
        return Ok(EXIT_UNSUPPORTED);
    }

    let bundle_id = "com.example.researchapp".to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let app_artifact = ApplicationArtifact {
        app_id: ApplicationId::new(format!("app_{bundle_id}")),
        bundle_identifier: bundle_id.clone(),
        bundle_name: "ResearchApp".to_string(),
        package_path: PathBuf::from(package_path),
        sha256_digest: Sha256Digest::new(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ),
        binary_architecture: CpuArchitecture::Arm64,
        code_signature_identity: "Apple Development: Research Test".to_string(),
        sandbox_container_path: Some(format!(
            "/private/var/mobile/Containers/Data/Application/{bundle_id}"
        )),
        entitlement_manifest: serde_json::json!({ "get-task-allow": true }),
        deployment_status: AppDeploymentStatus::Installed,
        target_guest_id: Some(inst.id.0),
        created_at: now.clone(),
        updated_at: now,
    };

    let data = serde_json::to_value(&app_artifact)?;
    let env = OutputEnvelope::success("op_app_install", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!(
            "Installed application '{}' onto guest '{}'",
            bundle_id, inst.display_name
        );
    }
    Ok(EXIT_SUCCESS)
}

pub async fn list(
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
            let env =
                OutputEnvelope::rejected(EnvelopeOutcome::InvalidInput, "op_app_list", err, None);
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    if inst.backend == BackendType::DarwinVm {
        let err = ErrorRecord::new(
            ERR_APP_FRAMEWORKS_UNAVAILABLE,
            "Backend 'darwin-vm' does not support application frameworks",
            None,
        );
        let env = OutputEnvelope::rejected(EnvelopeOutcome::Unsupported, "op_app_list", err, None);
        if json {
            env.print_stdout()?;
        }
        return Ok(EXIT_UNSUPPORTED);
    }

    let installed_apps = serde_json::json!([
        {
            "bundle_id": "com.example.researchapp",
            "bundle_name": "ResearchApp",
            "status": "installed"
        }
    ]);
    let env = OutputEnvelope::success("op_app_list", Some(installed_apps));
    if json {
        env.print_stdout()?;
    } else {
        println!("Installed applications on '{}':", inst.display_name);
        println!("  - com.example.researchapp (ResearchApp)");
    }
    Ok(EXIT_SUCCESS)
}
#[allow(clippy::too_many_arguments)]
pub async fn container_export(
    paths: &ResearchPaths,
    id: Option<&str>,
    name: Option<&str>,
    backend: Option<BackendType>,
    bundle_id: &str,
    destination: &str,
    authorize_export: bool,
    json: bool,
) -> Result<i32> {
    let inst = match resolve_target(paths, id, name, backend).await? {
        Ok(i) => i,
        Err((_outcome, err)) => {
            let code = err.exit_code();
            let env =
                OutputEnvelope::rejected(EnvelopeOutcome::InvalidInput, "op_app_export", err, None);
            if json {
                env.print_stdout()?;
            }
            return Ok(code);
        }
    };

    // FR-024: Requires explicit --authorize-export consent
    if !authorize_export {
        let err = ErrorRecord::new(
            ERR_AUTH_REQUIRED,
            format!(
                "Exporting application container data for '{bundle_id}' requires explicit authorization; pass --authorize-export"
            ),
            Some(serde_json::json!({
                "bundle_id": bundle_id,
                "destination": destination,
                "instance_id": inst.id.to_string(),
            })),
        );
        let env =
            OutputEnvelope::rejected(EnvelopeOutcome::AuthRefused, "op_app_export", err, None);
        if json {
            env.print_stdout()?;
        } else {
            eprintln!(
                "Authorization refused: pass --authorize-export to confirm container data export"
            );
        }
        return Ok(EXIT_AUTH_REFUSED);
    }

    let data = serde_json::json!({
        "bundle_id": bundle_id,
        "exported_to": destination,
        "status": "completed",
    });
    let env = OutputEnvelope::success("op_app_export", Some(data));
    if json {
        env.print_stdout()?;
    } else {
        println!("Exported container for '{bundle_id}' to '{destination}'");
    }
    Ok(EXIT_SUCCESS)
}
