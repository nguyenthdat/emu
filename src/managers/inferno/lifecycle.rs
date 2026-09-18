//! Lifecycle operations (start, stop, delete, wipe) for Inferno iOS research guests.

use crate::models::research::InstanceLifecycleState;
use crate::persistence::instances::{delete_instance, load_instance};
use crate::persistence::paths::ResearchPaths;
use crate::services::research::runtime::SupervisorClient;
use crate::utils::command::CommandRunner;
use crate::utils::command_process::{CommandSpec, ProcessGroupPolicy, StdioPolicy};
use anyhow::{Context, Result, bail};
use std::time::Duration;

pub async fn start_inferno_instance(paths: &ResearchPaths, id: &str) -> Result<()> {
    let inst = load_instance(paths, id)
        .await?
        .with_context(|| format!("Inferno guest '{id}' not found"))?;

    if inst.lifecycle_state == InstanceLifecycleState::Running {
        return Ok(());
    }

    let exe = std::env::current_exe().context("Failed to get current executable")?;
    let spec = CommandSpec::new(exe)
        .arg("__supervise")
        .arg("--vm-id")
        .arg(id)
        .process_group(ProcessGroupPolicy::NewProcessGroup)
        .stdout(StdioPolicy::Null)
        .stderr(StdioPolicy::Null);

    let runner = CommandRunner::new();
    let mut supervisor_handle = runner.spawn_typed(&spec).await?;

    // Poll for supervisor readiness via SupervisorClient
    let start_time = std::time::Instant::now();
    let mut ready = false;

    while start_time.elapsed() < Duration::from_secs(5) {
        if let Some(loaded) = load_instance(paths, id).await? {
            if loaded.lifecycle_state == InstanceLifecycleState::Running {
                let client = SupervisorClient::from_instance(&loaded);
                if client.query_runstate().await.is_ok() {
                    ready = true;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    if !ready {
        let _ = supervisor_handle
            .shutdown_and_reap(Duration::from_millis(500))
            .await;
        bail!("Supervisor failed to achieve readiness for Inferno guest '{id}' within deadline");
    }

    // Explicitly detach supervisor handle: supervisor persists across frontend exit
    supervisor_handle.detach()?;
    Ok(())
}

pub async fn stop_inferno_instance(paths: &ResearchPaths, id: &str) -> Result<()> {
    let inst = match load_instance(paths, id).await? {
        Some(i) => i,
        None => return Ok(()),
    };

    if inst.lifecycle_state == InstanceLifecycleState::Stopped {
        return Ok(());
    }

    let client = SupervisorClient::from_instance(&inst);
    let _ = client.shutdown(false).await;

    // Await run lock release up to 5 seconds
    let lock_path = paths.instance_run_lock_path(id)?;
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Ok(lock) = crate::persistence::lock::AdvisoryLock::try_acquire(&lock_path).await {
            drop(lock);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    Ok(())
}

pub async fn delete_inferno_instance(paths: &ResearchPaths, id: &str) -> Result<()> {
    let _ = stop_inferno_instance(paths, id).await;
    delete_instance(paths, id).await?;
    Ok(())
}

pub async fn wipe_inferno_instance(paths: &ResearchPaths, id: &str) -> Result<()> {
    stop_inferno_instance(paths, id).await?;
    let mut inst = load_instance(paths, id)
        .await?
        .with_context(|| format!("Inferno guest '{id}' not found"))?;
    inst.observed_privilege = crate::models::research::RootVerificationState::Unverified;
    inst.updated_at = chrono::Utc::now().to_rfc3339();
    crate::persistence::instances::save_instance(paths, &inst).await?;
    Ok(())
}
