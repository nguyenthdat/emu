# Implementation Plan: iOS Root VM and Darwin Security Research Backends

**Branch**: `002-ios-root-vm` | **Date**: 2026-09-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-ios-root-vm/spec.md`

## Summary

The iOS Root VM and Darwin Security Research Backends feature introduces dedicated low-level Darwin and virtualized iOS security research capabilities to Emu on macOS Apple Silicon (`aarch64`). The architecture incorporates a complementary dual-backend virtualization portfolio:

1. `darwin-vm` (specialized in minimal headless Darwin virtual machines, kernel bootstrap, benign guest CLI binaries, and low-level kernel debugging; userland application frameworks are explicitly reported as unavailable);
2. `Inferno` (specialized in complete iOS research environments based on QEMU-SPTM, supporting userland SpringBoard, owned compatible application lifecycle management via `InstallationProxy` and `ideviceinstaller`, and dynamic instrumentation).

The technical approach enforces strict process isolation and rejects in-process QEMU C/Rust FFI bindings (due to hypervisor crash containment, global mutable state, signal handler interception, and `CFRunLoop` ownership in upstream `Inferno` `system/main.c`, and GPL-2.0 distribution boundaries). Hypervisor backends are managed strictly as isolated child processes communicating via out-of-process protocols: line-delimited QEMU Machine Protocol (QMP) over owner-restricted Unix Domain Sockets for deterministic lifecycle control, and a typed GDB Remote Serial Protocol (RSP) client communicating over chardev sockets under an exclusive `KernelDebugLease` for register, memory, and single-step kernel debugging.

Dynamic instrumentation is decoupled into an isolated native child executable (`emu-frida-worker`) linked against the official `frida-core` 17.18.0 C devkit API, communicating with the supervisor over typed stdio JSON streams. This isolates GLib (`GMainContext`, `GMainLoop`) event loops from Tokio and Ratatui runtimes, enforces `DeviceManager.with_socket_backend_only()` to prevent accidental host process enumeration or injection, and guarantees that Rust panics and C callback leaks never destabilize the main Emu process.

Specialized restore workflows, ramdisk preparation, and USB-over-IP bridging for `Inferno` are managed via an isolated, local companion virtual machine on the same macOS host, avoiding external physical Linux hardware dependencies while binding companion lifetimes strictly to dependent restore workflows and live guest sessions.

The system adheres to a single-binary architecture (`emu`) that executes running guests via an unprivileged child supervisor (`emu __supervise --vm-id <ID>`) and long-running mutations via finite workers (`emu __worker --operation-id <ID>`), eliminating persistent host daemons (`launchd`). Transactional persistence and cross-process advisory locking (`fs4`) are rooted under `dirs::data_local_dir()/emu/research` (`~/Library/Application Support/emu/research`), with Unix Domain Sockets dynamically allocated under private `/tmp/emu-<short_uuid>/` directories to circumvent Darwin's 104-byte `sun_path` limit.

Empirical root proof (UID 0) in iOS-derived guests is established via harmless privileged execution against an isolated fixture (`/private/var/root/.emu_probe`) coupled with negative control falsification (UID 501 `mobile` denial), kernel telemetry, and benign test binary execution digests bound to runtime boot session IDs and configuration revisions. verified privilege states automatically invalidate upon reboot or configuration change. Standard Android AVD and macOS iOS Simulator workflows maintain 100% behavioral parity and zero regression (SC-008), with all eight Android 001 specification artifacts preserved byte-for-byte.

## Technical Context

**Language/Version**: Rust 2024 edition, pinned to `rustc 1.88.0` (declared in `.tool-versions`). The codebase adheres to the standard compiler toolchain with strict warnings (`-D warnings`) and clippy enforcement.

**Primary Dependencies**:

- Existing locked dependencies (`Cargo.lock`): `tokio 1.53.1` (features: `["full"]`), `clap 4.6.7` (features: `["derive", "env"]`), `ratatui 0.30.2`, `crossterm 0.29.0`, `serde 1.0.229` (features: `["derive"]`), `serde_json 1.0.151`, `anyhow 1.0.104`, `thiserror 2.0.20`, `dirs 7.0.0`, `regex 1.13`, `chrono 0.4` (features: `["serde"]`), `nix 0.29.0`.
- Proposed dependency additions / promotions:
  - `fs4 = "1.1.0"` (features: `["sync"]`, `default-features = false`) for cross-process advisory file locking via `fs4::FileExt` (MSRV 1.75, compatible with Rust 1.88.0; `std::fs::File::try_lock` rejected due to Rust 1.89+ requirement).
  - `sha2 = "0.10.9"` (promotion of existing locked transitive crate) for cryptographic artifact, profile manifest, and mutation proposal digest verification.
  - `uuid = "1.26.1"` (promotion of existing locked transitive crate, features: `["v4"]`) for unique instance, boot session, evidence, and transaction identifier generation.

**Storage**:

- File-based transactional persistence rooted at `dirs::data_local_dir()/emu/research` (`~/Library/Application Support/emu/research` on macOS). Missing local data directories yield contextual errors (no silent fallbacks or unwraps).
- Directory hierarchy:
  - `instances/`: Instance registration descriptors (`<instance_id>.json`).
  - `profiles/`: Versioned research experiment profile definitions (`<profile_id>.json`).
  - `artifacts/`: Validated firmware and disk image artifact records (`<sha256_digest>.json`).
  - `baselines/`: RecoveryBaseline reference state records (`<baseline_id>.json`).
  - `records/`: Immutable ExperimentRecord audit files (`<record_id>.json`).
  - `operations/`: OperationRecord journals (`<operation_id>.json`) and streamed diagnostic logs (`<operation_id>.events.jsonl`).
  - `proposals/`: MutationProposal records pending authorization (`<proposal_digest>.json`).
  - `security_profiles/`: GuestSecurityProfile definitions (`<profile_id>.json`).
  - `locks/`: Dedicated unlinked advisory lockfiles (`*.lock`).
- Short Unix Domain Socket Paths: Private owner-restricted (`0700`) temporary directory `/tmp/emu-<short_uuid>/` hosting `qmp.sock`, `gdb.sock`, `console.sock`, and `supervisor.sock` to strictly observe Darwin's 104-byte `sun_path` boundary (`sizeof(sockaddr_un.sun_path)`).
- Transactional semantics: Staged atomic writes (`.tmp` write + `File::sync_all` + `std::fs::rename`); dedicated unlinked lock files using `fs4` advisory locks; zero heavyweight C-FFI / SQL database engines to preserve clean Rust 1.88.0 toolchain compilation.

**Testing**:

- Automated CI Test Suite: Hermetic unit, contract, and integration tests under `cargo test --bins --tests` and `cargo test --features test-utils` executing with `RUST_TEST_THREADS=1`. External tool invocations (QEMU forks, `hdiutil`, `diskutil`, `ideviceinstaller`, `emu-frida-worker`) are simulated via `MockCommandExecutor`, `MockBackend`, and static JSON/plist fixtures. Standard CI runs completely isolated without host SDKs, hypervisors, or KVM requirements.
- Empirical Validation Laboratory: Separate controlled test harness on physical macOS Apple Silicon (`aarch64`) workstations executing empirical validation gates (G-01 through G-08, T-01 through T-08) and the 16 critical negative/falsification test cases within the 4-guest reference cohort (2 `darwin-vm`, 2 `Inferno`).
- Regression Testing: Every bug fix requires an observable failing-then-passing test before acceptance. Standard Android AVD and macOS iOS Simulator workflows evaluated across 20 fixed lifecycle trials.

**Target Platform**:

- Primary Host Platform: macOS Apple Silicon (`aarch64`, Darwin 25.x / macOS 15+) with `Hypervisor.framework` (`sysctl kern.hv_support = 1`).
- Gated / Unsupported Hosts: Linux and Windows report `unsupported` for Darwin/iOS research backends (FR-001). Intel macOS (`x86_64`) reports `unsupported` due to lack of Apple Silicon virtualization primitives.
- Host Preflight: Universal non-interactive diagnostics via `emu --check` and `emu research backend preflight --json`.

**Project Type**: Developer tool featuring both an interactive Terminal User Interface (TUI) and headless Command Line Interface (CLI) automation within a single unified binary.

**Performance Goals**:

- TUI event polling loop: 8ms (~120 fps target) with zero input-debouncing latency during navigation.
- Cold startup time: <150ms total execution time (typical ~104ms; headless CLI commands complete in <50ms).
- Device details loading: <50ms latency using in-memory caching and non-blocking background pre-fetching.
- Real-time device log streaming: <10ms buffer latency to TUI log panel and JSONL stderr stream.
- User interaction acknowledgment: <100ms for routine operations, <200ms for destructive action confirmation dialogs.
- Baseline recovery duration: <3 minutes (180s) to re-flash baseline disk/kernel artifacts and restore verified operational state.

**Constraints**:

- Zero magic constants: all command names, subcommands, arguments, timeouts, exit codes (`0`, `1`, `2`, `3`, `4`, `5`, `124`, `130`), regexes, and user-facing messages declared centrally in `src/constants/research.rs` and related constants modules.
- Formatting quality: strict inline variable syntax (`format!("{var}")`) across all format strings, log macros, and assertions; zero compiler warnings under `cargo clippy --all-targets --all-features -- -D warnings`; clean formatting under `cargo fmt --check`.
- Non-blocking async execution: long-running tasks run in Tokio background tasks with cancellation handles; zero `std::sync::Mutex` held across await points; `AppState` as central UI state.
- Guest containment: all guest privileges, root supercalls, custom kernels, and hooking frameworks operate strictly inside virtual guest instances; host operating system permissions, SIP, SSV, and network firewalls remain 100% unmodified.
- Destructive operation gating: persistent storage wipes, device deletions, kernel swaps, and baseline recoveries require explicit authorization (`--authorize sha256:<digest>`) or interactive confirmation naming affected resources and paths.

**Scale/Scope**:

- Support up to 32 concurrent virtual research instances per host (bounded by available host RAM, disk, and hypervisor resources).
- 10 standardized operational CLI command families (`backend`, `device`, `profile`, `artifact`, `app`, `tool`, `kernel`, `companion`, `baseline`, `operation`).
- 4-guest reference evaluation cohort (2 `darwin-vm`, 2 `Inferno`) with sequential execution and 16 critical negative falsification test cases.
- Full functional parity and zero regression for existing standard Android virtual devices (`avd:<name>`) and iOS simulator devices (`<udid>`).

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

### Principle I: Trait-Based Platform Abstraction & Command Decoupling

- **Pre-Phase 0 Research Gate**: Evaluated whether `darwin-vm` and `Inferno` backends can be integrated behind the shared `DeviceManager` trait and execute commands through `CommandExecutor`. Confirmed that hypervisor invocations and host utilities (`hdiutil`, `diskutil`, `ideviceinstaller`) must route through `CommandExecutor` rather than ad-hoc shell execution. Confirmed that QMP and GDB protocol clients must operate over pluggable async streams so that tests can substitute mock executors and stream fixtures without requiring physical hypervisors or Apple Silicon host hardware. (Result: PASS)
- **Post-Phase 1 Design Gate**: `DarwinVmManager` and `InfernoManager` implement `DeviceManager` alongside `AndroidManager`, `IosManager`, and `CuttlefishManager`. Out-of-process hypervisor control routes through `CommandExecutor` using typed `CommandSpec` and `ProcessHandle` abstractions. QMP and GDB protocol communication is cleanly abstracted into discrete client modules (`src/protocols/qmp/`, `src/protocols/gdb/`) that operate over arbitrary async byte streams (`AsyncRead + AsyncWrite`), allowing full unit and integration test coverage using `tokio::io::duplex` without launching hypervisors. (Result: PASS)

### Principle II: Non-Blocking Async State & Concurrency Invariants

- **Pre-Phase 0 Research Gate**: Analyzed TUI event loops in `src/app/mod.rs` and confirmed that headless automation commands (`emu research ...`) must bypass `App::new()` to prevent background UI thread spawning and terminal raw mode allocation. Analyzed long-running hypervisor operations (booting, image preparation, baseline recovery) and verified that all such mutations must execute in background tasks or private child processes (`emu __supervise`, `emu __worker`) with cooperative cancellation tokens and state journals. (Result: PASS)
- **Post-Phase 1 Design Gate**: Headless CLI automation routes directly through `src/cli/` and instantiates service coordinators without initializing the TUI runtime or spawning UI event loops. Running guests are supervised by dedicated, unprivileged child processes (`emu __supervise --vm-id <ID>`) that own process handles, QMP sockets, and advisory run locks. Long-running mutations execute via finite workers (`emu __worker --operation-id <ID>`). Shared mutable state uses `tokio::sync::Mutex` and atomic primitives; zero `std::sync::Mutex` instances are held across await points; `AppState` remains the single source of truth for UI coordination. (Result: PASS)

### Principle III: Zero Magic Constants & Strict Idiomatic Rust Quality

- **Pre-Phase 0 Research Gate**: Audited constants in `src/constants/` and confirmed that all new research CLI subcommands, flags, timeout limits, exit codes, QMP verbs, GDB RSP packet headers, file paths, regexes, and user-facing messages must be centrally declared in `src/constants/research.rs` and related constants modules. (Result: PASS)
- **Post-Phase 1 Design Gate**: All string literals, exit codes (`0`, `1`, `2`, `3`, `4`, `5`, `124`, `130`), CLI flags, default timeouts (8ms polling, <150ms startup, 50ms details, 10ms logs, 100/200ms UI ack, 180s baseline restore), storage paths, and error codes are centrally defined in `src/constants/research.rs`. String formatting strictly uses inline variable interpolation (`format!("{variable}")`). Error propagation uses `anyhow::Result` with `with_context` and `thiserror` domain enums; zero `.unwrap()` or `.expect()` calls exist in production code paths. 100% compliant with `clippy` and `fmt`. (Result: PASS)

### Principle IV: Responsiveness & Performance Budgets

- **Pre-Phase 0 Research Gate**: Evaluated performance budgets: 8ms terminal input polling loop (~120 fps target), <150ms cold startup, 50ms device details loading, 10ms log streaming buffer latency, 100ms routine UI acknowledgment, 200ms modal prompt acknowledgment, and <3 minutes (180s) baseline recovery. Confirmed architectural compatibility. (Result: PASS)
- **Post-Phase 1 Design Gate**: Headless CLI commands complete preflight checks in <50ms. TUI research widgets integrate with asynchronous background channels without impacting the 8ms polling loop. Device detail queries resolve in <50ms via local caching. Log streaming buffers operate within 10ms latency. Routine UI operations acknowledge in <100ms and modal prompts in <200ms. Baseline recovery procedures complete in under 3 minutes via staged artifact restoration. (Result: PASS)

### Principle V: Mock-Driven Test Isolation & Falsifiable Verification

- **Pre-Phase 0 Research Gate**: Reviewed CI test constraints. Confirmed that no physical iOS firmware, Apple Silicon hypervisors, IPSW restore files, or macOS-specific virtualization entitlements may be required for automated CI suites. (Result: PASS)
- **Post-Phase 1 Design Gate**: Automated CI test suite (`tests/unit/`, `tests/integration/`, `tests/contract/`) uses `MockCommandExecutor`, `MockBackend`, and static JSON/plist fixtures to verify all command routes, lifecycle state transitions, JSON envelopes, and error conditions under `RUST_TEST_THREADS=1`. Separate controlled empirical test gates (G-01 through G-08, T-01 through T-08) and the 16 critical negative/falsification test cases govern real hypervisor and kernel execution in dedicated lab environments. All bug fixes require observable failing-then-passing regression tests. (Result: PASS)

### Principle VI: Mobile Security Research Specialization & Grounded Scope

- **Pre-Phase 0 Research Gate**: Evaluated research mission and confirmed that Apple research backends (`darwin-vm` and `Inferno`) represent a required future delivery target and development scope for macOS Apple Silicon. Verified that public Corellium research concepts serve strictly as a capability benchmark and architectural aspiration rather than an existing implementation claim. (Result: PASS)
- **Post-Phase 1 Design Gate**: Design focuses concretely on `darwin-vm` (minimal Darwin kernel, root bootstrap shell, benign CLI binaries, low-level GDB debugging) and `Inferno` (iOS research emulation platform based on QEMU-SPTM supporting SpringBoard, owned app lifecycle via `InstallationProxy`, and Frida dynamic instrumentation). Documentation and interfaces truthfully state that candidate configurations are research targets subject to empirical proof, making no unverified parity claims. (Result: PASS)

### Principle VII: Evidence-Backed Backend Capability Boundaries

- **Pre-Phase 0 Research Gate**: Examined capability boundaries across host operating systems and backend hypervisors. Confirmed that user-space simulation (iOS Simulator via `simctl`), Darwin virtual machines (`darwin-vm`), and full iOS guest systems (`Inferno`) must not be conflated. (Result: PASS)
- **Post-Phase 1 Design Gate**: Preflight diagnostics inspect host OS, CPU architecture (`aarch64`), and hypervisor entitlements (`Hypervisor.framework`). Non-macOS hosts are truthfully reported as `unsupported` (Gate G-01). `darwin-vm` explicitly declares `app_frameworks_supported = false`, refusing application deployment with structured errors (`AppFrameworksUnavailable`; SC-003, FR-023) rather than fabricating success or falling back silently. (Result: PASS)

### Principle VIII: Authorized Isolated Environments & Reproducible Provenance

- **Pre-Phase 0 Research Gate**: Assessed guest privilege containment, host security posture invariance, and explicit authorization requirements for destructive actions. (Result: PASS)
- **Post-Phase 1 Design Gate**: All guest privilege escalations (UID 0 root, kernel debugging, dynamic instrumentation) are strictly contained within virtual guest environments with zero modification of host security posture (SIP, SSV, and host NVRAM remain 100% untouched). Destructive actions (disk wipes, instance deletions, kernel swaps, baseline restores) generate a `MutationProposal` with a SHA-256 digest and require explicit `--authorize sha256:<digest>` or interactive confirmation naming affected resources. Full environment provenance (kernel build IDs, image digests, toolchain versions, command options) is recorded in immutable `ExperimentRecord` files. (Result: PASS)

## Project Structure

### Documentation (this feature)

```text
specs/002-ios-root-vm/
├── plan.md                  # This file (Phase 1 execution plan)
├── research.md              # Phase 0 research & architectural decisions
├── data-model.md            # Phase 1 data model, entities & state transitions
├── quickstart.md            # Phase 1 walkthrough scenarios & operational validation
├── contracts/               # Phase 1 interfaces & machine-readable schemas
│   ├── cli.md               # CLI commands, arguments, and exit code specifications
│   └── research.schema.json # JSON Schema for output envelopes and models
├── checklists/              # Specification verification checklists
│   └── requirements.md      # Specification requirements quality checklist
└── tasks.md                 # Phase 2 work breakdown & task execution graph
```

### Source Code (repository root)

```text
src/
├── main.rs                          # Binary entrypoint, subcommand routing, --check preflight
├── lib.rs                           # Library root, module exports
├── cli/                             # Headless CLI automation & command parsing
│   ├── mod.rs                       # CLI hierarchy and clap definitions (emu research ...)
│   ├── backend.rs                   # Backend capability discovery and preflight commands
│   ├── device.rs                    # Guest instance lifecycle, creation, registration, deletion
│   ├── profile.rs                   # Declarative research profile management and proposals
│   ├── artifact.rs                  # Artifact registration, staging, and digest verification
│   ├── app.rs                       # Application lifecycle commands (install, launch, list, stop)
│   ├── tool.rs                      # Frida instrumentation deployment, attach, and trace
│   ├── kernel.rs                    # Kernel debugging commands (pause, resume, registers, step)
│   ├── companion.rs                 # Local companion VM inspection and lifecycle commands
│   ├── baseline.rs                  # Baseline capture, verification, and recovery commands
│   ├── operation.rs                 # Operation status, event streaming, and cancellation
│   ├── supervisor.rs                # Private child supervisor entrypoint (emu __supervise)
│   ├── worker.rs                    # Private worker process entrypoint (emu __worker)
│   └── envelope.rs                  # Standard JSON output and JSONL log streaming envelopes
├── managers/                        # Virtualization backend device managers
│   ├── mod.rs                       # Manager module declarations
│   ├── common.rs                    # DeviceManager trait and common interfaces
│   ├── mock.rs                      # Mock backend implementations
│   ├── android/                     # Standard Android AVD manager (untouched baseline)
│   ├── ios/                         # Standard iOS Simulator manager (untouched baseline)
│   ├── cuttlefish/                  # AOSP Cuttlefish virtualization backend (from feature 001)
│   ├── darwin_vm/                   # Darwin minimal virtual machine manager
│   │   ├── mod.rs                   # DarwinVmManager implementing DeviceManager
│   │   ├── discovery.rs             # Instance discovery from local data directory
│   │   ├── lifecycle.rs             # Boot, shutdown, reset, and supervisor process launch
│   │   └── details.rs               # Runtime properties, kernel version, and status inspection
│   └── inferno/                     # Inferno iOS research virtualization backend
│       ├── mod.rs                   # InfernoManager implementing DeviceManager
│       ├── discovery.rs             # Instance discovery from local data directory
│       ├── lifecycle.rs             # Boot, shutdown, reset, and supervisor process launch
│       ├── details.rs               # Runtime properties, SpringBoard status, and inspection
│       └── app_proxy.rs             # ideviceinstaller / InstallationProxy protocol integration
├── services/                        # Domain engines & research coordinators
│   ├── mod.rs                       # Services module exports
│   └── research/                    # Research domain engine
│       ├── mod.rs                   # Research service facade and coordinator
│       ├── coordinator.rs           # Operation dispatcher, proposal generator, worker spawner
│       ├── reconciler.rs            # Crash recovery, supervisor health, orphaned PID cleanup
│       ├── provenance.rs            # Immutable ExperimentRecord generation and verification
│       ├── verifier.rs              # RootProofEvidence execution engine (positive + negative)
│       └── companion.rs             # Local companion Linux VM lifecycle coordinator
├── protocols/                       # Hypervisor & debugging protocol clients
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
├── workers/                         # Native worker integration wrappers
│   ├── mod.rs                       # Worker module declarations
│   └── frida/                       # Isolated Frida dynamic instrumentation worker bridge
│       ├── mod.rs                   # Frida worker client facade
│       ├── bridge.rs                # Stdio line-delimited JSON IPC transport
│       └── protocol.rs              # Typed request, response, and telemetry event records
├── persistence/                     # Transactional filesystem persistence & locking
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
│   ├── platform.rs                  # Platform types
│   ├── device.rs                    # Device trait and existing models
│   ├── error.rs                     # Error types and domain error enums
│   └── research/                    # Research domain entities and value objects
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
│   ├── mod.rs                       # App struct, event processing, background workers
│   ├── state/                       # UI state models and active selections
│   └── research/                    # TUI research panels and interaction handlers
│       ├── mod.rs                   # Research panel integration
│       ├── guest_table.rs           # Dual-backend guest instance listing with badges
│       ├── root_status.rs           # Root proof verification status & telemetry widget
│       ├── app_panel.rs             # Application lifecycle & container inspection widget
│       ├── frida_panel.rs           # Frida dynamic instrumentation trace monitor
│       ├── kernel_panel.rs          # Kernel debugger registers, memory, and stepping widget
│       └── dialogs.rs               # Destructive action authorization modal dialogs
├── ui/                              # Terminal rendering and widgets
│   ├── panels/                      # TUI UI panels (devices, details, logs, commands)
│   ├── dialogs/                     # Confirmation, notification, and creation dialogs
│   └── widgets.rs                   # Custom Ratatui widgets (badges, status bars)
├── constants/                       # Centralized immutable constants (Principle III)
│   ├── mod.rs                       # Constants module exports
│   ├── android.rs                   # Existing Android constants
│   ├── ios.rs                       # Existing iOS constants
│   ├── commands.rs                  # Existing command constants
│   └── research.rs                  # Research CLI commands, timeouts, paths, exit codes, limits
└── utils/                           # Shared utility primitives
    ├── mod.rs                       # Utilities exports
    ├── command.rs                   # Command execution helpers
    ├── command_executor.rs          # CommandExecutor, CommandSpec, ProcessHandle, MockExecutor
    ├── validation.rs                # Input validation and SHA-256 helpers
    └── logger.rs                    # Logging configuration and JSONL formatters

src/bin/
└── emu-frida-worker.rs              # Isolated Frida dynamic instrumentation worker executable

tests/
├── contract/                        # CLI JSON envelope and schema verification tests
│   ├── cli_exit_codes.rs            # Exit code contract validation (0, 1, 2, 3, 4, 5, 124, 130)
│   ├── json_envelope.rs             # OutputEnvelope and StreamLogEnvelope schema tests
│   └── proposal_auth.rs             # MutationProposal digest and authorization contract tests
├── integration/                     # Multi-module workflow integration tests (mock-driven)
│   ├── dual_backend_lifecycle.rs     # Concurrent darwin-vm & Inferno instance management
│   ├── root_proof_falsification.rs   # Positive probe & negative control verification tests
│   ├── app_lifecycle_frida.rs       # Application deployment & Frida hook telemetry tests
│   ├── kernel_debug_lease.rs        # GDB RSP registers, memory, and exclusive lease tests
│   ├── companion_orchestration.rs   # Local companion VM lifecycle and dependency binding
│   ├── baseline_recovery.rs         # RecoveryBaseline rollback and state verification
│   └── crash_reconciliation.rs      # Supervisor failure and orphaned process cleanup tests
└── unit/                            # Unit tests for domain models, parsers, and persistence
    ├── guest_instance_test.rs       # Unique identifier formatting and validation
    ├── qmp_protocol_test.rs         # QMP greeting banner, negotiation, and framing tests
    ├── gdb_rsp_packet_test.rs       # GDB packet checksumming, ARM64 register layouts
    ├── atomic_persistence_test.rs   # Staged atomic write and fs4 file lock tests
    └── capability_profile_test.rs   # Preflight hypervisor and entitlement checks
```

**Structure Decision**:

The implementation retains a single unified binary project structure (`emu`) complemented by a private worker binary (`emu-frida-worker`). The existing `src/managers/android/` and `src/managers/ios/` modules remain untouched to preserve baseline functional parity and zero regressions for existing standard virtual devices (SC-008). The new `src/managers/darwin_vm/` and `src/managers/inferno/` managers implement the existing `DeviceManager` trait, providing clean polymorphic lifecycle dispatch.

Out-of-process hypervisor communication is cleanly isolated in `src/protocols/qmp/` and `src/protocols/gdb/`, operating over abstract asynchronous streams to enable deterministic unit and integration testing without requiring real hypervisors or host hardware. The Frida dynamic instrumentation engine is isolated into `src/bin/emu-frida-worker.rs` and accessed via `src/workers/frida/`, eliminating GLib runtime conflicts and preventing host process injection.

Headless automation is routed through `src/cli/`, completely bypassing `App::new()` to guarantee deterministic, non-blocking execution in CI and automated pipelines. Transactional persistence and advisory locking are isolated in `src/persistence/` using `dirs::data_local_dir()/emu/research` and `fs4`. The research engine, verifiers, and reconciler reside in `src/services/research/`, ensuring clean separation between business logic, hypervisor protocols, and terminal presentation.

## Complexity Tracking

> **Note**: All 8 core principles of the Emu Constitution pass without architectural violations. The structural trade-offs below document the rationale for rejecting simpler or naive alternatives (such as in-process FFI or background daemons) in favor of strictly isolated process boundaries.

| Architectural Boundary / Trade-off | Why Needed | Simpler Alternative Rejected Because |
| **Out-of-Process QMP/GDB Supervisor instead of in-process QEMU FFI** | Hypervisor crash containment, thread safety, and independent compiler toolchains. | Direct C/Rust FFI (`bindgen` to QEMU internals or UniFFI) was rejected because QEMU forks running experimental guest kernels experience panics and memory aborts that would crash the entire Emu CLI/TUI session. Furthermore, `Inferno` claims process-wide signal handlers, `CFRunLoop`, and global mutable state in `system/main.c:44-96` that conflict with Tokio and Ratatui, and GPL-2.0 distribution constraints require strict process separation. |
| **Isolated `emu-frida-worker` child executable instead of in-process Frida FFI** | GLib event loop isolation and host macOS process injection protection. | Linking `frida-core` C devkit directly into the Emu supervisor or TUI binary was rejected because `frida-core` relies on GLib (`GMainContext`, `GMainLoop`), causing thread contention, signal conflicts, and event loop starvation inside Tokio/Ratatui async loops. Memory management hazards with C callback retention and the risk of accidental host process injection from unconstrained local device discovery necessitated an isolated worker using `DeviceManager.with_socket_backend_only()`. |
| **Single-Binary Child Supervisor (`emu __supervise`) instead of Host Daemon** | Reliable process tree ownership and zero-friction developer setup without host mutation. | A persistent background host daemon (`emud` managed via `launchd`) was rejected because it introduces installation friction, requires elevated privileges or launchd plist registration, and leaves stale state across uninstalls. A single binary with an unprivileged child supervisor process (`emu __supervise --vm-id <ID>`) guarantees bounded process lifetimes, automatic resource reaping, and strict containment. |
