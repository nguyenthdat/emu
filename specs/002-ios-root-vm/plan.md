# Implementation Plan: iOS Root VM and Darwin Security Research Backends

**Branch**: `002-ios-root-vm` | **Date**: 2026-09-18 | **Spec**: [spec.md](spec.md) | **Research**: [research.md](research.md)  
**Input**: Feature specification from `specs/002-ios-root-vm/spec.md` and consolidated research from `specs/002-ios-root-vm/research.md`  
**Public Grammar Authority**: `specs/002-ios-root-vm/contracts/cli.md`  
**Canonical Schema Identifier**: `https://emu.rs/schemas/v1/research-ios.schema.json`

---

## Summary

The iOS Root VM and Darwin Security Research Backends feature introduces dedicated low-level Darwin and virtualized iOS security research capabilities to Emu on macOS Apple Silicon (`aarch64`). The architecture incorporates a complementary dual-backend virtualization portfolio:

1. **`darwin-vm`** (specialized in minimal headless Darwin virtual machines, kernel bootstrap, benign guest CLI binaries, and low-level kernel debugging; userland application frameworks are explicitly reported as unavailable);
2. **`Inferno`** (specialized in complete iOS research environments based on QEMU-SPTM, supporting userland SpringBoard, owned compatible application lifecycle management via `InstallationProxy` and `ideviceinstaller`, and dynamic instrumentation).

The technical approach enforces strict process isolation and rejects in-process QEMU C/Rust FFI bindings (due to hypervisor crash containment, global mutable state, signal handler interception, and `CFRunLoop` ownership in upstream `Inferno/system/main.c:44-96`, and process-level modularity pending formal legal review). Hypervisor backends are managed strictly as isolated child processes communicating via out-of-process protocols: line-delimited QEMU Machine Protocol (QMP) over owner-restricted Unix Domain Sockets for deterministic lifecycle control, and a typed GDB Remote Serial Protocol (RSP) client communicating over chardev sockets under an exclusive `KernelDebugLease` for register, memory, and single-step kernel debugging (initiated strictly upon first explicit debug entry to prevent premature upstream `vm_stop()` on boot, and retained throughout the remaining guest lifecycle).

Dynamic instrumentation is decoupled into an explicitly separate native child executable (`emu-frida-worker`) provisioned from a standalone private Cargo package (`tools/frida-worker/`) outside the root workspace/default target graph, linked against the official `frida-core` 17.18.0 C devkit API, communicating with the supervisor over typed stdio JSON streams. This isolates GLib (`GMainContext`, `GMainLoop`) event loops from Tokio and Ratatui runtimes, enforces `DeviceManager.with_socket_backend_only()` to eliminate Mach task-port injection backends, and binds exclusively to an owned guest bridge over owner-restricted Unix Domain Sockets with pinned Frida TLS server certificate and per-session client token over an owned authenticated SSH bridge with pinned SSH host key (server-auth TLS + token client auth; client certificate authentication is not used). Scripts incorporate pre-bundled `frida-objc-bridge` runtime preambles and modern `Process` module APIs (`Process.getModuleByName().getExportByName()`, synchronous `Process.enumerateModules()`).

Specialized restore workflows, ramdisk preparation, and USB-over-IP bridging for `Inferno` are managed via the documented custom `qemu-system-x86_64` companion virtual machine built from Inferno running under TCG on the same macOS host (`https://chefkiss.dev/guides/inferno/companion-setup/`), using Unix Domain Socket USB transport (`usb-tcp-remote`). The speculative ARM64/HVF companion candidate is excluded. The companion socket listener binds before the Inferno guest connects, and companion lifetimes are bound strictly to authoritative live reference sets of active restore workflows and live dependent guest sessions (FR-038).

The system adheres to a single public binary architecture (`emu`) containing both interactive TUI and headless CLI automation, plus private child supervisor (`emu __supervise --vm-id <ID>`) and worker (`emu __worker --operation-id <ID>`) modes, complemented by the separate native helper binary (`emu-frida-worker`). No persistent host daemons (`launchd`) are deployed. Transactional persistence and cross-process advisory locking (`fs4`) are rooted under `dirs::data_local_dir()/emu/research` (`~/Library/Application Support/emu/research`), with Unix Domain Sockets dynamically allocated under private `/tmp/emu-<short_uuid>/` directories to strictly observe Darwin's 104-byte `sun_path` boundary.

Empirical root proof (UID 0) in iOS-derived guests is established via an owned dedicated benign fixture helper (`emu-root-probe`) executing inside the guest: verifying positive effective UID 0 write to `/private/var/root/.emu_probe` returning full payload byte count with readback verification, validating that privilege dropping to a declared non-zero UID/GID succeeds (`geteuid() != 0`), closing privileged descriptors, and confirming that unprivileged open/write access to the fixture is denied with `errno` strictly `EACCES` or `EPERM`. A failed privilege drop leaves status `unverified` (no new enum). Verified privilege states automatically invalidate upon reboot or configuration change. Standard Android AVD and macOS iOS Simulator workflows maintain 100% behavioral parity and zero regression (SC-008), with all eight Android 001 specification artifacts preserved byte-for-byte.

---

## Technical Context

### Language, Toolchain & Compiler Strictness

- **Language / Version**: Rust 2024 edition, pinned strictly to `rustc 1.88.0` (declared in `.tool-versions`). `bun 1.2.17` is pinned for TypeScript tooling.
- **Compiler Flags & Diagnostics**: The codebase adheres to strict compiler diagnostics (`-D warnings`) and clippy enforcement (`cargo clippy --all-targets --all-features -- -D warnings`). Zero linter suppressions or unprincipled compiler warning allowances are introduced.

### Primary Dependencies & Manifest Contract

- **Existing Locked Dependencies (`Cargo.toml`, `Cargo.lock`)**:
  - Runtime: `tokio = { version = "1.53.1", features = ["full"] }`, `clap = { version = "4.6.7", features = ["derive", "env"] }`, `ratatui = "0.30"`, `crossterm = "0.29"`, `serde = { version = "1.0.229", features = ["derive"] }`, `serde_json = "1.0.151"`, `anyhow = "1.0"`, `thiserror = "2.0"`, `dirs = "7.0.0"`, `regex = "1.13"`, `chrono = { version = "0.4", features = ["serde"] }`, `env_logger = "0.11"`, `log = "0.4"`, `which = "8.0"`.
  - Dev/Test: `assert_cmd = "2.2"`, `predicates = "3.1"`, `tempfile = "3.27"`, `mockall = "0.15"`, `criterion = "0.8"`, `futures = "0.3"`, `rand = "0.10"`.
- **Proposed Dependency Additions / Promotions**:
  - `fs4 = "1.1.0"` (features: `["sync"]`, `default-features = false`) for cross-process advisory file locking via `fs4::FileExt`. Required because `fs4` supports MSRV 1.75+ (compatible with Rust 1.88.0), whereas `std::fs::File::try_lock` requires Rust 1.89+, which violates the repository's pinned `1.88.0` toolchain.
  - `sha2 = "0.10.9"` (promotion of existing locked transitive crate) for cryptographic artifact, profile manifest, proposal digest, and evidence verification.
  - `uuid = "1.26.1"` (promotion of existing locked transitive crate, features: `["v4"]`) for unique instance, boot session, operation, and proposal identifiers.
  - `nix = "0.29.0"` (promotion of existing locked transitive crate) for Unix domain socket permissions (`0700`), PTY allocation, and signal delivery.
- **Manifest Invariant**: No manifest edits are made during Phase 0 / Phase 1 design; dependency additions will be formally introduced in Phase 2 implementation.

### Existing Codebase Abstractions vs. Planned Research Extensions

- **Repository Layout**: The repository implements a single-crate layout with `src/` (not `crates/`).
- **`DeviceManager` Trait (`src/managers/common.rs:31-131`)**:
  - Currently defines: `list_devices`, `start_device`, `stop_device`, `create_device`, `delete_device`, `wipe_device`, `is_available`.
  - Uses RPITIT (`impl std::future::Future<Output = Result<...>> + Send`) and `DeviceConfig`.
  - **Invariant**: The legacy trait signatures remain completely unchanged because standard `AndroidManager` (`src/managers/android/`) and `IosManager` (`src/managers/ios/`) actively implement and consume them. The planned research managers (`DarwinVmManager`, `InfernoManager`) will implement `DeviceManager` alongside the legacy managers. Existing Android and iOS simulator code is functionally preserved with zero regressions (SC-008, FR-004), but is not byte-frozen; integration adjustments may occur. All eight Android 001 specification documentation artifacts (`specs/001-add-android-research-backends/`) remain strictly byte-for-byte immutable.
- **`CommandExecutor` Trait (`src/utils/command_executor.rs:15-37`)**:
  - Currently defines string-based `run(&self, command, args) -> Result<String>`, PID-based `spawn(&self, command, args) -> Result<u32>`, and legacy helpers `run_with_retry` and `run_ignoring_errors`.
  - **In-Place Upgrade & Concrete Process Boundary**: Rather than creating an independent ad-hoc subprocess boundary, `CommandExecutor` receives in-place typed execution methods:
    ```rust
    async fn run_typed(&self, spec: &CommandSpec) -> Result<CommandOutput>;
    async fn spawn_typed(&self, spec: &CommandSpec) -> Result<ProcessHandle>;
    ```
    implemented for both the production runner (`src/utils/command.rs`) and `MockCommandExecutor` (`src/utils/command_executor.rs`).
  - **`CommandSpec` Contract**:
    - `program`: `std::path::PathBuf` defining the target executable binary.
    - `args`: `Vec<String>` defining the ordered command-line arguments.
    - `cwd`: `Option<std::path::PathBuf>` defining explicit working directory (defaults to session working directory if unspecified).
    - `env`: `std::collections::BTreeMap<String, String>` defining explicit environment variables; host credentials and sensitive variables are stripped unless explicitly whitelisted.
    - `stdio`: `StdioPolicy` enum (`Null`, `Inherit`, `Piped`, `Capture`), where `stdin` defaults to `Null` (or `Piped` for PTY/repl feeding), and `stdout`/`stderr` default to `Capture` bounded by maximum buffer limits (e.g. 10MB) to prevent host memory exhaustion.
    - `timeout`: `Option<std::time::Duration>` defining caller execution deadline.
    - `cancellation`: `Option<tokio::sync::watch::Receiver<bool>>` defining cooperative cancellation using existing `tokio::sync::watch` primitives (without unapproved external crate dependencies).
  - **`CommandOutput` Contract**:
    - `status`: `std::process::ExitStatus` capturing process exit code or terminating Unix signal (via `nix` / `ExitStatusExt`).
    - `stdout`: `Vec<u8>` holding captured stdout bounded bytes.
    - `stderr`: `Vec<u8>` holding captured stderr bounded bytes.
    - Helper methods: `success(&self) -> bool`, `code(&self) -> Option<i32>`, `signal(&self) -> Option<i32>`, `stdout_str(&self) -> Result<&str>`, `stderr_str(&self) -> Result<&str>`.
  - **`ProcessHandle` Contract (Mockable Facade & Owner-Managed Reaping)**:
    - Implements a unified `ProcessHandle` facade over an internal `ProcessBackend` abstraction (supporting production OS processes via Tokio and synthetic in-memory test processes via `MockCommandExecutor`):
      ```rust
      pub struct ProcessHandle {
          pub pid: u32,
          pub stdin: Option<Box<dyn tokio::io::AsyncWrite + Send + Unpin>>,
          pub stdout: Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>>,
          pub stderr: Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>>,
          backend: Box<dyn ProcessBackend>,
      }
      ```
    - Streams use abstract asynchronous reader/writer traits (`AsyncRead` / `AsyncWrite`), allowing `MockCommandExecutor` to inject synthetic in-memory streams (e.g. `tokio::io::DuplexStream`) without creating OS pipes.
    - **Owner-Managed Async Reaping**:
      - `async fn wait(&mut self) -> Result<std::process::ExitStatus>`: awaits child process termination and reaps process status.
      - `async fn wait_timeout(&mut self, timeout: std::time::Duration) -> Result<Option<std::process::ExitStatus>>`: awaits child process termination up to the declared timeout without blocking or reaping prematurely.
      - `async fn shutdown_and_reap(&mut self, grace_period: std::time::Duration) -> Result<std::process::ExitStatus>`: issues graceful `SIGTERM`, awaits completion up to `grace_period`, falls back to `SIGKILL` verified against process start time and executable path, and reaps the child process.
    - **Drop Contract & Observer Timeout Non-Interference**:
      - Synchronous `Drop` cannot await asynchronous reaping. The supervisor explicitly retains and owns the `ProcessHandle` until async shutdown and reaping complete.
      - `Drop` serves solely as a non-blocking fallback that issues a best-effort `SIGTERM` signal and records any un-reaped residual resources to the operational error journal; it does not fabricate synchronous reaping success.
      - Observer CLI/TUI wait deadlines or disconnects (Exit Code 124) MUST NEVER drop or terminate the supervisor-owned `ProcessHandle`; the background guest process continues unaffected.
  - **`MockCommandExecutor` Contract**:
    - Implements identical `run_typed` and `spawn_typed` traits, returning configurable `CommandOutput` and synthetic `ProcessHandle` instances backed by `MockProcessBackend` with matching wait, stream, and cancellation semantics, allowing deterministic unit testing without spawning OS processes.
  - Legacy methods remain for existing Android/iOS consumers; research backends never invoke retry or error-ignoring helpers.
- **Headless CLI Execution Path vs. Legacy Check**:
  - In the current codebase, `src/main.rs:121-153` (`run_local_check`) calls `App::new().await` during `emu --check` to construct the application shell and discover devices without entering the interactive TUI event loop.
  - All new headless research CLI subcommands (`emu research ...`) introduce a dedicated headless routing branch in `src/main.rs` that completely bypasses `App::new()`, preventing raw terminal allocation, alternate screen buffer creation, mouse capture, or UI background thread polling. Legacy `--check` behavior is documented accurately rather than falsely claiming an existing bypass.

- **Native Frida Helper Build Isolation (`tools/frida-worker/`)**:
  - The root `emu` crate retains strictly typed IPC protocol definitions, stdio streaming transport handlers, and mock executors (`src/workers/frida/`).
  - The native helper executable `emu-frida-worker` resides in a dedicated standalone Cargo package under `tools/frida-worker/` outside the root workspace and default target graph.
  - This intentional architectural boundary ensures that root CI runs, `cargo clippy --all-targets --all-features`, and standard `cargo test` suites never require the `frida-core` C devkit or a C compilation toolchain on developer workstations. Explicit builds for lab or release environments are invoked via `cargo build --manifest-path tools/frida-worker/Cargo.toml`.

### Storage & Persistence Hierarchy

- **Platform-Local Persistence Root**: `dirs::data_local_dir()/emu/research` (`~/Library/Application Support/emu/research` on macOS). Missing directories return contextual errors.
- **Directory Hierarchy**:
  - `instances/`: Instance registration descriptors (`<id>.json`).
  - `instances/locks/`: Dedicated, permanent never-unlinked advisory run locks (`<id>.run.lock`).
  - `profiles/`: Versioned research experiment profiles (`<profile_id>.json`).
  - `artifacts/`: Validated firmware and disk image artifact records (`<sha256_digest>.json`).
  - `baselines/`: RecoveryBaseline reference state records (`<baseline_id>.json`).
  - `records/`: Immutable ExperimentRecord audit files (`<record_id>.json`).
  - `operations/`: OperationRecord journals (`<operation_id>.json`) and streamed event logs (`<operation_id>.events.jsonl`).
  - `operations/locks/`: Dedicated, permanent never-unlinked worker execution locks (`<operation_id>.op.lock`).
  - `proposals/`: MutationProposal records pending authorization (`<proposal_digest>.json`).
  - `security_profiles/`: GuestSecurityProfile definitions (`<profile_id>.json`).
- **Offline Destructive Operation Lock Contract**:
  - Offline destructive workers (disk wipe, image prep, baseline restore) MUST acquire BOTH `<vm_id>.device.lock` AND the instance run lock (`instances/locks/<vm_id>.run.lock`).
  - This requires verifying that any prior supervising guest process has been completely stopped and reaped before persistent storage modifications begin, guaranteeing single-owner process exclusivity and preventing race conditions with active hypervisors.
- **Short Unix Domain Socket Paths on macOS**:
  - In Darwin, `sizeof(((struct sockaddr_un *)0)->sun_path)` is strictly 104 bytes. Sockets are allocated in private owner-restricted (`0700`) temporary directories: `/tmp/emu-<short_uuid>/` (`qmp.sock`, `gdb.sock`, `console.sock`, `supervisor.sock`, `inferno-usb.sock`).
- **Transactional Semantics**: Staged atomic writes (`.tmp` write + `File::sync_all` + `std::fs::rename`); dedicated permanent never-unlinked lockfiles using `fs4` advisory locks; proposal consumption under lock before side effects begin; stale proposal revisions rejected.

### Public CLI Grammar Authority (`contracts/cli.md`)

- **Subcommand Hierarchy**: 11 subcommands across 10 grouped families under `emu research` governed by `contracts/cli.md`:
  1. `backend` (`preflight`, `list`)
  2. `guest` (`create`, `list`, `inspect`, `start`, `stop`, `restart`, `delete`, `wipe`)
  3. `root` (`verify`, `status`, `console`, `fs-read`, `fs-write`, `fs-export`, `ps`, `mach-services`, `security inspect`, `security apply`, `security revert`)
  4. `app` (`import`, `install`, `list`, `launch`, `inspect`, `stop`, `remove`, `container-read`, `container-write`, `container-export`)
  5. `frida` (`prepare`, `install`, `configure`, `start`, `attach`, `spawn`, `inspect`, `detach`, `stop`, `remove`)
  6. `debug` (`pause`, `resume`, `registers`, `memory`, `breakpoint`, `step`, `status`, `disconnect`)
  7. `image` (`register`, `prepare`, `verify-mount`, `cleanup`, `inspect`, `list`)
  8. `companion` (`start`, `inspect`, `stop`, `status`)
  9. `profile` (`apply`, `export`, `import`) / `baseline` (`create`, `restore`, `inspect`)
  10. `operation` (`status`, `cancel`, `wait`, `events`) / `record` (`inspect`, `list`)
- **Canonical Exit Codes**:
  - `0`: Success (`completed` / `already_satisfied` / `proposal_created`)
  - `1`: Runtime failure (`execution_failed`)
  - `2`: Invalid input (`invalid_input`)
  - `3`: Capability unsupported (`unsupported`)
  - `4`: Authorization refused (`auth_refused` / `AUTH_REQUIRED`)
  - `5`: Concurrency or state conflict (`conflict`)
  - `124`: Timeout or cancellation pending (`timeout` / `cancellation_pending`)
  - `130`: Cancelled at verified safe boundary (`cancelled`)
- **Stream Separation**: `stdout` outputs strictly finite JSON (`OutputEnvelope`); `stderr` outputs append-only JSONL progress records (`StreamLogEnvelope`).

### Target Platform & Cohort Scale

- **Target Platform**: macOS Apple Silicon (`aarch64`, Darwin 25.x / macOS 15+). While `Hypervisor.framework` availability (`sysctl kern.hv_support = 1`) is evaluated during preflight diagnostics, the research hypervisors (`darwin-vm`, `Inferno`, companion `qemu-system-x86_64`) run in user space using QEMU TCG, so HVF entitlement is not an execution prerequisite. Non-macOS hosts report `unsupported` (Exit Code 3).
- **Evaluation Cohort**: 4-guest reference evaluation cohort (2 `darwin-vm`, 2 `Inferno`), executed sequentially on Apple Silicon hardware.
- **Resource Discipline**: No invented 32-instance target; no unrealistic 180s recovery or 50ms preflight promises. Configuration-specific deadlines are frozen per configuration, adhering to existing constitution budgets.

---

## Constitution Check

> **GATE STATUS**: Pre-Phase 0 and Post-Phase 1 gates are marked **DESIGN REVIEW ONLY**. Runtime passing status requires empirical laboratory execution on physical hardware during Phase 2.

### Principle I: Trait-Based Platform Abstraction & Command Decoupling

- **Pre-Phase 0 Research Gate**: Evaluated whether `darwin-vm` and `Inferno` backends can be integrated behind the shared `DeviceManager` trait and execute commands through `CommandExecutor`. Confirmed that hypervisor invocations and host utilities (`hdiutil`, `diskutil`, `ideviceinstaller`) must route through `CommandExecutor` rather than ad-hoc shell execution. Confirmed that QMP and GDB protocol clients must operate over pluggable async streams so that tests can substitute mock executors and stream fixtures without requiring physical hypervisors or Apple Silicon host hardware. (Status: DESIGN REVIEW ONLY - VALIDATED)
- **Post-Phase 1 Design Gate**: `DarwinVmManager` and `InfernoManager` are designed to implement `DeviceManager` alongside `AndroidManager` and `IosManager`. Out-of-process hypervisor control routes through `CommandExecutor` using typed in-place `run_typed` and `spawn_typed` methods returning `CommandOutput` and `ProcessHandle` respectively. QMP and GDB protocol communication is cleanly abstracted into discrete client modules (`src/protocols/qmp/`, `src/protocols/gdb/`) operating over `AsyncRead + AsyncWrite`, enabling unit and integration test coverage using `tokio::io::duplex` without launching hypervisors. (Status: DESIGN REVIEW ONLY - VALIDATED POST-DESIGN)

### Principle II: Non-Blocking Async State & Concurrency Invariants

- **Pre-Phase 0 Research Gate**: Analyzed TUI event loops in `src/app/mod.rs` and confirmed that headless automation commands (`emu research ...`) must bypass `App::new()` to prevent background UI thread spawning and terminal raw mode allocation. Analyzed long-running hypervisor operations (booting, image preparation, baseline recovery) and verified that all such mutations must execute in background tasks or private child processes (`emu __supervise`, `emu __worker`) with cooperative cancellation tokens and state journals. (Status: DESIGN REVIEW ONLY - VALIDATED)
- **Post-Phase 1 Design Gate**: Headless CLI automation routes directly through `src/cli/` and instantiates service coordinators without initializing the TUI runtime or spawning UI event loops. Running guests are supervised by dedicated, unprivileged child processes (`emu __supervise --vm-id <ID>`) that own process handles, QMP sockets, and advisory run locks. Long-running mutations execute via finite workers (`emu __worker --operation-id <ID>`). Shared mutable state uses `tokio::sync::Mutex` and atomic primitives; zero `std::sync::Mutex` instances are held across await points; `AppState` remains the single source of truth for UI coordination. (Status: DESIGN REVIEW ONLY - VALIDATED POST-DESIGN)

### Principle III: Zero Magic Constants & Strict Idiomatic Rust Quality

- **Pre-Phase 0 Research Gate**: Audited constants in `src/constants/` and confirmed that all new research CLI subcommands, flags, timeout limits, exit codes, QMP verbs, GDB RSP packet headers, file paths, regexes, and user-facing messages must be centrally declared in `src/constants/research.rs` and related constants modules. (Status: DESIGN REVIEW ONLY - VALIDATED)
- **Post-Phase 1 Design Gate**: All string literals, exit codes (`0`, `1`, `2`, `3`, `4`, `5`, `124`, `130`), CLI flags, configuration timeouts, storage paths, and error codes are planned for central definition in `src/constants/research.rs`. String formatting strictly uses inline variable interpolation (`format!("{variable}")`). Error propagation uses `anyhow::Result` with `with_context` and `thiserror` domain enums; zero `.unwrap()` or `.expect()` calls in production code paths. (Status: DESIGN REVIEW ONLY - VALIDATED POST-DESIGN)

### Principle IV: Responsiveness & Performance Budgets

- **Pre-Phase 0 Research Gate**: Evaluated performance budgets: 8ms terminal input polling loop (~120 fps target), <150ms cold startup, 50ms device details loading, 10ms log streaming buffer latency, 100ms routine UI acknowledgment, 200ms modal prompt acknowledgment, and configuration-specific baseline recovery. Confirmed architectural compatibility. (Status: DESIGN REVIEW ONLY - VALIDATED)
- **Post-Phase 1 Design Gate**: Headless CLI commands complete preflight checks without UI overhead. TUI research panels integrate with asynchronous background channels without impacting the 8ms polling loop. Device detail queries resolve via local caching. Log streaming buffers operate within 10ms latency. Routine UI operations acknowledge in <100ms and modal prompts in <200ms. Baseline recovery procedures operate within pre-declared configuration deadlines. (Status: DESIGN REVIEW ONLY - VALIDATED POST-DESIGN)

### Principle V: Mock-Driven Test Isolation & Falsifiable Verification

- **Pre-Phase 0 Research Gate**: Reviewed CI test constraints. Confirmed that no physical iOS firmware, Apple Silicon hypervisors, IPSW restore files, or macOS-specific virtualization entitlements may be required for automated CI suites. (Status: DESIGN REVIEW ONLY - VALIDATED)
- **Post-Phase 1 Design Gate**: Automated CI test suite (`tests/unit/`, `tests/integration/`, `tests/contract/`) is designed to use `MockCommandExecutor`, `MockBackend`, and static JSON/plist fixtures to verify all command routes, lifecycle state transitions, JSON envelopes, and error conditions under `RUST_TEST_THREADS=1`. Separate controlled empirical test gates (G-01 through G-08, T-01 through T-08) and the 16 critical negative/falsification test cases govern real hypervisor and kernel execution in dedicated lab environments. (Status: DESIGN REVIEW ONLY - VALIDATED POST-DESIGN)

### Principle VI: Mobile Security Research Specialization & Grounded Scope

- **Pre-Phase 0 Research Gate**: Evaluated research mission and confirmed that Apple research backends (`darwin-vm` and `Inferno`) represent a required delivery target for macOS Apple Silicon. Verified that public Corellium research concepts serve strictly as a capability benchmark rather than an existing implementation claim. (Status: DESIGN REVIEW ONLY - VALIDATED)
- **Post-Phase 1 Design Gate**: Design focuses concretely on `darwin-vm` (minimal Darwin kernel, root bootstrap shell, benign CLI binaries, low-level GDB debugging) and `Inferno` (iOS research emulation platform based on QEMU-SPTM supporting SpringBoard, owned app lifecycle via `InstallationProxy`, and Frida dynamic instrumentation). Documentation and interfaces truthfully state that candidate configurations are research targets subject to empirical proof, making no unverified parity claims. (Status: DESIGN REVIEW ONLY - VALIDATED POST-DESIGN)

### Principle VII: Evidence-Backed Backend Capability Boundaries

- **Pre-Phase 0 Research Gate**: Examined capability boundaries across host operating systems and backend hypervisors. Confirmed that user-space simulation (iOS Simulator via `simctl`), Darwin virtual machines (`darwin-vm`), and full iOS guest systems (`Inferno`) must not be conflated. (Status: DESIGN REVIEW ONLY - VALIDATED)
- **Post-Phase 1 Design Gate**: Preflight diagnostics inspect host OS, CPU architecture (`aarch64`), and hypervisor entitlements (`Hypervisor.framework`). Non-macOS hosts are truthfully reported as `unsupported` (Gate G-01). `darwin-vm` explicitly declares `app_frameworks_supported = false`, refusing application deployment with structured errors (`AppFrameworksUnavailable`; SC-003, FR-023) rather than fabricating success or falling back silently. (Status: DESIGN REVIEW ONLY - VALIDATED POST-DESIGN)

### Principle VIII: Authorized Isolated Environments & Reproducible Provenance

- **Pre-Phase 0 Research Gate**: Assessed guest privilege containment, host security posture invariance, and explicit authorization requirements for destructive actions. (Status: DESIGN REVIEW ONLY - VALIDATED)
- **Post-Phase 1 Design Gate**: All guest privilege escalations (UID 0 root, kernel debugging, dynamic instrumentation) are strictly contained within virtual guest environments with zero modification of host security posture (SIP, SSV, and host NVRAM remain 100% untouched). Destructive actions (disk wipes, instance deletions, kernel swaps, baseline restores) generate a `MutationProposal` with a SHA-256 digest and require explicit `--authorize sha256:<digest>` or interactive confirmation naming affected resources. Full environment provenance is recorded in immutable `ExperimentRecord` files. (Status: DESIGN REVIEW ONLY - VALIDATED POST-DESIGN)

---

## Project Structure

### Documentation Structure (`specs/002-ios-root-vm/`)

```text
specs/002-ios-root-vm/
├── plan.md                  # This file (Phase 1 execution plan)
├── research.md              # Phase 0 research & architectural decisions
├── data-model.md            # Phase 1 data model, entities & state transitions
├── quickstart.md            # Phase 1 walkthrough scenarios & operational validation
├── contracts/               # Phase 1 interfaces & machine-readable schemas
│   ├── cli.md               # Public CLI grammar authority & exit code contract
│   └── research.schema.json # JSON Schema for output envelopes and models
├── checklists/              # Specification verification checklists
│   └── requirements.md      # Specification requirements quality checklist
└── tasks.md                 # Phase 2 work breakdown & task execution graph (from earlier flow; untouched)
```

> **Task Reconciliation Note**: `tasks.md` exists from an earlier generation flow; it is left untouched during Phase 1 design and MUST be reconciled during the next `speckit.tasks` command.

### Planned Source Code Structure (`src/`)

The implementation extends the existing single-crate repository layout under `src/`:

```text
src/
├── main.rs                          # Binary entrypoint, subcommand routing, headless CLI bypass
├── lib.rs                           # Library root, module exports
├── cli/                             # Headless CLI automation & command parsing (PLANNED)
│   ├── mod.rs                       # CLI hierarchy and clap definitions (emu research ...)
│   ├── backend.rs                   # Backend capability discovery and preflight commands (preflight, list)
│   ├── guest.rs                     # Guest instance lifecycle (create, list, inspect, start, stop, restart, delete, wipe)
│   ├── root.rs                      # Root proof, console, fs-read/write/export, ps, mach-services, security inspect/apply/revert
│   ├── app.rs                       # App lifecycle (import, install, list, launch, inspect, stop, remove, container access)
│   ├── frida.rs                     # Frida instrumentation (prepare, install, configure, start, attach, spawn, inspect, detach, stop, remove)
│   ├── debug.rs                     # Kernel debugging (pause, resume, registers, memory, breakpoint, step, status, disconnect)
│   ├── image.rs                     # Image artifact management (register, prepare, verify-mount, cleanup, inspect, list)
│   ├── companion.rs                 # Local companion VM inspection and lifecycle commands (start, inspect, stop, status)
│   ├── profile.rs                   # Profile and baseline management (profile apply/export/import, baseline create/restore/inspect)
│   ├── operation.rs                 # Operation tracking and records (operation status/cancel/wait/events, record inspect/list)
│   ├── supervisor.rs                # Private child supervisor entrypoint (emu __supervise)
│   ├── worker.rs                    # Private worker process entrypoint (emu __worker)
│   └── envelope.rs                  # Standard JSON output and JSONL log streaming envelopes
├── managers/                        # Virtualization backend device managers
│   ├── mod.rs                       # Manager module declarations
│   ├── common.rs                    # DeviceManager trait and common interfaces (CURRENT, UNTOUCHED)
│   ├── mock.rs                      # Mock backend implementations (CURRENT)
│   ├── android/                     # Standard Android AVD manager (CURRENT, FUNCTIONALLY PRESERVED)
│   ├── ios/                         # Standard iOS Simulator manager (CURRENT, FUNCTIONALLY PRESERVED)
│   ├── darwin_vm/                   # Darwin minimal virtual machine manager (PLANNED)
│   │   ├── mod.rs                   # DarwinVmManager implementing DeviceManager
│   │   ├── discovery.rs             # Instance discovery from local data directory
│   │   ├── lifecycle.rs             # Boot, shutdown, reset, and supervisor process launch
│   │   └── details.rs               # Runtime properties, kernel version, and status inspection
│   └── inferno/                     # Inferno iOS research virtualization backend (PLANNED)
│       ├── mod.rs                   # InfernoManager implementing DeviceManager
│       ├── discovery.rs             # Instance discovery from local data directory
│       ├── lifecycle.rs             # Boot, shutdown, reset, and supervisor process launch
│       ├── details.rs               # Runtime properties, SpringBoard status, and inspection
│       └── app_proxy.rs             # ideviceinstaller / InstallationProxy protocol integration
├── services/                        # Domain engines & research coordinators (PLANNED)
│   ├── mod.rs                       # Services module exports
│   └── research/                    # Research domain engine
│       ├── mod.rs                   # Research service facade and coordinator
│       ├── coordinator.rs           # Operation dispatcher, proposal generator, worker spawner
│       ├── reconciler.rs            # Crash recovery, supervisor health, orphaned PID cleanup
│       ├── provenance.rs            # Immutable ExperimentRecord generation and verification
│       ├── verifier.rs              # RootProofEvidence execution engine (positive + negative probe)
│       └── companion.rs             # Local companion Linux VM lifecycle coordinator
├── protocols/                       # Hypervisor & debugging protocol clients (PLANNED)
│   ├── mod.rs                       # Protocols module declarations
│   ├── qmp/                         # QEMU Machine Protocol (QMP) client over Unix Domain Socket
│   │   ├── mod.rs                   # QMP client facade and exports
│   │   ├── client.rs                # Async QMP client connection, handshake, command dispatch
│   │   ├── codec.rs                 # Line-delimited JSON framing and wire encoder/decoder
│   │   ├── commands.rs              # Typed QMP commands (qmp_capabilities, stop, cont, quit)
│   │   └── events.rs                # Asynchronous QMP event parsing (SHUTDOWN, RESET, STOP)
│   └── gdb/                         # GDB Remote Serial Protocol (RSP) client
│       ├── mod.rs                   # GDB client facade and exports
│       ├── client.rs                # Async GDB RSP client over chardev/loopback socket
│       ├── packet.rs                # RSP packet framing, checksumming ($...#xx), and ACK/NAK
│       ├── registers.rs             # ARM64 register set mapping (x0-x30, sp, pc, pstate)
│       └── lease.rs                 # Exclusive KernelDebugLease concurrency contract
├── workers/                         # Native worker integration wrappers (PLANNED)
│   ├── mod.rs                       # Worker module declarations
│   └── frida/                       # Isolated Frida dynamic instrumentation worker bridge
│       ├── mod.rs                   # Frida worker client facade
│       ├── bridge.rs                # Stdio line-delimited JSON IPC transport
│       └── protocol.rs              # Typed request, response, and telemetry event records
├── persistence/                     # Transactional filesystem persistence & locking (PLANNED)
│   ├── mod.rs                       # Storage engine interface
│   ├── paths.rs                     # dirs::data_local_dir()/emu/research path resolution
│   ├── atomic.rs                    # Staged atomic writes (.tmp + File::sync_all + rename)
│   ├── lock.rs                      # Cross-process advisory file locking via fs4
│   ├── instances.rs                 # ResearchGuestInstance repository
│   ├── profiles.rs                  # ResearchExperimentProfile repository
│   ├── artifacts.rs                 # ResearchImageArtifact repository and staging
│   ├── baselines.rs                 # RecoveryBaseline repository
│   ├── records.rs                   # Immutable ExperimentRecord repository
│   ├── operations.rs                # OperationRecord journal and event log streaming
│   └── proposals.rs                 # MutationProposal storage and authorization matching
├── models/                          # Domain models and data types
│   ├── mod.rs                       # Model exports
│   ├── platform.rs                  # Platform types (CURRENT)
│   ├── device.rs                    # Device trait and existing models (CURRENT)
│   ├── error.rs                     # Error types and domain error enums (CURRENT)
│   └── research/                    # Research domain entities and value objects (PLANNED)
│       ├── mod.rs                   # Research model exports
│       ├── guest.rs                 # ResearchGuestInstance, ResearchGuestId, LifecycleState
│       ├── backend.rs               # BackendCapabilityProfile, BinaryPrerequisite, Entitlements
│       ├── artifact.rs              # ResearchImageArtifact, ArtifactTrustStatus, Sha256Digest
│       ├── proof.rs                 # RootProofEvidence, RootVerificationState, ProbeOutcome
│       ├── app.rs                   # ApplicationArtifact, AppDeploymentStatus
│       ├── instrumentation.rs       # InstrumentationSession, FridaAttachmentState, HookStatus
│       ├── security_profile.rs      # GuestSecurityProfile, CodeSigningMode, AmfiStatus
│       ├── companion.rs             # CompanionEnvironment, CompanionLifecycleState
│       ├── profile.rs               # ResearchExperimentProfile, TargetBackend
│       ├── record.rs                # ExperimentRecord, ImmutableTrialSnapshot
│       ├── baseline.rs              # RecoveryBaseline, BaselineDiskArtifact
│       ├── operation.rs             # OperationRecord, OperationState, OperationPhase, ExitCode
│       └── proposal.rs              # MutationProposal, ResourceTarget, AuthorizationSignature
├── app/                             # Interactive TUI application runtime & event loop
│   ├── mod.rs                       # App struct, event processing, background workers (CURRENT)
│   ├── state/                       # UI state models and active selections (CURRENT)
│   └── research/                    # TUI research panels and interaction handlers (PLANNED)
│       ├── mod.rs                   # Research panel integration
│       ├── guest_table.rs           # Dual-backend guest instance listing with badges
│       ├── root_status.rs           # Root proof verification status & telemetry widget
│       ├── app_panel.rs             # Application lifecycle & container inspection widget
│       ├── frida_panel.rs           # Frida dynamic instrumentation trace monitor
│       ├── kernel_panel.rs          # Kernel debugger registers, memory, and stepping widget
│       └── dialogs.rs               # Destructive action authorization modal dialogs
├── ui/                              # Terminal rendering and widgets (CURRENT)
│   ├── panels/                      # TUI UI panels (devices, details, logs, commands)
│   ├── dialogs/                     # Confirmation, notification, and creation dialogs
│   └── widgets.rs                   # Custom Ratatui widgets (badges, status bars)
├── constants/                       # Centralized immutable constants (CURRENT)
│   ├── mod.rs                       # Constants module exports
│   ├── android.rs                   # Existing Android constants
│   ├── ios.rs                       # Existing iOS constants
│   ├── commands.rs                  # Existing command constants
│   └── research.rs                  # Research CLI commands, timeouts, paths, exit codes (PLANNED)
└── utils/                           # Shared utility primitives (CURRENT)
    ├── mod.rs                       # Utilities exports
    ├── command.rs                   # CommandRunner implementing CommandExecutor (CURRENT)
    ├── command_executor.rs          # CommandExecutor trait, in-place typed methods (UPGRADE)
    ├── validation.rs                # Input validation and SHA-256 helpers (CURRENT)
    └── logger.rs                    # Logging configuration and formatters (CURRENT)

src/bin/
└── debug_avd.rs                     # Existing debug tool (CURRENT)

tools/                               # Separately built private helper tools (PLANNED)
└── frida-worker/                    # Standalone Cargo package outside root workspace
    ├── Cargo.toml                   # Links frida-core devkit (explicit build only)
    └── src/
        └── main.rs                  # emu-frida-worker executable entrypoint

tests/
├── contract/                        # CLI JSON envelope and schema verification tests (PLANNED)
│   ├── cli_exit_codes.rs            # Exit code contract validation (0, 1, 2, 3, 4, 5, 124, 130)
│   ├── json_envelope.rs             # OutputEnvelope and StreamLogEnvelope schema tests
│   └── proposal_auth.rs             # MutationProposal digest and authorization contract tests
├── integration/                     # Multi-module workflow integration tests (PLANNED)
│   ├── dual_backend_lifecycle.rs    # Concurrent darwin-vm & Inferno instance management
│   ├── root_proof_falsification.rs  # Positive probe & negative control verification tests
│   ├── app_lifecycle_frida.rs       # Application deployment & Frida hook telemetry tests
│   ├── kernel_debug_lease.rs        # GDB RSP registers, memory, and exclusive lease tests
│   ├── companion_orchestration.rs   # Local companion VM lifecycle and dependency binding
│   ├── baseline_recovery.rs         # RecoveryBaseline rollback and state verification
│   └── crash_reconciliation.rs      # Supervisor failure and orphaned process cleanup tests
└── unit/                            # Unit tests for domain models, parsers, and persistence (PLANNED)
    ├── guest_instance_test.rs       # Unique identifier formatting and validation
    ├── qmp_protocol_test.rs         # QMP greeting banner, negotiation, and framing tests
    ├── gdb_rsp_packet_test.rs       # GDB packet checksumming, ARM64 register layouts
    ├── atomic_persistence_test.rs   # Staged atomic write and fs4 file lock tests
    └── capability_profile_test.rs   # Preflight hypervisor and entitlement checks
```

---

## Complexity Tracking

> **Note**: All 8 core principles of the Emu Constitution pass without architectural violations. The structural trade-offs below document the rationale for rejecting simpler or naive alternatives in favor of strictly isolated process boundaries and verified operational invariants.

| Architectural Boundary / Trade-off                                                                                                                                                           | Why Needed                                                                                                                                                                                                                        | Simpler Alternative Rejected Because                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Out-of-Process QMP/GDB Supervisor instead of in-process QEMU FFI**                                                                                                                         | Hypervisor crash containment, thread safety, and independent compiler toolchains.                                                                                                                                                 | Direct C/Rust FFI (`bindgen` to QEMU internals or UniFFI) was rejected because QEMU forks running experimental guest kernels experience panics and memory aborts that would crash the entire Emu CLI/TUI session. Furthermore, `Inferno` claims process-wide signal handlers, `CFRunLoop`, and global mutable state in `system/main.c:44-96` that conflict with Tokio and Ratatui, and process isolation provides clean operational and crash separation, avoiding binary linking pending formal legal review.                                                                                                                                                                                                               |
| **Isolated `emu-frida-worker` child executable in `tools/frida-worker/` with Server-Auth TLS & Client Token over Owned SSH Bridge instead of in-process Frida FFI or root `src/bin` helper** | GLib event loop isolation, host macOS process injection protection, rogue local service defense, and build isolation preventing root `cargo clippy --all-targets --all-features` and CI tests from requiring the native C devkit. | Linking `frida-core` C devkit directly into the Emu supervisor or TUI binary was rejected because `frida-core` relies on GLib (`GMainContext`, `GMainLoop`), causing potential thread contention, signal conflicts, and event loop starvation inside Tokio/Ratatui async loops. Furthermore, placing `emu-frida-worker` under `src/bin/` in the root crate would force every root `cargo clippy --all-targets --all-features` and test run to compile and link the native C devkit. Placing it in `tools/frida-worker/` outside the root workspace allows the main crate to retain typed IPC and mocks only, with explicit lab/release builds via `--manifest-path tools/frida-worker/Cargo.toml`.                           |
| **Single-Binary Child Supervisor (`emu __supervise`) instead of Host Daemon**                                                                                                                | Reliable process tree ownership and zero-friction developer setup without host mutation.                                                                                                                                          | A persistent background host daemon (`emud` managed via `launchd`) was rejected because it introduces installation friction, requires elevated privileges or launchd plist registration, and leaves stale state across uninstalls. A single binary with an unprivileged child supervisor process (`emu __supervise --vm-id <ID>`) guarantees bounded process lifetimes, automatic resource reaping, and strict containment. The supervisor does not terminate on guest pause or halt; it terminates strictly when the guest process is reaped AND zero dependent helper resources or workflows remain.                                                                                                                       |
| **Documented Local `qemu-system-x86_64` Companion VM on Same Mac via UDS USB instead of Speculative ARM64/HVF or Remote Linux Host**                                                         | Self-contained Apple Silicon execution matching official upstream Inferno documentation.                                                                                                                                          | Requiring an external physical Linux machine complicates developer environments and violates self-contained testing on macOS Apple Silicon. Speculative ARM64/HVF companion alternatives lack upstream documentation. The documented custom `qemu-system-x86_64` binary built from Inferno running under TCG on the same Mac via UDS USB (`dev-tcp-remote` listener / `hcd-tcp` client) executes cleanly in user space without nested virtualization entitlements.                                                                                                                                                                                                                                                           |
| **Supervisor RSP Ownership with Truthful Disconnect Handling instead of Forwarding `D`/`c` or Auto-Stop**                                                                                    | Preventing silent execution resumption and state mutation on observer disconnect.                                                                                                                                                 | In QEMU `gdbstub/gdbstub.c:1061`, processing a `D` (detach) packet calls `gdb_continue() -> vm_start()`, silently resuming execution. Having the supervisor own the RSP connection and never forward `D`/`c`/`k` ensures the guest stays paused. On unexpected transport loss, querying QMP `query-status` reports actual state (`paused`, `running`, `unknown`) truthfully without issuing mutating QMP `stop` or auto-retrying.                                                                                                                                                                                                                                                                                            |
| **Dedicated Benign Fixture Helper (`emu-root-probe`) for Root Falsification instead of Shell `su` Exit Code**                                                                                | Genuine effective UID 0 positive verification and non-zero UID negative control denial.                                                                                                                                           | Checking shell `su - mobile` exit codes is fragile and misleading: a missing `su` binary (exit 127) or non-existent `mobile` account (exit 1) produces non-zero exits that mimic permission denial. A dedicated fixture helper verifies actual `geteuid() == 0` for positive control with full payload byte count (`bytes_written == payload.len()`) and readback matching, executes a verified privilege drop to a declared non-zero UID/GID (`geteuid() != 0`), closes privileged descriptors, and asserts unprivileged open/write denial with `errno` strictly `EACCES` or `EPERM`. If privilege dropping fails or prerequisites are missing, the helper fails closed to `unverified` without inventing new status enums. |
| **Streaming Plain-Text Parsing for `ideviceinstaller` Mutations instead of Fabricating JSON**                                                                                                | Grounded in actual upstream `ideviceinstaller.c` source code.                                                                                                                                                                     | Assuming JSON or XML output on mutating subcommands (`install`, `uninstall`) fails because upstream source proves mutating commands output carriage-return plain text (`\rInstall: Status (N%)`) and stderr error strings. Only `list --json` emits structured JSON. `CommandExecutor` captures stdout/stderr and monitors for the terminal `Complete` token.                                                                                                                                                                                                                                                                                                                                                                |
| **Short UDS Sockets under `/tmp/emu-<short_id>/` instead of `$HOME/Library/...`**                                                                                                            | Strict compliance with Darwin's 104-byte `sun_path` boundary.                                                                                                                                                                     | In Darwin, `sizeof(((struct sockaddr_un *)0)->sun_path)` is strictly 104 bytes. Paths under standard user data directories (`$HOME/Library/Application Support/emu/research/...`) exceed 104 bytes and fail with `EINVAL` or `ENAMETOOLONG`. Short temporary directories with `0700` permissions resolve this without security compromise.                                                                                                                                                                                                                                                                                                                                                                                   |
| **Two-Step Destructive Authorization with Proposal Digest Consumption under Lock**                                                                                                           | Preventing accidental or unauthorized destructive mutations.                                                                                                                                                                      | Single-step prompts in headless automation either hang or allow accidental data loss. Generating a `MutationProposal` with a SHA-256 digest via `--dry-run` and requiring `--authorize sha256:<digest>` ensures explicit researcher intent. The proposal is verified and consumed under the instance lock before side effects begin, rejecting stale revisions.                                                                                                                                                                                                                                                                                                                                                              |
| **Authoritative Live Reference Sets for Companion Lifecycle instead of Simple Refcount**                                                                                                     | Preventing premature teardown of shared companion resources.                                                                                                                                                                      | Simple reference counters can decrement prematurely during race conditions. Tracking authoritative sets of active restore operation IDs and live dependent guest instance IDs ensures the companion VM terminates ONLY when both sets are completely empty.                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
