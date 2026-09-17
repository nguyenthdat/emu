<!--
=== Sync Impact Report ===
- Version change: 1.0.0 -> 1.1.0
- Modified principles:
  * I. Trait-Based Platform Abstraction & Command Decoupling (title
    unchanged; scope broadened additively to encompass future research
    backends, hypervisors, and container tooling under DeviceManager and
    CommandExecutor boundaries)
- Added sections:
  * VI. Mobile Security Research Specialization & Grounded Scope
  * VII. Evidence-Backed Backend Capability Boundaries
  * VIII. Authorized Isolated Environments & Reproducible Provenance
- Expanded sections:
  * Platform Support & Environment Standards (added planned Android
    Emulator + Cuttlefish / KernelSU direction and Apple darwin-vm +
    Inferno complementary backend portfolio, architecture verification,
    and non-parity boundaries)
  * Development Workflow & Quality Gates (added research feature
    specifications, empirical validation gates, and clear separation between
    mock CI and controlled real-environment verification)
  * Governance (strengthened review requirement against relevant
    constitutional principles)
- Removed sections:
  * None
- Deferred placeholder TODOs:
  * None (all requirements resolved without deferred placeholders;
    template and command files remain unchanged)
- Note: This impact report is temporary scratch metadata for human review
  and pre-write validation; it MUST be removed before commit.
==========================
-->

# Emu Constitution

## Core Principles

### I. Trait-Based Platform Abstraction & Command Decoupling

All device management operations (Android AVD, iOS Simulator, and future research
backends) MUST implement the shared `DeviceManager` trait. Direct operating system
command calls to host binaries (such as `avdmanager`, `emulator`, `adb`,
`xcrun simctl`, or future hypervisor, VM, and container tooling) MUST NOT be
executed ad-hoc; they MUST route through the pluggable `CommandExecutor`
abstraction. Production implementations MUST remain cleanly separated from
platform facades so that tests can substitute mock executors without requiring
physical devices, specialized hypervisors, or host SDK installations. Lifecycle
operations across existing and future backends MUST strictly obey these trait and
executor boundaries without introducing unabstracted side channels.

_Rationale_: Decoupling device operations behind uniform traits and mocked
command pipelines guarantees cross-platform reliability, test isolation in
automated environments, and safe extensibility for future platforms and research
environments.

### II. Non-Blocking Async State & Concurrency Invariants

The application runtime and UI event loop MUST remain non-blocking. Long-running
or I/O-intensive operations—including device discovery, detail inspection, log
streaming, and system image downloads—MUST run in asynchronous background tasks
with deterministic lifecycle coordination and cancellation handles. Shared mutable
state across async tasks MUST use `tokio::sync::Mutex` or thread-safe atomic
primitives; `std::sync::Mutex` MUST NOT be held across await points. `AppState`
MUST remain the centralized single source of truth for UI coordination.

_Rationale_: Emu is an interactive terminal tool. Blocking the main thread
causes UI stutter, dropped keyboard events, and perceived unresponsiveness. Using
asynchronous primitives with centralized state ensures deterministic thread safety
and swift event response.

### III. Zero Magic Constants & Strict Idiomatic Rust Quality

Hardcoded magic numbers, hardcoded tool command names, configuration defaults,
regex patterns, and user-facing messages MUST NOT exist within operational code;
every constant MUST be centrally declared in `src/constants/`. String formatting
across `format!`, `log::*`, `println!`, `eprintln!`, `bail!`, and test assertions
MUST strictly use inline variable syntax (`format!("{variable}")`) to eliminate
uninlined format argument violations. All codebase artifacts MUST pass
`cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --check`
without exceptions.

_Rationale_: Centralizing constants avoids drift across platform parsers and UI
layouts. Enforcing strict Rust formatting and lint checks prevents subtle runtime
formatting regressions, dead code, and maintainability friction across platforms.

### IV. Responsiveness & Performance Budgets

Emu MUST satisfy explicit performance and latency budgets across user-facing
interactions:

- Terminal input polling MUST execute with an 8ms polling loop (~120 fps target)
  without input-debouncing lag during navigation.
- Cold startup time MUST remain under 150ms (typical ~104ms).
- Device details loading MUST resolve within 50ms (leveraging smart caching and
  background pre-fetching).
- Real-time device log streaming latency MUST NOT exceed 10ms.
  Any new operational workflow or inspection loop MUST NOT degrade these
  performance boundaries.

_Rationale_: High-performance responsiveness is a foundational differentiator
for a developer TUI. Strict performance ceilings protect the user experience from
accidental regressions during feature evolution.

### V. Mock-Driven Test Isolation & Falsifiable Verification

All feature implementations, manager methods, state mutations, and parser
additions MUST be covered by deterministic automated tests. Test suites MUST NOT
depend on installed host Android SDKs or Xcode runtimes; external interactions
MUST be simulated via `MockCommandExecutor`, `MockBackend`, and fixture data.
Every bug fix MUST supply an observable regression test proving failure before
the fix and success after. The test suite MUST pass reliably under
`RUST_TEST_THREADS=1 cargo test --bins --tests --features test-utils`.

_Rationale_: Requiring physical SDKs or devices renders CI flaky and locks out
contributors. Mock-driven deterministic tests verify observable contracts quickly
and reliably across diverse developer workstations and platforms.

### VI. Mobile Security Research Specialization & Grounded Scope

Emu's primary mission is managing Android and iOS virtual research environments,
with both operating system foundations (core OS, kernel, and security protocols)
and application workloads (including APK and IPA package analysis) established as
first-class research scopes. Advanced commercial platforms such as Corellium serve
as a long-term capability benchmark and architectural aspiration, rather than an
already achieved state. Features, documentation, and user interfaces MUST NOT claim
or imply current parity with commercial virtualization suites without verifiable,
empirical evidence implemented and proven in the codebase.

_Rationale_: Defining core OS, kernel, protocol, and application layers as
first-class scopes directs Emu toward meaningful security research depth, while
treating benchmarks like Corellium as an aspiration preserves scientific rigor
and prevents unverified parity claims.

### VII. Evidence-Backed Backend Capability Boundaries

Every virtualization, emulation, or simulation backend MUST truthfully declare
its operational fidelity and architectural boundaries based on empirical
evidence. Emu MUST maintain clear distinctions between user-space simulators
(e.g., iOS Simulator via `simctl`), Darwin virtual machines, full iOS guest
systems, and Android Emulator/AVD and Cuttlefish virtual devices. Running an
iOS Simulator or booting a Darwin VM MUST NOT be conflated with or represented
as full iOS or IPA execution. When a capability, kernel feature, or package
format is unsupported, experimental, or unavailable on the host environment,
Emu MUST report that status explicitly; it MUST NOT fabricate success, silently
fall back to an unrequested backend, or mask missing capabilities.

_Rationale_: Security research demands rigorous provenance and verifiable
execution guarantees. Conflating user-space simulation with true guest OS
virtualization produces flawed research conclusions and invalid security
assessments.

### VIII. Authorized Isolated Environments & Reproducible Provenance

All virtual environments and research workflows MUST operate within authorized,
strictly isolated boundaries. Guest privilege experiments (including root access,
custom kernel execution, or KernelSU instrumentation) MUST remain contained
within the guest environment and MUST NOT weaken, bypass, or compromise host
operating system protections. Destructive persistent guest data or image
operations—such as storage wipes, partition flashes, and system image
modifications—MUST require explicit user authorization that clearly identifies
the affected resources, while routine lifecycle operations and process cleanups
proceed without redundant confirmation prompts. Workflows MUST preserve
repeatable environment provenance (including base image versions, kernel
configurations, and launch parameters) to ensure security research findings
and dynamic experiments remain falsifiable and reproducible.

_Rationale_: Security analysis tools must protect host workstation integrity
and support scientific rigor. Strict containment prevents accidental host
compromise during invasive testing, clear authorization gates protect persistent
research artifacts without impeding routine process lifecycle cleanup, and
recorded provenance ensures security findings can be independently reproduced.

## Platform Support & Environment Standards

Emu supports Android AVD workflows across Linux, macOS, and Windows, while iOS
Simulator workflows MUST be strictly isolated to macOS hosts using conditional
compilation (`#[cfg(target_os = "macos")]`).

Non-interactive diagnostics (`emu --check`) MUST be maintained as a safe preflight
entry point that verifies local PATH availability and environment variables
without initializing terminal graphics or hanging on absent tools.

Error propagation MUST standardize on `anyhow::Result` with contextual messages
(`with_context`), while domain-specific failure categories MUST be modeled via
`thiserror` enums. Panicking methods such as `.unwrap()` and `.expect()` MUST NOT
appear in user-facing production code paths.

To advance toward deeper mobile security research capabilities, Emu establishes
the following architectural directions and selection standards:

- **Android Research Direction**: The planned backend portfolio combines Android
  Emulator / AVD workflows with Cuttlefish virtual devices for system and
  application research, including custom kernels and KernelSU where compatible.
- **Apple Research Direction**: Emu adopts darwin-vm (https://github.com/jprx/darwin-vm)
  plus Inferno (https://github.com/ChefKissInc/Inferno) as its intended complementary
  backend portfolio for lower-level Darwin research. Exact backend responsibilities
  and interoperability MUST be determined by empirical capability evidence;
  architectures MUST NOT assume that one backend can boot the other or assert
  unverified iOS system or `.ipa` application execution capabilities.

Host operating systems, CPU architectures (e.g., x86_64, aarch64), hypervisor
entitlements, guest OS versions, and actual kernel/firmware/application execution
support MUST be empirically verified for each backend. The project makes no
universal promises of hardware emulation parity, general `.ipa` execution,
KernelSU support, or ubiquitous platform availability. These criteria constitute
governance and backend selection requirements, NOT active implementation
acceptance claims.

## Development Workflow & Quality Gates

Every contribution MUST satisfy the repository verification pipeline before merge:

1. **Formatting**: Code MUST format cleanly via `cargo fmt`.
2. **Linting**: All targets and features MUST compile cleanly under
   `cargo clippy --all-targets --all-features -- -D warnings`.
3. **Automated Testing**: Test suites MUST pass cleanly via `cargo test --bins --tests`
   and `cargo test --features test-utils`.
4. **Commit Hygiene**: Commits MUST adhere to the Conventional Commits specification
   (`type(scope): description`) to support automated changelog generation and
   release tagging.
5. **Research Feature Specifications**: Any specification, design plan, or pull
   request introducing or modifying research backend capabilities MUST explicitly
   identify the affected research layer (core OS/kernel, security protocols, or
   application layer), detail host and hypervisor prerequisites and limitations,
   and define observable, falsifiable acceptance evidence.
6. **Empirical Validation Gates**: Claims of live backend virtualization, custom
   kernel loading, or guest device execution MUST be validated through separate
   controlled testing on compatible environments. While mock-driven tests remain
   mandatory for unit isolation and CI verification, mock-only proof MUST NOT be
   accepted as empirical evidence of real backend virtualization fidelity.
   Physical SDKs, hypervisors, and specialized hardware MUST NOT be mandated for
   the standard CI suite.

Pull requests introducing breaking modifications to `DeviceManager` or `AppState`
contracts MUST update all affected platform implementations and integration
fixtures in the same change.

## Governance

This Constitution is the supreme architectural and operational specification for
the Emu project. Its principles and constraints supersede ad-hoc PR feedback,
informal conventions, and legacy shortcuts. Every feature implementation, backend
addition, and architectural modification MUST be actively reviewed against all
applicable constitutional principles before acceptance.

Amendments to this Constitution require:

- A formal pull request detailing the problem, proposed modification, and
  migration impact on existing modules.
- A semantic version update according to SemVer rules:
  - **MAJOR**: Incompatible principle redefinitions, governance rule deletions,
    or architectural removals.
  - **MINOR**: Addition of new principles, new sections, or materially expanded
    governance rules.
  - **PATCH**: Clarifications, wording refinements, typo corrections, and
    non-semantic formatting changes.
- Compliance verification during code review and CI/CD validation before merge.

Runtime implementation guidelines and developer procedures are maintained in
`.omp/AGENTS.md` and `docs/DEVELOPMENT.md`.

**Version**: 1.1.0 | **Ratified**: 2026-09-17 | **Last Amended**: 2026-09-17
