# Implementation Plan: Android Research Backends and Security Tooling

**Branch**: `001-add-android-research-backends` | **Date**: 2026-09-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-add-android-research-backends/spec.md`

## Summary

The Android Research Backends and Security Tooling feature introduces first-class mobile security research capabilities to Emu, prioritizing Android virtualization over future iOS research. The system integrates two core virtualization backends: standard Android Emulator (AVD) and AOSP Cuttlefish virtual devices. Building upon this dual-backend foundation, Emu enables automated and interactive deployment of a core security research triad (KernelSU Next v3.3.0 for root privilege mediation, Frida 17.18.0 for dynamic runtime instrumentation, and Vector v2.2 + NeoZygisk v2.4 for Xposed-compatible hooking) alongside an extended catalog of 9 advanced research capability families (alternative root flavors, SUSFS root hiding, NoMount/Mountify mount mediation, advanced in-kernel networking, tmpfs xattr security labels, eBPF/BTF observability, NTSync kernel synchronization, Droidspaces in-guest container runtimes, and partition protection).

The technical approach adheres to a single-binary architecture (`emu`) that cleanly decouples headless CLI automation from the interactive TUI. Unattended automation commands (`emu research ...`) instantiate backend managers directly and bypass TUI initialization (`App::new()`), preventing background UI thread spawning and raw terminal allocation. Long-running mutations (kernel flashing, boot monitoring, baseline recovery) execute via a private, finite worker child process (`emu __worker --operation-id <ID>`) with transactional JSON state journals and cross-process advisory locking (`fs4`) under `dirs::data_local_dir()/emu/research`. Devices are addressed via stable, backend-qualified unique identifiers (`<backend>:<opaque>`) that decouple internal targeting from user-facing display names, ephemeral ports, or ADB transport serials. Cuttlefish instances run within dedicated isolated runtime roots (`HOME=<dir>`), strictly enforcing single-instance groups to prevent cascading `stop_cvd` group-spill kills. Destructive operations require explicit, resource-scoped authorization (`--authorize sha256:<digest>`). Standard Android virtual devices (`avd:<name>`) and iOS simulator devices (`<udid>`) remain completely untouched with zero regressions. The implementation is verified through deterministic mock-driven unit and integration CI tests (`MockCommandExecutor`) alongside separate controlled empirical lab validation gates.

## Technical Context

**Language/Version**: Rust 2024 edition, pinned to `rustc 1.88.0` (declared in `.tool-versions`). The codebase adheres to the standard compiler toolchain with strict warnings (`-D warnings`) and clippy enforcement.

**Primary Dependencies**:

- Existing locked dependencies (`Cargo.lock`): `tokio 1.53.1` (features: `["full"]`), `clap 4.6.7` (features: `["derive", "env"]`), `ratatui 0.30.2`, `crossterm 0.29.0`, `serde 1.0.229` (features: `["derive"]`), `serde_json 1.0.151`, `anyhow 1.0.104`, `thiserror 2.0.20`, `dirs 7.0.0`, `regex 1.13`, `chrono 0.4` (features: `["serde"]`).
- Proposed dependency additions / promotions:
  - `fs4 = "1.1.0"` (features: `["sync"]`, `default-features = false`) for cross-process advisory file locking via `fs4::FileExt` (MSRV 1.75, compatible with Rust 1.88.0; `std::fs::File::try_lock` rejected due to Rust 1.89+ requirement).
  - `sha2 = "0.10.9"` (promotion of existing locked transitive crate) for cryptographic artifact and mutation proposal digest verification.
  - `uuid = "1.26.1"` (promotion of existing locked transitive crate, features: `["v4"]`) for unique operation and transaction identifier generation.

**Storage**:

- File-based transactional persistence rooted at `dirs::data_local_dir()/emu/research` (`~/.local/share/emu/research` on Linux, `~/Library/Application Support/emu/research` on macOS, `%LOCALAPPDATA%\emu\research` on Windows). Missing local data directories yield contextual errors (no silent fallbacks or unwraps).
- Directory hierarchy: `devices/`, `profiles/`, `artifacts/`, `baselines/`, `provenance/`, `operations/`, and `proposals/`.
- Transactional semantics: Staged atomic writes (`.tmp` write + `sync_all` + `std::fs::rename`); dedicated unlinked lock files (`.lock`) using `fs4` advisory locks; zero heavyweight C-FFI / SQL database engines to preserve clean Rust 1.88.0 toolchain compilation.

**Testing**:

- Automated CI Test Suite: Hermetic unit and integration tests under `cargo test --bins --tests` and `cargo test --features test-utils` executing with `RUST_TEST_THREADS=1`. External tool invocations are simulated via `MockCommandExecutor`, `MockBackend`, and static JSON/XML fixtures. Standard CI runs completely isolated without host SDKs, hypervisors, or KVM requirements.
- Empirical Validation Laboratory: Separate controlled test harness on physical/virtualized Linux (KVM x86_64/arm64) and macOS (Apple Silicon) workstations executing empirical validation gates (G-01 through G-08, T-01 through T-06) for live guest boot, kernel driver UAPI handshakes, Frida telemetry capture, and Vector scoped mediation.
- Regression Testing: Every bug fix requires an observable failing-then-passing test before acceptance.

**Target Platform**:

- Android Emulator: Linux x86_64 (`/dev/kvm`), Linux arm64 (`/dev/kvm`, runtime binary availability verified), macOS Apple Silicon arm64 (`Hypervisor.Framework`), Windows 10/11 x86_64 (WHPX).
- Cuttlefish Virtual Devices: Linux x86_64 and Linux arm64 with native KVM (`/dev/kvm`, `cuttlefish-base`, `kvm` and `cvdnetwork` groups). Reports `unsupported` on macOS and Windows hosts.
- iOS Simulator: macOS only, strictly isolated via `#[cfg(target_os = "macos")]` with zero baseline regression.
- Host Preflight: Universal non-interactive diagnostics via `emu --check` and `emu research backend preflight`.

**Project Type**: Developer tool featuring both an interactive Terminal User Interface (TUI) and headless Command Line Interface (CLI) automation within a single unified binary.

**Performance Goals**:

- TUI event polling loop: 8ms (~120 fps target) with zero input-debouncing latency during navigation.
- Cold startup time: <150ms total execution time (typical ~104ms; headless CLI commands complete in <50ms).
- Device details loading: <50ms latency using in-memory caching and non-blocking background pre-fetching.
- Real-time device log streaming: <10ms buffer latency to TUI log panel and JSONL stderr stream.
- User interaction acknowledgment: <100ms for routine operations, <200ms for destructive action confirmation dialogs.
- Baseline recovery duration: <3 minutes to re-flash baseline kernel/system artifacts and restore verified operational state.

**Constraints**:

- Zero magic constants: all command names, subcommands, arguments, timeouts, exit codes, regexes, and user-facing messages declared centrally in `src/constants/research.rs` and related constants modules.
- Formatting quality: strict inline variable syntax (`format!("{var}")`) across all format strings, log macros, and assertions; zero compiler warnings under `cargo clippy --all-targets --all-features -- -D warnings`; clean formatting under `cargo fmt --check`.
- Non-blocking async execution: long-running tasks run in Tokio background tasks with cancellation handles; zero `std::sync::Mutex` held across await points; `AppState` as central UI state.
- Guest containment: all guest privileges, root supercalls, custom kernels, and hooking frameworks operate strictly inside virtual guest instances; host operating system permissions and network firewalls remain unmodified.
- Destructive operation gating: persistent storage wipes, device deletions, kernel swaps, and baseline recoveries require explicit authorization (`--authorize sha256:<digest>`) or interactive confirmation naming affected resources.

**Scale/Scope**:

- Support up to 32 concurrent virtual research instances per host (bounded by available host RAM, disk, and KVM resources).
- 8 standardized operational CLI command families (`backend`, `device`, `profile`, `artifact`, `tool`, `experiment`, `baseline`, `operation`).
- 9 extended research capability families across operating system, kernel, protocol, and application layers.
- Full functional parity and zero regression for existing standard Android virtual devices (`avd:<name>`) and iOS simulator devices (`<udid>`).

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

### Principle I: Trait-Based Platform Abstraction & Command Decoupling

- **Pre-Phase 0 Research Gate**: Evaluated whether Android Emulator and Cuttlefish can be integrated through the existing `DeviceManager` trait and execute commands through `CommandExecutor`. Confirmed that `CuttlefishManager` can implement `DeviceManager` while encapsulating Cuttlefish-specific tooling (`launch_cvd`, `stop_cvd`, `powerwash_cvd`, `status`). Confirmed that ad-hoc OS commands must be eliminated in favor of an enhanced `CommandExecutor`. (Result: PASS)
- **Post-Phase 1 Design Gate**: `CuttlefishManager` implements `DeviceManager` alongside `AndroidManager` and `IosManager`. All host binary calls route through `CommandExecutor` using typed `CommandSpec` and `ProcessHandle` abstractions, eliminating dropped child handles and ad-hoc execution. Pluggable `MockCommandExecutor` covers all commands in deterministic unit/integration CI without requiring live SDKs or hypervisors. (Result: PASS)

### Principle II: Non-Blocking Async State & Concurrency Invariants

- **Pre-Phase 0 Research Gate**: Analyzed `App::new()` in `src/app/mod.rs` and identified that it unconditionally spawns background device and cache loading tasks, which would block or hang headless CLI automation. Determined that headless commands must bypass `App::new()` and instantiate managers directly. (Result: PASS)
- **Post-Phase 1 Design Gate**: Headless CLI commands (`emu research ...`) instantiate backend managers directly and run deterministically without initializing the TUI runtime or spawning UI event loops. Long-running asynchronous operations execute via a private, finite worker child process (`emu __worker --operation-id <ID>`) with exclusive file locks and state journals. Shared state uses `tokio::sync::Mutex` and atomic primitives; no `std::sync::Mutex` is held across await points; `AppState` remains the single source of truth for UI coordination. (Result: PASS)

### Principle III: Zero Magic Constants & Strict Idiomatic Rust Quality

- **Pre-Phase 0 Research Gate**: Audited existing constants in `src/constants/` and confirmed that all new research command names, subcommands, parameter keys, exit codes, timeout durations, regex patterns, and user messages must be centrally declared in `src/constants/research.rs`. (Result: PASS)
- **Post-Phase 1 Design Gate**: All string literals, exit code values (`0`, `1`, `2`, `3`, `4`, `5`, `124`, `130`), CLI flags, storage paths, default timeouts, and error codes are centrally defined in `src/constants/research.rs`. String formatting strictly uses inline variable interpolation (`format!("{variable}")`). Error propagation uses `anyhow::Result` with `with_context` and `thiserror` domain enums; zero `.unwrap()` or `.expect()` calls exist in production code paths. 100% compliant with `clippy` and `fmt`. (Result: PASS)

### Principle IV: Responsiveness & Performance Budgets

- **Pre-Phase 0 Research Gate**: Evaluated performance budgets: 8ms input loop, <150ms startup, 50ms details, 10ms log latency, 100/200ms UI acknowledgment, 3min baseline recovery. Confirmed architectural compatibility. (Result: PASS)
- **Post-Phase 1 Design Gate**: Headless CLI invocations bypass terminal initialization and execute preflight checks in <50ms. TUI research widgets integrate with asynchronous background channels without impacting the 8ms (~120 fps) polling loop. Device detail queries resolve in <50ms via local caching. Log streaming buffers operate within 10ms latency. Routine UI operations acknowledge in <100ms and modal prompts in <200ms. Baseline recovery procedures complete in under 3 minutes via staged artifact restoration. (Result: PASS)

### Principle V: Mock-Driven Test Isolation & Falsifiable Verification

- **Pre-Phase 0 Research Gate**: Reviewed CI test constraints. Confirmed that no physical Android SDKs, hypervisors, KVM access, or physical devices may be required for automated CI suites. (Result: PASS)
- **Post-Phase 1 Design Gate**: Automated CI test suite (`tests/unit/`, `tests/integration/`, `tests/contract/`) uses `MockCommandExecutor`, `MockBackend`, and static JSON/XML fixtures to verify all command routes, lifecycle state transitions, JSON envelopes, and error conditions under `RUST_TEST_THREADS=1`. Separate controlled empirical test gates (G-01 through G-08, T-01 through T-06) govern real hypervisor and kernel execution in dedicated lab environments. All bug fixes require observable failing-then-passing regression tests. (Result: PASS)

### Principle VI: Mobile Security Research Specialization & Grounded Scope

- **Pre-Phase 0 Research Gate**: Evaluated the research mission, confirming that Android virtualization (Android Emulator and Cuttlefish) is prioritized before iOS research. Acknowledged commercial platforms (e.g., Corellium) as long-term benchmarks rather than claiming existing parity. (Result: PASS)
- **Post-Phase 1 Design Gate**: Design focuses concretely on Android Emulator and Cuttlefish with KernelSU Next v3.3.0, Frida 17.18.0, and Vector v2.2 + NeoZygisk v2.4 across core OS, kernel, and application research scopes. Documentation and interfaces truthfully state that candidate configurations are research targets subject to empirical proof, making no unverified parity claims. (Result: PASS)

### Principle VII: Evidence-Backed Backend Capability Boundaries

- **Pre-Phase 0 Research Gate**: Examined capability boundaries across host operating systems and backend hypervisors. (Result: PASS)
- **Post-Phase 1 Design Gate**: Preflight diagnostics inspect host OS, CPU architecture, and hypervisor entitlements. Cuttlefish on macOS/Windows is truthfully reported as `unsupported` (Gate G-02). Linux arm64 Emulator execution is explicitly gated on native binary availability (Gate G-04). Baseband Guard is marked `Not Applicable` on virtual targets without cellular hardware (Family 9). The system refuses launch rather than fabricating success or falling back silently. (Result: PASS)

### Principle VIII: Authorized Isolated Environments & Reproducible Provenance

- **Pre-Phase 0 Research Gate**: Assessed guest privilege containment and explicit authorization requirements for destructive actions. (Result: PASS)
- **Post-Phase 1 Design Gate**: All guest privilege escalations (KernelSU Next, custom kernels, module injection) are strictly contained within virtual guest environments with zero modification of host security posture. Destructive actions (wipes, deletions, kernel swaps, baseline restores) generate a `MutationProposal` with a SHA-256 digest and require explicit `--authorize sha256:<digest>` or interactive confirmation naming affected resources. Full environment provenance (kernel build IDs, image digests, toolchain versions, command options) is recorded in immutable `ExperimentProvenanceRecord` files. (Result: PASS)

## Project Structure

### Documentation (this feature)

```text
specs/001-add-android-research-backends/
├── plan.md              # This file (Phase 1 execution plan)
├── research.md          # Phase 0 research & architectural decisions
├── data-model.md        # Phase 1 data model, entities & state transitions
├── quickstart.md        # Phase 1 walkthrough scenarios & operational validation
├── contracts/           # Phase 1 interfaces & machine-readable schemas
│   ├── cli.md           # CLI commands, arguments, and exit code specifications
│   └── research.schema.json # JSON Schema for output envelopes and models
└── tasks.md             # Phase 2 work breakdown & task execution graph
```

### Source Code (repository root)

```text
src/
├── main.rs                      # Binary entrypoint, subcommand routing, --check preflight
├── lib.rs                       # Library root, module exports
├── cli/                         # Headless CLI automation & command parsing
│   ├── mod.rs                   # CLI hierarchy and clap definitions (emu research ...)
│   ├── backend.rs               # Backend capability discovery and preflight commands
│   ├── device.rs                # Device lifecycle, creation, registration, and deletion
│   ├── profile.rs               # Declarative profile management and dry-run proposals
│   ├── artifact.rs              # Artifact registration, staging, and digest verification
│   ├── tool.rs                  # Toolchain deployment, inspection, and removal
│   ├── experiment.rs            # Scoped hook execution, tracing, and container testing
│   ├── baseline.rs              # Baseline capture, verification, and recovery
│   ├── operation.rs             # Operation status, event streaming, and cancellation
│   ├── worker.rs                # Private worker process entrypoint (emu __worker)
│   └── envelope.rs              # Standard JSON output and JSONL log streaming envelopes
├── research/                    # Android research domain engine & coordination
│   ├── mod.rs                   # Research service facade and coordinator
│   ├── coordinator.rs           # Operation dispatcher, proposal generator, and worker spawner
│   ├── reconciler.rs            # Operation state reconciliation and crash recovery
│   ├── provenance.rs            # Experiment provenance recording and validation
│   ├── capabilities.rs          # 9-family capability catalog and conflict resolution
│   └── toolchain/               # Research toolchain integrations
│       ├── mod.rs               # Toolchain manager trait and shared types
│       ├── kernelsu.rs          # KernelSU Next v3.3.0 driver probe and management
│       ├── frida.rs             # Frida 17.18.0 daemon supervision and telemetry probe
│       ├── vector.rs            # Vector v2.2 + NeoZygisk v2.4 Xposed framework mediation
│       └── extended.rs          # SUSFS, NoMount/Mountify, BBR, eBPF, NTSync, Droidspaces
├── persistence/                 # Transactional filesystem persistence & locking
│   ├── mod.rs                   # Storage engine interface
│   ├── paths.rs                 # dirs::data_local_dir()/emu/research path resolution
│   ├── atomic.rs                # Staged atomic writes (.tmp + fsync + rename)
│   ├── lock.rs                  # Cross-process advisory file locking via fs4
│   ├── devices.rs               # ResearchDevice repository
│   ├── profiles.rs              # ResearchProfile repository
│   ├── artifacts.rs             # ArtifactPackage repository and staging
│   ├── baselines.rs             # RecoveryBaseline repository
│   ├── operations.rs            # OperationRecord journal and event log streaming
│   └── proposals.rs             # MutationProposal storage and authorization digest matching
├── managers/                    # Virtualization backend device managers
│   ├── mod.rs                   # Manager module declarations
│   ├── common.rs                # DeviceManager trait and common interfaces
│   ├── mock.rs                  # Mock backend implementations
│   ├── android/                 # Standard Android AVD manager (untouched baseline)
│   ├── ios/                     # Standard iOS Simulator manager (untouched baseline)
│   └── cuttlefish/              # AOSP Cuttlefish virtualization backend
│       ├── mod.rs               # CuttlefishManager implementing DeviceManager
│       ├── discovery.rs         # Instance discovery from isolated runtime roots
│       ├── lifecycle.rs         # launch_cvd, stop_cvd, and powerwash execution
│       ├── details.rs           # Cuttlefish runtime properties and status inspection
│       └── config.rs            # Cuttlefish runtime configuration and port mapping
├── models/                      # Domain models and data types
│   ├── mod.rs                   # Model exports
│   ├── platform.rs              # Platform types
│   ├── device.rs                # Device trait and existing models
│   ├── error.rs                 # Error types and domain error enums
│   └── research/                # Research domain entities and value objects
│       ├── mod.rs               # Research model exports
│       ├── device.rs            # ResearchDevice, ResearchDeviceId, DeviceLifecycleState
│       ├── backend.rs           # BackendCapabilityProfile, BinaryPrerequisite, Entitlements
│       ├── profile.rs           # ResearchProfile, DesiredToolState, ApplicationScope
│       ├── artifact.rs          # ArtifactPackage, ArtifactKind, Sha256Digest
│       ├── tool.rs              # ResearchToolState, ObservedToolStatus, ToolFlavor
│       ├── baseline.rs          # RecoveryBaseline, BaselineDiskArtifact
│       ├── operation.rs         # OperationRecord, OperationState, OperationPhase, ExitCode
│       └── proposal.rs          # MutationProposal, ResourceTarget, AuthorizationSignature
├── app/                         # Interactive TUI application runtime & event loop
│   ├── mod.rs                   # App struct, event processing, background workers
│   ├── state/                   # UI state models and active selections
│   └── research/                # TUI research panels and interaction handlers
│       ├── mod.rs               # Research panel integration
│       ├── device_table.rs      # Dual-backend device listing with badges
│       ├── tool_status.rs       # Toolchain inspection and telemetry widgets
│       └── dialogs.rs           # Destructive action authorization modals
├── ui/                          # Terminal rendering and widgets
│   ├── panels/                  # TUI UI panels (devices, details, logs, commands)
│   ├── dialogs/                 # Confirmation, notification, and creation dialogs
│   └── widgets.rs               # Custom Ratatui widgets (badges, status bars)
├── constants/                   # Centralized immutable constants (Principle III)
│   ├── mod.rs                   # Constants module exports
│   ├── android.rs               # Existing Android constants
│   ├── ios.rs                   # Existing iOS constants
│   ├── commands.rs              # Existing command constants
│   └── research.rs              # Research CLI commands, timeouts, paths, exit codes, limits
└── utils/                       # Shared utility primitives
    ├── mod.rs                   # Utilities exports
    ├── command.rs               # Command execution helpers
    ├── command_executor.rs      # CommandExecutor, CommandSpec, ProcessHandle, MockExecutor
    ├── validation.rs            # Input validation and SHA-256 helpers
    └── logger.rs                # Logging configuration and JSONL formatters

tests/
├── contract/                    # CLI JSON envelope and schema verification tests
│   ├── cli_exit_codes.rs        # Exit code contract validation (0, 1, 2, 3, 4, 5, 124, 130)
│   ├── json_envelope.rs         # OutputEnvelope and StreamLogEnvelope schema tests
│   └── proposal_auth.rs         # MutationProposal digest and authorization contract tests
├── integration/                 # Multi-module workflow integration tests (mock-driven)
│   ├── dual_backend_lifecycle.rs # Concurrent Emulator & Cuttlefish instance management
│   ├── toolchain_triad.rs       # KernelSU Next, Frida, and Vector joint deployment
│   ├── extended_catalog.rs      # SUSFS, NoMount, BBR, and Droidspaces profile tests
│   ├── baseline_recovery.rs     # Multi-stage baseline restore and rollback tests
│   └── crash_recovery.rs        # Worker process failure and reconciler recovery tests
└── unit/                        # Unit tests for domain models, parsers, and persistence
    ├── research_device_test.rs  # Unique identifier formatting and validation
    ├── atomic_persistence_test.rs # Staged atomic write and fs4 file lock tests
    ├── supervised_process_test.rs # CommandSpec, timeout, and cancellation tests
    └── capability_profile_test.rs # Preflight hypervisor and entitlement checks
```

**Structure Decision**:
The implementation retains a single unified binary project structure. The existing `src/managers/android/` and `src/managers/ios/` modules remain untouched to preserve baseline functional parity and zero regressions for existing standard virtual devices. The new `src/managers/cuttlefish/` manager implements the existing `DeviceManager` trait, providing clean polymorphic lifecycle dispatch. Headless automation is cleanly routed through `src/cli/`, completely bypassing `App::new()` to guarantee deterministic, non-blocking execution in CI and automated pipelines. Transactional persistence and advisory locking are isolated in `src/persistence/` using `dirs::data_local_dir()/emu/research` and `fs4`. The research engine, toolchain integrations, and operation reconciler reside in `src/research/`. Centralized constants in `src/constants/research.rs` satisfy Principle III. Standard CI unit and integration tests are isolated in `tests/` using `MockCommandExecutor` without external hypervisor or SDK dependencies, fulfilling Principle V.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because                                                                                                    |
| :-------- | :--------- | :-------------------------------------------------------------------------------------------------------------------------------------- |
| None      | N/A        | All 8 constitutional principles pass without violations or unjustified complexity. All components adhere strictly to Principles I-VIII. |
