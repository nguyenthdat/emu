//! Command Line Interface automation and routing hierarchy.
pub mod app;
pub mod artifact;
pub mod backend;
pub mod baseline;
pub mod companion;
pub mod device;
pub mod envelope;
pub mod kernel;
pub mod operation;
pub mod profile;
pub mod record;
pub mod supervisor;
pub mod tool;
pub mod worker;

use crate::models::research::BackendType;
use crate::persistence::paths::ResearchPaths;
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Private child supervisor process manager
    #[command(name = "__supervise", hide = true)]
    Supervise {
        #[arg(long = "vm-id")]
        vm_id: String,
    },
    /// Private finite worker process manager
    #[command(name = "__worker", hide = true)]
    Worker {
        #[arg(long = "operation-id")]
        operation_id: String,
    },
    /// iOS and Darwin Security Research command hierarchy
    Research {
        #[command(subcommand)]
        family: ResearchFamily,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ResearchFamily {
    /// Backend capability discovery and preflight diagnostics
    Backend {
        #[command(subcommand)]
        action: BackendAction,
    },
    /// Guest instance lifecycle management
    Guest {
        #[command(subcommand)]
        action: GuestAction,
    },
    /// Operation tracking, waiting, and cancellation
    Operation {
        #[command(subcommand)]
        action: OperationAction,
    },
    /// Empirical root proof verification, status, and console
    Root {
        #[command(subcommand)]
        action: RootAction,
    },
    /// Owned iOS application lifecycle and container management
    App {
        #[command(subcommand)]
        action: AppAction,
    },
    /// Frida dynamic instrumentation runtime and hook inspection
    Frida {
        #[command(subcommand)]
        action: FridaAction,
    },
    /// Low-level kernel debugging under exclusive KernelDebugLease
    Debug {
        #[command(subcommand)]
        action: DebugAction,
    },
    /// Research experiment profile and security profile management
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// Image artifact preparation, verification, and registration
    Image {
        #[command(subcommand)]
        action: ImageAction,
    },
    /// Local helper Linux companion VM lifecycle and status
    Companion {
        #[command(subcommand)]
        action: CompanionAction,
    },
    /// Clean baseline reference snapshot management
    Baseline {
        #[command(subcommand)]
        action: BaselineAction,
    },
    /// Immutable trial experiment record inspection
    Record {
        #[command(subcommand)]
        action: RecordAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum BackendAction {
    /// Evaluate host platform and hypervisor capabilities
    Preflight {
        #[arg(long)]
        json: bool,
    },
    /// List supported research virtualization backends
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum GuestAction {
    /// Register a new research guest instance
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        backend: String,
        #[arg(long)]
        kernelcache: Option<String>,
        #[arg(long)]
        devicetree: Option<String>,
        #[arg(long)]
        root_disk: Option<String>,
        #[arg(long)]
        ramdisk: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List registered research guest instances
    List {
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Inspect details of a guest instance
    Inspect {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Boot a guest instance
    Start {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Gracefully stop a running guest instance
    Stop {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Restart a guest instance and invalidate active root proof
    Restart {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Permanently delete a guest instance with Two-Step Safety Gate
    Delete {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        authorize: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum OperationAction {
    /// Inspect status of an asynchronous operation
    Status {
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Cancel an in-flight operation at a safe transaction boundary
    Cancel {
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Await completion of an asynchronous operation
    Wait {
        #[arg(long)]
        id: String,
        #[arg(long)]
        timeout: Option<u64>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum RootAction {
    /// Execute empirical root proof verification workflow
    Verify {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long, hide = true)]
        simulate_unprivileged_leak: bool,
        #[arg(long)]
        json: bool,
    },
    /// Inspect desired vs observed privilege status
    Status {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Establish root launch console or execute non-interactive command
    Console {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum AppAction {
    /// Install an owned iOS application package onto an Inferno guest
    Install {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        package: String,
        #[arg(long)]
        json: bool,
    },
    /// List installed application bundles
    List {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Export application sandboxed container data
    ContainerExport {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        bundle_id: String,
        #[arg(long)]
        destination: String,
        #[arg(long)]
        authorize_export: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum FridaAction {
    /// Attach Frida 17.18.0 script to target process
    Attach {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        pid: Option<u32>,
        #[arg(long)]
        script: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Detach Frida probes cleanly preserving target process execution
    Detach {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum DebugAction {
    /// Pause virtual CPU execution under exclusive debug lease
    Pause {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Resume virtual CPU execution and release debug lease
    Resume {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Read general purpose processor registers
    Registers {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Disconnect debugger session with truthful runstate preservation
    Disconnect {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        action: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProfileAction {
    /// Apply a research or security profile to a guest instance
    Apply {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        authorize: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ImageAction {
    /// Register a research image artifact
    Register {
        #[arg(long)]
        file: String,
        #[arg(long = "type")]
        artifact_type: String,
        #[arg(long)]
        backend: String,
        #[arg(long)]
        allow_experimental: bool,
        #[arg(long)]
        json: bool,
    },
    /// Prepare host-side guest disk or ramdisk
    Prepare {
        #[arg(long)]
        source: String,
        #[arg(long = "target-backend")]
        target_backend: String,
        #[arg(long)]
        output: String,
        #[arg(long)]
        unattended: bool,
        #[arg(long)]
        json: bool,
    },
    /// Verify guest disk mount point safety
    VerifyMount {
        #[arg(long = "mount-path")]
        mount_path: String,
        #[arg(long)]
        json: bool,
    },
    /// Clean up disposable loopback attachments
    Cleanup {
        #[arg(long = "operation-id")]
        operation_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List registered image artifacts
    List {
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum CompanionAction {
    /// Start isolated local companion Linux VM
    Start {
        #[arg(long = "parent-guest-id")]
        parent_guest_id: String,
        #[arg(long)]
        cpus: Option<u32>,
        #[arg(long = "memory-mb")]
        memory_mb: Option<u64>,
        #[arg(long)]
        json: bool,
    },
    /// Stop companion VM with dependency check
    Stop {
        #[arg(long = "parent-guest-id")]
        parent_guest_id: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// Query companion environment status
    Status {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum BaselineAction {
    /// Capture clean baseline reference snapshot
    Create {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long = "deadline-ms", default_value = "15000")]
        deadline_ms: u64,
        #[arg(long)]
        json: bool,
    },
    /// Restore guest to clean recovery baseline snapshot
    Restore {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        authorize: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum RecordAction {
    /// Inspect an immutable trial experiment record
    Inspect {
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// List historical trial experiment records
    List {
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

fn parse_backend(s: &str) -> Option<BackendType> {
    match s.to_lowercase().as_str() {
        "darwin-vm" | "darwin" => Some(BackendType::DarwinVm),
        "inferno" => Some(BackendType::Inferno),
        _ => None,
    }
}

pub async fn run_command(cmd: Commands) -> Result<i32> {
    let paths = ResearchPaths::platform()?;
    paths.ensure().await?;

    match cmd {
        Commands::Supervise { vm_id } => supervisor::run_supervisor(&vm_id, &paths).await,
        Commands::Worker { operation_id } => worker::run_worker(&operation_id, &paths).await,
        Commands::Research { family } => match family {
            ResearchFamily::Backend { action } => match action {
                BackendAction::Preflight { json } => backend::preflight(&paths, json).await,
                BackendAction::List { json } => backend::list(&paths, json).await,
            },
            ResearchFamily::Guest { action } => match action {
                GuestAction::Create {
                    name,
                    backend,
                    kernelcache,
                    devicetree,
                    root_disk,
                    ramdisk,
                    json,
                } => {
                    let b = parse_backend(&backend).unwrap_or(BackendType::DarwinVm);
                    device::create(
                        &paths,
                        &name,
                        b,
                        kernelcache,
                        devicetree,
                        root_disk,
                        ramdisk,
                        json,
                    )
                    .await
                }
                GuestAction::List { backend, json } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    device::list(&paths, b, json).await
                }
                GuestAction::Inspect {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    device::inspect(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                GuestAction::Start {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    device::start(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                GuestAction::Stop {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    device::stop(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                GuestAction::Restart {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    device::restart(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                GuestAction::Delete {
                    id,
                    name,
                    backend,
                    authorize,
                    dry_run,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    device::delete(
                        &paths,
                        id.as_deref(),
                        name.as_deref(),
                        b,
                        authorize.as_deref(),
                        dry_run,
                        json,
                    )
                    .await
                }
            },
            ResearchFamily::Operation { action } => match action {
                OperationAction::Status { id, json } => operation::status(&paths, &id, json).await,
                OperationAction::Cancel { id, json } => operation::cancel(&paths, &id, json).await,
                OperationAction::Wait { id, timeout, json } => {
                    operation::wait(&paths, &id, timeout, json).await
                }
            },
            ResearchFamily::Root { action } => match action {
                RootAction::Verify {
                    id,
                    name,
                    backend,
                    simulate_unprivileged_leak,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    tool::verify(
                        &paths,
                        id.as_deref(),
                        name.as_deref(),
                        b,
                        simulate_unprivileged_leak,
                        json,
                    )
                    .await
                }
                RootAction::Status {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    tool::status(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                RootAction::Console {
                    id,
                    name,
                    backend,
                    command,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    tool::console(
                        &paths,
                        id.as_deref(),
                        name.as_deref(),
                        b,
                        command.as_deref(),
                        json,
                    )
                    .await
                }
            },
            ResearchFamily::App { action } => match action {
                AppAction::Install {
                    id,
                    name,
                    backend,
                    package,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    app::install(&paths, id.as_deref(), name.as_deref(), b, &package, json).await
                }
                AppAction::List {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    app::list(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                AppAction::ContainerExport {
                    id,
                    name,
                    backend,
                    bundle_id,
                    destination,
                    authorize_export,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    app::container_export(
                        &paths,
                        id.as_deref(),
                        name.as_deref(),
                        b,
                        &bundle_id,
                        &destination,
                        authorize_export,
                        json,
                    )
                    .await
                }
            },
            ResearchFamily::Frida { action } => match action {
                FridaAction::Attach {
                    id,
                    name,
                    backend,
                    pid,
                    script,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    tool::frida_attach(
                        &paths,
                        id.as_deref(),
                        name.as_deref(),
                        b,
                        pid,
                        script.as_deref(),
                        json,
                    )
                    .await
                }
                FridaAction::Detach {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    tool::frida_detach(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
            },
            ResearchFamily::Debug { action } => match action {
                DebugAction::Pause {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    kernel::pause(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                DebugAction::Resume {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    kernel::resume(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                DebugAction::Registers {
                    id,
                    name,
                    backend,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    kernel::registers(&paths, id.as_deref(), name.as_deref(), b, json).await
                }
                DebugAction::Disconnect {
                    id,
                    name,
                    backend,
                    action,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    kernel::disconnect(
                        &paths,
                        id.as_deref(),
                        name.as_deref(),
                        b,
                        action.as_deref(),
                        json,
                    )
                    .await
                }
            },
            ResearchFamily::Profile { action } => match action {
                ProfileAction::Apply {
                    id,
                    name,
                    backend,
                    profile,
                    authorize,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    profile::apply(
                        &paths,
                        id.as_deref(),
                        name.as_deref(),
                        b,
                        &profile,
                        authorize.as_deref(),
                        json,
                    )
                    .await
                }
            },
            ResearchFamily::Image { action } => match action {
                ImageAction::Register {
                    file,
                    artifact_type,
                    backend,
                    allow_experimental,
                    json,
                } => {
                    let b = parse_backend(&backend).unwrap_or(BackendType::DarwinVm);
                    artifact::register(&paths, &file, &artifact_type, b, allow_experimental, json)
                        .await
                }
                ImageAction::Prepare {
                    source,
                    target_backend,
                    output,
                    unattended,
                    json,
                } => {
                    let b = parse_backend(&target_backend).unwrap_or(BackendType::DarwinVm);
                    artifact::prepare(&paths, &source, b, &output, unattended, json).await
                }
                ImageAction::VerifyMount { mount_path, json } => {
                    artifact::verify_mount(&mount_path, json).await
                }
                ImageAction::Cleanup {
                    operation_id: _,
                    json,
                } => {
                    let env = envelope::OutputEnvelope::success(
                        "op_image_cleanup",
                        Some(serde_json::json!({ "cleaned": true })),
                    );
                    if json {
                        env.print_stdout()?;
                    }
                    Ok(crate::constants::research::EXIT_SUCCESS)
                }
                ImageAction::List { backend, json } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    artifact::list(&paths, b, json).await
                }
            },
            ResearchFamily::Companion { action } => match action {
                CompanionAction::Start {
                    parent_guest_id,
                    cpus,
                    memory_mb,
                    json,
                } => companion::start(&paths, &parent_guest_id, cpus, memory_mb, json).await,
                CompanionAction::Stop {
                    parent_guest_id,
                    force,
                    json,
                } => companion::stop(&paths, &parent_guest_id, force, json).await,
                CompanionAction::Status { json } => companion::status(&paths, json).await,
            },
            ResearchFamily::Baseline { action } => match action {
                BaselineAction::Create {
                    id,
                    name,
                    backend,
                    deadline_ms,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    baseline::create(&paths, id.as_deref(), name.as_deref(), b, deadline_ms, json)
                        .await
                }
                BaselineAction::Restore {
                    id,
                    name,
                    backend,
                    authorize,
                    dry_run,
                    json,
                } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    baseline::restore(
                        &paths,
                        id.as_deref(),
                        name.as_deref(),
                        b,
                        authorize.as_deref(),
                        dry_run,
                        json,
                    )
                    .await
                }
            },
            ResearchFamily::Record { action } => match action {
                RecordAction::Inspect { id, json } => record::inspect(&paths, &id, json).await,
                RecordAction::List { backend, json } => {
                    let b = backend.as_deref().and_then(parse_backend);
                    record::list(&paths, b, json).await
                }
            },
        },
    }
}
