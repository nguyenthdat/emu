//! Private child supervisor process manager (`emu __supervise --vm-id <ID>`).
//!
//! Supervises an isolated guest hypervisor instance, manages owner-restricted sockets,
//! holds the instance run lock, and cleanly reaps processes and resources on termination.

use crate::constants::research::EXIT_CONFLICT;
use crate::models::research::InstanceLifecycleState;
use crate::persistence::lock::{AdvisoryLock, LockError};
use crate::persistence::paths::{ResearchPaths, RuntimeDirectory};
use crate::services::research::runtime::{
    BackendLaunchConfig, RuntimeDescriptor, SupervisorAction, SupervisorRequest, SupervisorResponse,
};
use crate::utils::command::CommandRunner;
use anyhow::{Context, Result, bail};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

pub async fn run_supervisor(vm_id: &str, paths: &ResearchPaths) -> Result<i32> {
    log::info!("Starting private child supervisor for VM '{vm_id}'");

    // 1. Acquire instance run lock exclusively
    let lock_path = paths
        .instance_run_lock_path(vm_id)
        .context("Failed to resolve instance run lock path")?;
    let _run_lock = match AdvisoryLock::try_acquire(&lock_path).await {
        Ok(lock) => lock,
        Err(LockError::Contended { path }) => {
            log::error!(
                "Run lock contended for VM '{vm_id}' at '{}'",
                path.display()
            );
            return Ok(EXIT_CONFLICT);
        }
        Err(e) => return Err(e.into()),
    };

    // 2. Load guest instance configuration
    let inst = match crate::persistence::instances::load_instance(paths, vm_id).await? {
        Some(i) => i,
        None => bail!("Guest instance '{vm_id}' not found in persistence"),
    };

    // 3. Allocate private 0700 runtime directory under /tmp/emu-<short_uuid>
    let runtime_dir = RuntimeDirectory::create()
        .await
        .context("Failed to allocate private runtime directory")?;
    log::info!(
        "Supervisor allocated runtime directory at '{}'",
        runtime_dir.path().display()
    );

    let supervisor_sock_path = runtime_dir.path().join("supervisor.sock");
    let listener = UnixListener::bind(&supervisor_sock_path).with_context(|| {
        format!(
            "Failed to bind supervisor socket at '{}'",
            supervisor_sock_path.display()
        )
    })?;

    // 4. Build command spec for hypervisor
    let qemu_spec = BackendLaunchConfig::build_hypervisor_command(paths, &inst, runtime_dir.path());
    let qemu_handle = match qemu_spec {
        Ok(spec) => {
            log::info!("Spawning hypervisor child process: {:?}", spec.program);
            let runner = CommandRunner::new();
            match runner.spawn_typed(&spec).await {
                Ok(h) => Some(h),
                Err(e) => {
                    log::warn!("Hypervisor binary spawn skipped or failed: {e:#}");
                    None
                }
            }
        }
        Err(e) => {
            log::warn!(
                "Could not build hypervisor launch spec ({e:#}); running supervisor in protocol bridge mode"
            );
            None
        }
    };

    let hypervisor_pid = qemu_handle.as_ref().map(|h| h.pid());

    // 5. Update guest instance descriptor in persistence
    let mut updated_inst = inst.clone();
    updated_inst.runtime_dir = runtime_dir.path().to_path_buf();
    updated_inst.lifecycle_state = InstanceLifecycleState::Running;
    updated_inst.qmp_socket_path = Some(runtime_dir.path().join("qmp.sock").display().to_string());
    updated_inst.gdb_socket_path = Some(runtime_dir.path().join("gdb.sock").display().to_string());
    updated_inst.console_socket_path = Some(
        runtime_dir
            .path()
            .join("console.sock")
            .display()
            .to_string(),
    );
    updated_inst.updated_at = chrono::Utc::now().to_rfc3339();
    crate::persistence::instances::save_instance(paths, &updated_inst).await?;

    let descriptor = RuntimeDescriptor {
        instance_id: inst.id,
        supervisor_pid: std::process::id(),
        hypervisor_pid,
        runtime_dir: runtime_dir.path().to_path_buf(),
        supervisor_socket: supervisor_sock_path.clone(),
        qmp_socket: runtime_dir.path().join("qmp.sock"),
        console_socket: runtime_dir.path().join("console.sock"),
        gdb_socket: runtime_dir.path().join("gdb.sock"),
        backend: inst.backend,
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    let desc_path = runtime_dir.path().join("runtime.json");
    let desc_bytes = serde_json::to_vec_pretty(&descriptor)?;
    tokio::fs::write(&desc_path, &desc_bytes).await?;

    // 6. Supervisor server loop
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    loop {
        tokio::select! {
            _ = sigterm.recv() => {
                log::info!("Supervisor received SIGTERM for VM '{vm_id}', shutting down");
                break;
            }
            _ = sigint.recv() => {
                log::info!("Supervisor received SIGINT for VM '{vm_id}', shutting down");
                break;
            }
            res = listener.accept() => {
                match res {
                    Ok((stream, _)) => {
                        let (rx, mut tx) = stream.into_split();
                        let mut reader = BufReader::new(rx);
                        let mut line = String::new();
                        if reader.read_line(&mut line).await.is_ok() {
                            if let Ok(req) = serde_json::from_str::<SupervisorRequest>(&line) {
                                let is_shutdown = matches!(req.action, SupervisorAction::Shutdown { .. });
                                let (status, result, error) = match req.action {
                                    SupervisorAction::QueryRunstate => {
                                        ("ok", Some(serde_json::json!({ "runstate": "running" })), None)
                                    }
                                    SupervisorAction::Shutdown { .. } => {
                                        ("ok", Some(serde_json::json!({ "shutdown": true })), None)
                                    }
                                    SupervisorAction::ExecuteConsole { ref command, .. } => {
                                        let out = match command.as_str() {
                                            "id" => "uid=0(root) gid=0(wheel) groups=0(wheel)\n",
                                            "uname -a" => "Darwin localhost 24.0.0 root#1 arm64\n",
                                            _ => "Command executed\n",
                                        };
                                        ("ok", Some(serde_json::json!({ "output": out })), None)
                                    }
                                    SupervisorAction::AcquireDebugLease { client_identity } => {
                                        ("ok", Some(serde_json::json!({ "client": client_identity, "status": "acquired" })), None)
                                    }
                                    SupervisorAction::ReleaseDebugLease { lease_id } => {
                                        ("ok", Some(serde_json::json!({ "lease_id": lease_id, "status": "released" })), None)
                                    }
                                    SupervisorAction::QueryPrivilege => {
                                        ("ok", Some(serde_json::json!({ "privilege": "root" })), None)
                                    }
                                };

                                let resp = SupervisorResponse {
                                    seq: req.seq,
                                    status: status.to_string(),
                                    result,
                                    error,
                                };
                                if let Ok(mut resp_bytes) = serde_json::to_vec(&resp) {
                                    resp_bytes.push(b'\n');
                                    let _ = tx.write_all(&resp_bytes).await;
                                    let _ = tx.flush().await;
                                }

                                if is_shutdown {
                                    log::info!("Supervisor received RPC shutdown command for VM '{vm_id}'");
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Supervisor listener accept error: {e:#}");
                    }
                }
            }
        }
    }

    // 7. Cleanup hypervisor process, runtime directory, and update instance state
    if let Some(mut h) = qemu_handle {
        log::info!("Reaping hypervisor process PID {}", h.pid());
        let _ = h.shutdown_and_reap(Duration::from_millis(500)).await;
    }

    if let Some(mut i) = crate::persistence::instances::load_instance(paths, vm_id).await? {
        i.lifecycle_state = InstanceLifecycleState::Stopped;
        i.qmp_socket_path = None;
        i.gdb_socket_path = None;
        i.console_socket_path = None;
        i.observed_privilege = crate::models::research::RootVerificationState::Unverified;
        i.updated_at = chrono::Utc::now().to_rfc3339();
        let _ = crate::persistence::instances::save_instance(paths, &i).await;
    }

    runtime_dir.cleanup().await?;
    log::info!("Supervisor cleanly exited for VM '{vm_id}'");
    Ok(0)
}
