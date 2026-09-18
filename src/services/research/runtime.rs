//! Runtime execution descriptor, supervisor RPC protocol, and backend launcher configuration.
//!
//! Defined in accordance with plan.md §Principle I, D-01, D-04, and contracts/cli.md §6.

use crate::models::research::{BackendType, ResearchGuestId, ResearchGuestInstance};
use crate::persistence::paths::ResearchPaths;
use crate::utils::command_process::{CommandSpec, ProcessGroupPolicy, StdioPolicy};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Runtime descriptor persisted to `runtime.json` in the guest's short temporary directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDescriptor {
    pub instance_id: ResearchGuestId,
    pub supervisor_pid: u32,
    pub hypervisor_pid: Option<u32>,
    pub runtime_dir: PathBuf,
    pub supervisor_socket: PathBuf,
    pub qmp_socket: PathBuf,
    pub console_socket: PathBuf,
    pub gdb_socket: PathBuf,
    pub backend: BackendType,
    pub started_at: String,
}

/// Commands accepted by the supervisor Unix domain socket RPC server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SupervisorAction {
    QueryRunstate,
    Shutdown { force: bool },
    ExecuteConsole { command: String, timeout_ms: u64 },
    AcquireDebugLease { client_identity: String },
    ReleaseDebugLease { lease_id: String },
    QueryPrivilege,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupervisorRequest {
    pub seq: u64,
    #[serde(flatten)]
    pub action: SupervisorAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupervisorResponse {
    pub seq: u64,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Backend launch configuration resolver and command-line argument builder.
pub struct BackendLaunchConfig;

impl BackendLaunchConfig {
    /// Resolves the dedicated patched hypervisor executable binary for the target backend.
    ///
    /// FR-001 / Gate G-01: Generic upstream Homebrew QEMU does NOT satisfy custom Apple Silicon
    /// SPTM or darwin boot structures. A dedicated patched executable is required.
    pub fn resolve_hypervisor_binary(backend: BackendType) -> Result<PathBuf> {
        let env_var = match backend {
            BackendType::DarwinVm => "EMU_DARWIN_VM_QEMU",
            BackendType::Inferno => "EMU_INFERNO_QEMU",
        };

        if let Ok(path_str) = std::env::var(env_var) {
            let p = PathBuf::from(&path_str);
            if p.exists() {
                return Ok(p);
            }
            bail!("Environment variable {env_var} points to non-existent binary '{path_str}'");
        }

        // Check local default research build directories
        let local_candidates = match backend {
            BackendType::DarwinVm => vec![
                PathBuf::from("/opt/emu/bin/darwin-vm-qemu"),
                dirs::home_dir()
                    .map(|h| h.join("Projects/darwin-vm/qemu-sptm/build/qemu-system-aarch64"))
                    .unwrap_or_default(),
            ],
            BackendType::Inferno => vec![
                PathBuf::from("/opt/emu/bin/inferno-qemu"),
                dirs::home_dir()
                    .map(|h| h.join("Projects/Inferno/build/qemu-system-aarch64"))
                    .unwrap_or_default(),
            ],
        };

        for candidate in local_candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Allow test execution or fallback if explicitly configured
        if std::env::var("EMU_ALLOW_MOCK_HYPERVISOR").is_ok() {
            if let Ok(exe) = std::env::current_exe() {
                return Ok(exe);
            }
        }

        bail!(
            "Missing required custom hypervisor binary for backend '{backend}'. \
             Set {env_var} to the validated executable path or build upstream from source."
        )
    }

    /// Constructs the CommandSpec for launching the hypervisor child process.
    pub fn build_hypervisor_command(
        paths: &ResearchPaths,
        inst: &ResearchGuestInstance,
        runtime_dir: &Path,
    ) -> Result<CommandSpec> {
        let binary = Self::resolve_hypervisor_binary(inst.backend)?;
        let qmp_sock = runtime_dir.join("qmp.sock");
        let gdb_sock = runtime_dir.join("gdb.sock");
        let console_sock = runtime_dir.join("console.sock");

        let mut spec = CommandSpec::new(binary)
            .process_group(ProcessGroupPolicy::NewProcessGroup)
            .cwd(runtime_dir.to_path_buf())
            .stdout(StdioPolicy::Null)
            .stderr(StdioPolicy::Capture);

        match inst.backend {
            BackendType::DarwinVm => {
                let kc_path = paths.artifact_path(&inst.boot_artifacts.kernelcache.0)?;
                let dt_path = paths.artifact_path(&inst.boot_artifacts.devicetree.0)?;

                spec = spec
                    .arg("-M")
                    .arg("darwin")
                    .arg("-bootkc")
                    .arg(kc_path.to_string_lossy().to_string())
                    .arg("-dtree")
                    .arg(dt_path.to_string_lossy().to_string())
                    .arg("-nographic")
                    .arg("-qmp")
                    .arg(format!("unix:{},server=on,wait=off", qmp_sock.display()))
                    .arg("-chardev")
                    .arg(format!(
                        "socket,id=gdb,path={},server=on,wait=off",
                        gdb_sock.display()
                    ))
                    .arg("-chardev")
                    .arg(format!(
                        "socket,id=console,path={},server=on,wait=off",
                        console_sock.display()
                    ))
                    .arg("-serial")
                    .arg("chardev:console");
            }
            BackendType::Inferno => {
                let kc_path = paths.artifact_path(&inst.boot_artifacts.kernelcache.0)?;
                let dt_path = paths.artifact_path(&inst.boot_artifacts.devicetree.0)?;

                spec = spec
                    .arg("-M")
                    .arg("t8030")
                    .arg("-kernel")
                    .arg(kc_path.to_string_lossy().to_string())
                    .arg("-dtb")
                    .arg(dt_path.to_string_lossy().to_string())
                    .arg("-nographic")
                    .arg("-qmp")
                    .arg(format!("unix:{},server=on,wait=off", qmp_sock.display()))
                    .arg("-chardev")
                    .arg(format!(
                        "socket,id=gdb,path={},server=on,wait=off",
                        gdb_sock.display()
                    ))
                    .arg("-chardev")
                    .arg(format!(
                        "socket,id=console,path={},server=on,wait=off",
                        console_sock.display()
                    ))
                    .arg("-serial")
                    .arg("chardev:console");
            }
        }

        Ok(spec)
    }
}

/// Client IPC wrapper communicating with the active guest supervisor via `supervisor.sock`.
pub struct SupervisorClient {
    socket_path: PathBuf,
}

impl SupervisorClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    pub fn from_instance(inst: &ResearchGuestInstance) -> Self {
        Self::new(inst.runtime_dir.join("supervisor.sock"))
    }

    /// Connects to `supervisor.sock` and executes an RPC round-trip.
    pub async fn request(&self, action: SupervisorAction) -> Result<serde_json::Value> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to connect to supervisor at '{}'",
                    self.socket_path.display()
                )
            })?;

        let (rx, mut tx) = stream.into_split();
        let mut reader = BufReader::new(rx);

        let req = SupervisorRequest { seq: 1, action };
        let mut line = serde_json::to_vec(&req)?;
        line.push(b'\n');

        tx.write_all(&line).await?;
        tx.flush().await?;

        let mut resp_str = String::new();
        reader.read_line(&mut resp_str).await?;

        let resp: SupervisorResponse = serde_json::from_str(&resp_str)
            .with_context(|| format!("Failed to parse supervisor response: {resp_str}"))?;

        if let Some(err) = resp.error {
            bail!("Supervisor returned error: {err}");
        }

        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }

    pub async fn query_runstate(&self) -> Result<String> {
        let val = self.request(SupervisorAction::QueryRunstate).await?;
        Ok(val
            .get("runstate")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown")
            .to_string())
    }

    pub async fn shutdown(&self, force: bool) -> Result<()> {
        let _ = self.request(SupervisorAction::Shutdown { force }).await?;
        Ok(())
    }

    pub async fn execute_console(&self, command: &str, timeout_ms: u64) -> Result<String> {
        let val = self
            .request(SupervisorAction::ExecuteConsole {
                command: command.to_string(),
                timeout_ms,
            })
            .await?;
        Ok(val
            .get("output")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string())
    }
}
