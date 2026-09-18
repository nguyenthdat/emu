# Feature Specification: iOS Root VM and Darwin Security Research Backends

**Feature Branch**: `002-ios-root-vm`

**Created**: 2026-09-17

**Status**: Draft

**Input**: User description: "giúp tập trung thiết kế chuẩn xác kiến trúc darwin-vm, Inferno và cơ chế root VM cho macOS Apple Silicon, đồng thời bảo tồn nguyên vẹn toàn bộ đặc tả và kế hoạch Android (vừa lập rất chi tiết) để sẵn sàng triển khai ngay khi có môi trường Linux."

**Additional User Context**:

- "ios thì cũng na ná android, cũng có frida các kiểu nhưng mà hay vì có root bằng kernelsu thì ios ngâm cứu cách mà Corellium cung cấp root vm, tại mình cần truy cập sâu vào APP/thệ thống để research"

> **Notice**: The capabilities specified in this document represent a required future delivery target and development scope for the Emu project on macOS Apple Silicon. They do not constitute an active implementation claim of existing repository capabilities. Full support requires empirical verification on controlled real environments, while automated CI continues to rely on decoupled unit and mock tests. Acceptance of an iOS root VM requires a verified iOS-derived guest identity, an accessible root console with benign privileged execution, and low-level kernel/userland research capabilities—not blanket consumer GUI parity or arbitrary IPA execution. Eventual feature acceptance mandates a verified application-layer research gate on the `Inferno` backend executing a compatible owned iOS application with Frida dynamic instrumentation; a minimal root CLI environment on `darwin-vm` cannot fulfill this application research gate. References to public Corellium research concepts serve strictly as an architectural benchmark and long-term research aspiration rather than a mandatory guarantee of commercial feature parity or proprietary service integration, without claiming that all upstream versions or Corellium proprietary methods are known. Upstream references include [darwin-vm](https://github.com/jprx/darwin-vm), [Inferno](https://github.com/ChefKissInc/Inferno), and [Inferno Research Docs](https://chefkiss.dev/applehax/inferno/). Both `darwin-vm` and `Inferno` are in-scope separate environments, not combined into one engine. At least one verified iOS-derived root guest configuration per backend is required for eventual acceptance; no macOS-only guest or iOS Simulator substitute satisfies this requirement. The new Apple-first priority applies now for macOS Apple Silicon, superseding the Android 001 implementation schedule only, while preserving all eight Android 001 specification artifacts byte-for-byte intact for Linux later.

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Dual-Backend Lifecycle Management, Instance Identity, and Standard Platform Non-Regression (Priority: P1)

A security researcher on macOS Apple Silicon manages virtualized iOS and Darwin research environments across two distinct backends: `darwin-vm` (specialized in headless, minimal root shell environments, custom guest CLI programs, and low-level kernel debugging) and `Inferno` (specialized in iOS research environments supporting higher userland components including SpringBoard when configured). The researcher evaluates host compatibility via non-interactive preflight diagnostics verifying applicable acceleration and permissions (without assuming universal hypervisor entitlements on emulation fallbacks), registers guest instances from legally obtained disk images or firmware artifacts, inspects operational status, and controls standard lifecycle phases (create, inspect, start, stop, restart, delete). When multiple instances share identical display names across backends, the system unambiguously identifies and isolates each instance. Destructive operations require explicit confirmation identifying the target instance, while existing standard Android AVD and macOS iOS Simulator workflows continue to operate without regression.

**Why this priority**: Independent backend lifecycle control, unambiguous instance addressing, truthful preflight compatibility, and preservation of standard developer virtualization form the indispensable baseline for all subsequent research operations.

**Independent Test**: On macOS Apple Silicon, register two instances with the same display name "ios-sec-lab" under `darwin-vm` and `Inferno`. Verify that preflight checks report applicable acceleration and permissions, both instances operate independently without ID collisions, stopping one does not affect the other, and standard Android AVD and iOS Simulator instances continue functioning normally.

**Acceptance Scenarios**:

1. **Given** a macOS Apple Silicon host meeting applicable backend prerequisites,
   **When** the researcher creates a `darwin-vm` instance named "ios-sec-lab" and an `Inferno` instance named "ios-sec-lab",
   **Then** both instances are assigned distinct immutable identifiers and backend badges, and starting the `darwin-vm` instance boots only that instance while the `Inferno` instance remains stopped.
2. **Given** an active research guest instance,
   **When** the researcher issues a delete command,
   **Then** the system requires explicit authorization specifying the affected operation, target instance ID, backend, and persistent data paths, aborting immediately if confirmation is withheld and cleanly removing guest resources upon confirmation.
3. **Given** an environment with active Android AVD and macOS iOS Simulator configurations,
   **When** iOS research backend instances are registered, started, or stopped,
   **Then** the standard Android and iOS Simulator instances execute with zero regressions across 20 fixed lifecycle trials and zero performance degradation on baseline constitution budgets.

---

### User Story 2 - Verifiable iOS-Derived Root Proof, Guest Bootstrap Console, and Falsification Controls (Priority: P1)

A security researcher requires empirical proof of privileged root control within a booted iOS-derived guest environment on both `darwin-vm` and `Inferno` backends. The researcher establishes an interactive root console session via a prepared guest bootstrap mechanism. The researcher executes a verification workflow that proves effective root status (UID 0) by executing a harmless privileged action on a dedicated guest test fixture that is verifiably denied to non-privileged callers. The verification captures guest kernel version, boot parameters, and the output of a supplied benign guest CLI test binary, binding the verification evidence to the active boot session ID and root-relevant configuration revision. If any evidence item is missing, stale, or contradictory—or if the negative control action unexpectedly succeeds as an unprivileged user—the system marks root status as unverified. The system reports separate desired and observed privilege states, invalidating verification tokens across guest restarts, base image replacements, root-relevant security policy modifications, or lost communication sessions—without invalidating status on ordinary, authorized in-guest runtime or fixture filesystem writes—and strictly enforces that guest root privileges remain contained to the guest without granting unauthorized host elevation.

**Why this priority**: Verifiable guest root execution is the core value proposition of this feature. Without rigorous, empirical root proof and privilege boundary validation, research claims lack scientific and security validity.

**Independent Test**: Boot an iOS-derived guest on `darwin-vm` and another on `Inferno`. Run the privilege verification workflow: establish a root launch console, execute the benign privileged fixture action as root, verify success, attempt the same action as an unprivileged guest user, verify denial, inspect observed kernel state, and confirm that restarting the guest clears the active verification status until re-proven. In a negative test case, simulate an unexpected unprivileged success and verify that the system marks privilege status unverified.

**Acceptance Scenarios**:

1. **Given** a running iOS-derived guest instance on either backend,
   **When** the researcher initiates root proof verification,
   **Then** the system binds the check to the current boot session ID and root configuration revision, verifies UID 0 execution via the guest root launch console against an isolated fixture denied to unprivileged users, confirms negative control denial, records kernel observation evidence, and marks observed privilege status as verified.
2. **Given** a root verification workflow where the negative control action unexpectedly succeeds or fixture evidence is missing,
   **When** the verification evaluates evidence,
   **Then** the system marks the privilege status as unverified, emits structured diagnostic warnings, and refuses to report successful root proof.
3. **Given** a verified root guest instance,
   **When** the guest is rebooted, its base image replaced, its root-relevant security policy modified, or the communication session lost,
   **Then** the system immediately transitions the observed privilege state to unverified until a fresh verification check completes, while preserving active verification during authorized guest runtime and fixture filesystem writes.
4. **Given** an active root console session,
   **When** guest commands are executed,
   **Then** all operations remain strictly confined to the guest environment, resulting in zero unauthorized host elevation or host filesystem modification in declared containment fixtures.

---

### User Story 3 - Owned Compatible iOS Application Lifecycle, Complete Frida Runtime Control, and Target Specificity (Priority: P1)

A security researcher analyzes an owned, compatible iOS application within a supported iOS research guest environment (`Inferno`). The researcher manages the full application lifecycle (import, install, list, launch, monitor, stop, remove) without requiring App Store access or DRM decryption. The researcher manages the complete Frida dynamic instrumentation runtime lifecycle (prepare, install, configure, start, inspect, stop, remove), binding deployment status to the exact guest instance identity, boot session identifier, agent package version, and integrity hash. The researcher attaches to the target application process (or performs a controlled spawn), injects a user-supplied instrumentation script, and captures execution evidence from actual native and Objective-C application methods when the application utilizes both. Instrumentation readiness is confirmed only after receiving verifiable hook execution evidence. Probes attach strictly to the designated target process, leaving concurrent control processes unhooked. Stopping or removing Frida removes only the owned agent and session without terminating unrelated applications or corrupting guest data, and detaching a probe leaves the target application running. If an injected script throws an exception or encounters a syntax error, the system records the actual observed target status rather than guaranteeing invulnerability. Application signature validation is evaluated against declared guest security profiles, and unsupported ABIs or missing frameworks (such as minimal backend configurations) are reported explicitly without masking capabilities.

**Why this priority**: Deep mobile security research requires inspecting real application execution and runtime behaviors using industry-standard dynamic instrumentation (Frida), fulfilling the user's explicit objective of achieving Corellium-like research depth for apps on an iOS root VM.

**Independent Test**: On a supported `Inferno` iOS research guest, import a sample iOS application binary, deploy and start Frida, launch the application, attach Frida via script injection, observe hooked native and Objective-C method calls, verify that a concurrent control application remains unhooked, detach the probe verifying the app continues running, and stop and remove the Frida agent verifying target status. On a backend configuration lacking application frameworks, confirm that app installation truthfully reports missing application frameworks.

**Acceptance Scenarios**:

1. **Given** a running `Inferno` guest with application userland support,
   **When** the researcher installs and launches an owned compatible iOS application,
   **Then** the application starts, registers a valid guest process ID, and makes its sandboxed container directory available for authorized research inspection.
2. **Given** an active application process,
   **When** the researcher deploys Frida, starts the agent, and attaches a script hooking both a native C/ARM64 function and an Objective-C method,
   **Then** the system logs intercepted invocations with arguments and timestamps, confirms instrumentation readiness only after observed hook execution, leaves an identical uninstrumented control application unaffected, and detaches cleanly without terminating the application.
3. **Given** an active Frida instrumentation session,
   **When** the researcher requests stopping and removing the instrumentation runtime,
   **Then** the system detaches probes, halts the agent process, removes owned agent artifacts from the guest, and preserves unrelated guest processes and application data.
4. **Given** an injected Frida script that encounters a runtime syntax error or runtime exception,
   **When** the failure occurs,
   **Then** the system captures the diagnostic error trace, transitions the instrumentation session state to failed, and reports the observed target process status as running, terminated, faulted, or unknown as actually observed, while isolating the host and other guests and reporting any guest fault without falsely asserting VM invulnerability to arbitrary guest code.
5. **Given** a guest backend configuration lacking required iOS application frameworks,
   **When** application installation or Frida app spawn is requested,
   **Then** the system explicitly reports that the configuration lacks application-layer frameworks and refuses the operation without masking capabilities.

---

### User Story 4 - Deep System and Kernel Security Research Workflows, Controlled State Modification, and Guest Security Profiles (Priority: P2)

A security researcher investigates low-level system behaviors, privileged daemon interactions, and kernel execution states within booted research guests across both named backends. The researcher performs authorized read, write, and export operations on guest root filesystem paths (with runtime modifications executing directly inside the guest without mounting the active VM disk on the host). The researcher inspects system processes, enumerates Mach services, and attaches Frida probes to selected compatible guest system daemons on `Inferno`. On reference configurations supporting kernel debugging (`darwin-vm` minimal kernel debug configuration and `Inferno` system research configuration), the researcher pauses guest execution, inspects processor registers and memory structures, sets hardware/software breakpoints, steps through instructions, applies controlled test state modifications, and restores test state. The system strictly distinguishes between paused/stopped debugger states and guest boot failures. If a debugger client disconnects while attached to a guest, the system inspects and reports the actual observed state (paused, running, or unknown) truthfully without silently resuming or fabricating a paused assertion, and provides explicit recovery commands via both CLI and TUI. Furthermore, the system manages `GuestSecurityProfile` selection, application, and reversion with explicit guest-only relaxations (code-signing enforcement modes, sandbox profile state, and filesystem permissions) and baselines, reporting effective policies independently of root UID status without asserting unverified universal bypasses of hardware security features.

**Why this priority**: Meaningful vulnerability and systems research requires visibility into operating system daemons and low-level kernel execution, drawing inspiration from public Corellium research concepts as an architectural aspiration without claiming proprietary commercial feature parity.

**Independent Test**: On a running guest configuration with kernel debugging enabled, pause the guest, inspect CPU register and memory values, trigger a breakpoint, single-step execution, edit and restore a test register value, and resume, verifying that the guest continues normal operation. Disconnect the debugger while paused, verifying that the system reports actual paused state truthfully and provides recovery. Attach Frida to a compatible guest system daemon and capture trace logs. Apply a custom guest security profile, verify independent reporting of root access versus sandbox status, and revert to baseline. Read and write an authorized guest path directly within the guest environment, verifying persistence in runtime storage without host-side disk mounts.

**Acceptance Scenarios**:

1. **Given** an active guest instance on a reference configuration supporting kernel debugging (`darwin-vm` minimal kernel debug configuration or `Inferno` system research configuration),
   **When** the researcher issues a kernel pause command,
   **Then** guest CPU execution halts cleanly, processor registers and memory are made readable for inspection, and the system reports the guest as paused rather than crashed or failed.
2. **Given** a paused guest instance on a reference kernel debug configuration,
   **When** the researcher triggers a breakpoint, steps through instructions, modifies a test register or memory value, and issues a resume command,
   **Then** the guest executes the stepped instructions, applies and restores test state, resumes normal execution, and transitions to running status.
3. **Given** an active debugger session attached to a guest instance,
   **When** the debugger client disconnects unexpectedly or closes the session,
   **Then** the system preserves the actual execution state where observed paused without silently resuming or fabricating a paused assertion, inspects and reports the observed status truthfully as paused, running, or unknown without falsely claiming resumption, and offers explicit CLI and TUI session recovery commands.
4. **Given** an active root research session,
   **When** the researcher writes to an authorized guest system path inside the guest runtime,
   **Then** the write succeeds within the guest filesystem without requiring a host-side disk mount and without modifying any host files.
5. **Given** an active `Inferno` guest instance on a supported research configuration,
   **When** the researcher attaches Frida to a selected compatible guest system daemon,
   **Then** the system captures trace events from system service calls with an uninstrumented control daemon remaining unaffected, and detaches cleanly without daemon termination.
6. **Given** a guest security profile configuration,
   **When** the researcher applies or reverts a declared profile,
   **Then** the system applies guest-only security policy relaxations or restores the verified baseline, and reports effective guest code-signing, sandbox, and filesystem protection states as separate observable properties distinct from root UID 0 status.

---

### User Story 5 - Controlled Image Preparation, Verified Disk Identity, Experimental Opt-In, and Disposable Resource Cleanup (Priority: P2)

A researcher prepares or imports base system images and firmware components for `darwin-vm` and `Inferno` using legally obtained assets. For `darwin-vm`, the system assists in preparing minimal root disk images from recovery assets, requiring explicitly scoped administrative authorization on the host strictly bounded to the target image and workspace. When mounting guest disks for preparation or userland patching on the host, the system strictly verifies the isolated device identity and unique volume identifier of the mounted disk, rejecting hardcoded path trust (such as `/Volumes/System`). All host-side image preparation operations operate exclusively on the target guest image, preserving host Sealed System Volume (SSV) and System Integrity Protection (SIP) untouched. Unverified or unknown compatibility configurations are marked as experimental and require explicit user opt-in alongside an available verified baseline, while known corrupt or incompatible images are always blocked. In unattended automation mode, preparation commands refuse execution with an actionable error rather than prompting interactively if authorization is absent. If an image preparation task fails, the system releases only owned disposable resources at a verified safe boundary, reporting committed guest changes and pending unmounts without promising impossible rollbacks. If cleanup itself encounters an error, residual mounts and scratch files are reported explicitly, and the image is marked ineligible for ready status.

**Why this priority**: Secure, repeatable image handling ensures that research environments start from a known-good baseline without exposing the host system to unauthorized privilege escalation, filesystem corruption, or invalid configurations.

**Independent Test**: Provide valid, corrupted, experimental, and mismatched firmware images to the image preparation workflow. Confirm that valid images compute and record provenance metadata, mounted guest disk identities are verified independently of host system paths, corrupted images are rejected before registration, experimental images require opt-in and verified baseline, administrative authorization is requested with explicit scope, unattended runs without credentials exit with error code, and partial preparation failures clean up disposable resources reporting residual state.

**Acceptance Scenarios**:

1. **Given** a legally obtained Apple firmware or recovery artifact,
   **When** the researcher registers the image,
   **Then** the system computes its cryptographic hash, verifies format compatibility against the target backend (`darwin-vm` or `Inferno`), and records immutable provenance metadata (origin, hash, build identity).
2. **Given** an image artifact with unverified or experimental compatibility,
   **When** registration is attempted without explicit opt-in or without a verified baseline available,
   **Then** the system rejects registration, explains the missing baseline or required opt-in flag, and prevents hypervisor launch.
3. **Given** a guest disk preparation process that mounts a guest image on the host for filesystem patching,
   **When** the mount occurs,
   **Then** the system verifies the exact device node and volume identifier of the mounted guest disk, applies patches exclusively within the mounted guest boundary without touching host SSV or SIP, and unmounts the disk cleanly upon completion.
4. **Given** an image preparation workflow requiring temporary elevated host privileges,
   **When** executed in unattended automation mode without pre-authorized credentials,
   **Then** the system refuses the operation, returns a non-zero exit code with structured diagnostics, and leaves host filesystem and security settings untouched.
5. **Given** an image preparation workflow that fails mid-process,
   **When** the failure occurs,
   **Then** the system releases all owned disposable temporary files and virtual disk mounts up to verified safe boundaries, reports any committed artifacts or unmount failures explicitly, and marks the image ineligible for ready status.

---

### User Story 6 - Isolated Local Companion VM Orchestration for Restore and Live Dependencies (Priority: P3)

When operating the `Inferno` backend, certain firmware restoration, ramdisk preparation, or USB-over-IP bridging workflows require specialized Linux-based restore utilities. The system provides a controlled, local companion virtual machine on the same macOS host that runs these utilities without requiring a separate physical Linux machine. The lifecycle, resource consumption (CPU, memory, disk), and network boundaries of the companion VM are strictly managed and bound to the active restore workflow or live guest session dependencies. Communication is strictly restricted to same-host access-controlled channels with no external control or debug endpoints. Companion instances and associated helper resources (network forwarders, virtual bridge endpoints) are safely terminated ONLY when no active restore workflow remains AND no live guest dependency requires the companion. Cancellation requests for a shared companion must not terminate resources required by other live dependent guests. If a restore workflow completes but an active guest session still depends on helper services, the companion is retained and the dependency is reported. Caller wait timeouts or observation window closes do not terminate continuing companion processes.

**Why this priority**: Enabling advanced `Inferno` restore workflows on macOS Apple Silicon without external hardware dependencies requires local helper virtualization that remains transparent, bounded, and resource-controlled.

**Independent Test**: Initiate an `Inferno` guest setup requiring companion restore services on the local macOS host. Verify that the companion VM launches with declared resource limits, performs its helper tasks over same-host access-controlled channels, reports progress to the researcher, remains running when a dependent guest session is active, and shuts down completely only after all dependent workflows conclude without orphaned processes.

**Acceptance Scenarios**:

1. **Given** an `Inferno` guest preparation workflow requiring restore utilities,
   **When** the companion workflow is triggered,
   **Then** the system starts an isolated local companion VM on the macOS host with declared resource limits, establishes same-host access-controlled communication, and monitors helper progress.
2. **Given** an active companion VM session where the restore workflow finishes while a live guest instance remains dependent on companion services,
   **When** the restore phase concludes,
   **Then** the system retains the companion VM, reports its live dependent binding, and continues providing helper services.
3. **Given** an active companion VM session with no remaining active workflows and no live guest dependencies,
   **When** the session cleanup is evaluated or verified safe cancellation occurs without other live dependents,
   **Then** the system terminates the companion VM, cleans up any associated forwarders or temporary bridge endpoints, releases allocated host resources, and reports truthful completion status.
4. **Given** an active companion VM,
   **When** inspect commands are issued or caller wait deadlines elapse,
   **Then** the system reports companion operational status, resource consumption, and lifecycle binding, leaving continuing background helper processes running without termination.

---

### User Story 7 - Reproducible Research Profiles, Immutable Experiment Records, and Baseline State Recovery (Priority: P4)

A researcher conducts dynamic security experiments, modifying guest configurations, deploying instrumentation scripts, and testing kernel parameters. The system captures the complete environment state into a versioned, portable research experiment profile containing image hashes, kernel boot arguments, applied patches, guest test fixture configurations, and instrumentation manifests, automatically excluding all confidential host credentials, private keys, and host environment secrets. Guest container data, logs, or filesystem artifacts are included only when explicitly selected and authorized by the researcher. Execution trials are permanently captured in an immutable `ExperimentRecord` retaining backend identifiers, upstream builds, checksums, session parameters, observed root and kernel results, and errors, ensuring history cannot be rewritten. The researcher restores an altered or unstable guest back to a verified RecoveryBaseline within a predeclared, configuration-specific deadline recorded before measurement. A second researcher imports the exported profile onto an independent Apple Silicon workstation (or fresh instance on the same workstation), re-validates compatible artifact components, and executes a fresh trial. Upon import, configuration metadata is validated but privilege and instrumentation status remain unverified until fresh guest execution confirms active capabilities. If a requested baseline is missing, corrupt, or unverified, the system safely refuses restoration without automatic deletion or recreation.

**Why this priority**: Scientific rigor and reproducible vulnerability analysis require immutable provenance and reliable baseline recovery so researchers can validate findings across independent environments without secret leakage.

**Independent Test**: Configure a customized guest environment with specific boot arguments, application artifacts, and Frida scripts. Export the research profile as portable data, verify that no host secrets are included, import the profile onto a fresh instance on the same workstation, confirm that imported privilege status is unverified until booted and proven, deliberately corrupt the guest filesystem, execute a profile restore verifying recovery to baseline within the declared deadline, and verify that immutable experiment records retain trial history.

**Acceptance Scenarios**:

1. **Given** an active guest research configuration,
   **When** the researcher exports a research profile,
   **Then** the system produces portable data containing the configuration manifest, boot arguments, artifact checksums, instrumentation scripts, and fixture specifications, automatically excluding all host credentials and secrets, and including guest data only upon explicit authorization.
2. **Given** an executed research trial,
   **When** trial execution completes, fails, or cancels,
   **Then** the system generates an immutable `ExperimentRecord` capturing input parameters, configuration revisions, observed evidence, errors, and timestamps, preserving historical records from subsequent modifications.
3. **Given** a guest instance with corrupted userland state,
   **When** the researcher triggers a baseline recovery from a verified RecoveryBaseline,
   **Then** the system restores the guest to its clean baseline state within the predeclared deadline, requiring explicit confirmation if persistent guest data will be overwritten.
4. **Given** an exported profile,
   **When** imported on a fresh guest instance,
   **Then** the system validates local artifact hashes against the profile manifest, registers the guest with validated configuration, and marks privilege and instrumentation readiness as unverified until proven during fresh live execution.

---

### User Story 8 - Complete Automation CLI, Interactive TUI Parity, Bounded Waiting, and Cancellation Truth (Priority: P5)

A researcher interacts with Emu through both headless CI/scripted automation and an interactive terminal user interface (TUI). Every research backend capability—including preflight checks, lifecycle management, root verification, application deployment, Frida instrumentation, kernel debugging, companion inspection, profile export, and state recovery—is fully accessible through non-interactive CLI commands with structured, machine-parseable output (stdout) and separated diagnostic logs (stderr). The interactive TUI surfaces the identical capabilities with non-blocking navigation, immediate cancel responsiveness, and clear status badges. Neither surface possesses unique operational capabilities over the other, and no manual TUI steps are required when CLI automation is invoked. When caller wait deadlines elapse, the system reports the actual known operational state without falsely claiming task cessation. When cancellation is requested, the system transitions status to cancellation-pending, which remains if wait deadlines expire before verified cessation, confirming cessation only after reaching a verified safe boundary. Terminating an observation window or closing the UI does not cancel underlying guest tasks. Deterministic desired-state mutations apply without extra side effects, and each imperative action executes exactly once without automatic retries.

**Why this priority**: Operational surface parity ensures seamless transitions between rapid interactive exploration and automated research pipelines, while non-blocking UI behavior maintains researcher productivity.

**Independent Test**: Execute the entire lifecycle, root verification, application installation, Frida attachment, and profile export sequence through non-interactive CLI commands using structured output mode. Then perform the identical sequence within the interactive TUI. Measure TUI navigation latency during background guest operations, test cancellation during long-running tasks, and verify that canceling long-running operations acknowledges immediately without UI freezing.

**Acceptance Scenarios**:

1. **Given** any supported research operation,
   **When** invoked via non-interactive CLI,
   **Then** the command outputs machine-parseable data on standard output with diagnostics separated on standard error, returning exit code 0 for success (including read-only queries of unavailable capabilities reporting structured status) and distinct non-zero codes for operational failures, unsupported actions, timeouts, and cancellations.
2. **Given** a long-running guest operation in progress,
   **When** a caller wait deadline elapses before completion,
   **Then** the system returns a non-zero exit code reporting actual operational state as continuing, stopped, or unknown, without terminating the underlying guest task.
3. **Given** an active guest mutation,
   **When** the researcher issues an explicit cancellation request,
   **Then** the system immediately transitions status to cancellation-pending without promising instantaneous task termination; if the caller wait deadline expires before reaching a clean cessation boundary, status remains cancellation-pending, and upon reaching a verified safe boundary the system confirms cessation and outputs a structured log of any partially committed changes.
4. **Given** an active guest operation observed through the CLI or TUI,
   **When** the researcher closes the observation window or disconnects the client,
   **Then** the system stops local log streaming while the underlying guest operation continues uninterrupted in the background.
5. **Given** an active guest operation in progress,
   **When** the researcher navigates or requests cancellation in the interactive TUI,
   **Then** UI navigation responds within 100ms, cancellation request is acknowledged within 200ms, and the UI reports truthful operational state without thread freezing.

---

### Edge Cases

- **Host Hypervisor and Acceleration Fallbacks**: If executed on a host lacking hardware virtualization extensions, preflight commands report available acceleration status truthfully (including software emulation modes if supported) without falsely asserting missing hypervisor entitlements.
- **Identical Display Names Across Backends**: When instances share identical display names across `darwin-vm` and `Inferno`, the system rejects ambiguous commands and requires explicit backend or ID qualification.
- **Corrupted, Truncated, or Incompatible Images**: If a registered disk image or firmware file has an invalid checksum, corrupted partition table, or unsupported format, registration and boot are aborted immediately before hypervisor launch.
- **Unverified Mount Paths and Host Isolation**: If image preparation or patching detects an unverified mount target or attempts to interact with host system volumes, the operation aborts immediately to protect host integrity.
- **Unattended Elevation Failure**: If an image preparation command requires administrative privileges during unattended CLI execution without the required explicit authorization, the process terminates immediately with an exit code indicating missing authorization without hanging on interactive prompts.
- **Guest Root Privilege Containment**: Guest root grants no host authority. Emu MUST NOT expose raw host disks or unapproved host resources to guest research operations; declared containment and unauthorized-access fixtures must show no unauthorized host modification. These checks do not establish immunity to every possible emulator escape.
- **Root and Frida Invalidation Boundaries**: Active root verification and Frida attachment are invalidated upon guest reboot, crash, base image replacement, root-relevant security policy or kernel modifications, target process termination, or lost session connection. Normal runtime filesystem writes by guest processes or test fixtures do not invalidate active verification.
- **Frida Version Mismatch and Script Errors**: If a deployed Frida agent version mismatches the host toolchain or if an injected script encounters a syntax error or causes target process termination, the system transitions the instrumentation session to failed, captures diagnostic error traces, and reports the observed process state truthfully (running, terminated, faulted, or unknown). The host and concurrent guest instances remain isolated, and any guest-level fault is recorded without asserting that arbitrary guest scripts cannot crash a guest VM.
- **App Missing Frameworks on Minimal Backends**: If an iOS application is targeted at a guest configuration lacking application frameworks, the system reports that userland app frameworks are unavailable and refuses execution without failing silently.
- **Kernel Debugger Disconnection**: If a debugger client disconnects abruptly while attached to a guest, the system preserves the observed execution state (including paused states without silent resumption), inspects and reports status accurately as paused, running, or unknown without fabricating state or claiming resumption, and allows explicit CLI/TUI resumption or safe session recovery.
- **Companion VM Failure and Lifecycle Bounds**: If a local companion VM encounters a terminal failure or verified safe cancellation, disposable helper resources (companion process, virtual disk attachments, forwarders) are torn down cleanly ONLY when no live dependent guest requires helper services. Caller wait timeouts or observation window closes NEVER tear down continuing companion tasks, and any residual helper resources or uncertain states are reported truthfully as pending or unknown.
- **Concurrent Conflicting Mutations**: If simultaneous lifecycle or configuration mutations are issued against the same guest instance, the system rejects conflicting operations with a concurrency conflict error while preserving instance integrity and isolating other concurrent guests.
- **Caller Timeout vs. Cancellation**: If a caller-specified wait deadline elapses before a long-running guest boot or restore finishes, the system reports the current in-progress state without falsely claiming task cessation; explicit cancellation halts operations at the nearest verified safe boundary.
- **Cross-Backend State Transfer**: While raw disk snapshots cannot be converted across backends, portable experiment profiles allow shared compatible configuration parameters and benign binaries to be revalidated and reused.
- **Idempotent Profile Re-Application**: If a researcher applies a desired research profile that matches the current active guest state exactly, the system confirms satisfaction without redundant re-preparation or unnecessary rebooting, and never performs automatic mutation retries on failed steps.
- **Host System Protection Invariance**: Emu never alters System Integrity Protection (SIP), Sealed System Volume (SSV), or host boot arguments; all guest research modifications remain strictly confined to guest disk artifacts.

---

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: System MUST support macOS Apple Silicon (`aarch64`) as the primary host virtualization platform for iOS and Darwin security research.
- **FR-002**: System MUST provide non-interactive preflight diagnostics verifying host CPU architecture, applicable hardware acceleration, and environment permissions without launching graphical interfaces.
- **FR-003**: System MUST maintain strict architectural and operational separation between `darwin-vm` and `Inferno` backends, treating them as independent research environments rather than interchangeable engines.
- **FR-004**: System MUST preserve full functional behavioral parity and zero regressions for existing legacy Android AVD and macOS iOS Simulator workflows, while preserving exact byte-for-byte integrity for all eight Android 001 specification artifacts (specification, plan, research, data model, quickstart, two contract files, and requirements checklist) without imposing source-code byte constraints.
- **FR-005**: System MUST assign an immutable, globally unique identifier to every guest instance upon creation or registration, independent of user-assigned display names.
- **FR-006**: System MUST support standard guest lifecycle operations (create/register, inspect, start, stop, restart, delete) across both `darwin-vm` and `Inferno` backends.
- **FR-007**: System MUST clearly disambiguate guest instances sharing identical display names across different backends in all views, logs, and command invocations.
- **FR-008**: System MUST require explicit confirmation identifying the affected operation, target instance identifier, backend type, and affected persistent images, guest data partitions, or workspace paths prior to executing destructive operations (including disk wipes, instance deletions, destructive baseline restorations, and root security policy modifications), while allowing routine lifecycle operations (start, stop, restart) to proceed without redundant prompts when persistent resources are not destroyed.
- **FR-009**: System MUST prevent raw disk image or snapshot reuse across different backends, while allowing compatible profile metadata to be revalidated upon import.
- **FR-010**: System MUST support verifying effective root privilege (UID 0) within booted iOS-derived guest environments on both `darwin-vm` and `Inferno`, explicitly correlating iOS-derived guest build identity and input image artifact checksums with runtime boot session identifiers.
- **FR-011**: System MUST establish interactive root console access via a prepared guest bootstrap mechanism without reliance on external commercial jailbreak tools or closed exploits.
- **FR-012**: System MUST perform root verification by executing a harmless privileged action against a dedicated guest test fixture that is verifiably denied to non-privileged callers.
- **FR-013**: System MUST observe and record guest kernel version, boot parameters, and execution output of supplied benign guest CLI test binaries during verification.
- **FR-014**: System MUST report desired privilege state and observed privilege state as distinct attributes in all status queries, and MUST mark observed root privilege status as unverified whenever evidence is missing, stale, contradictory, or when a negative control action unexpectedly succeeds.
- **FR-015**: System MUST automatically invalidate verified root status upon guest reboot, crash, base image replacement, root-relevant security policy or kernel parameter modification, or lost session connection, while preserving active verification during authorized guest runtime and test-fixture filesystem operations.
- **FR-016**: System MUST support importing, installing, listing, launching, inspecting, stopping, and removing owned compatible iOS applications on supported research guests (`Inferno`).
- **FR-017**: System MUST support the complete lifecycle of Frida instrumentation agents (prepare, install, configure, start, inspect, stop, remove), binding deployment status to the exact guest instance identity, boot session identifier, agent package version, and integrity hash.
- **FR-018**: System MUST support attaching Frida to running application targets and controlled process spawning with user-supplied instrumentation scripts, requiring actual hook evidence before reporting instrumentation ready.
- **FR-019**: System MUST automatically invalidate Frida deployment readiness and active instrumentation sessions upon guest reboot, base image replacement, root-relevant security policy modification, target process termination, or lost session connection.
- **FR-020**: System MUST support capturing dynamic hook evidence from both native C/ARM64 functions and Objective-C methods when utilized by the target application.
- **FR-021**: System MUST verify that instrumentation probes attach exclusively to the designated target process, leaving concurrent control processes unhooked.
- **FR-022**: System MUST validate application ABI compatibility, code signatures, and required frameworks against the declared guest security profile prior to launch, reporting structured errors on failure.
- **FR-023**: System MUST report application-layer and framework capabilities as unavailable when queried on backend configurations lacking application frameworks, refusing app deployment without failing silently.
- **FR-024**: System MUST support reading and exporting files from authorized sandboxed application containers and scoped container modifications for research purposes only when explicitly selected and authorized by the researcher.
- **FR-025**: System MUST support authorized read, write, and export operations within guest root filesystem paths during live execution, executing runtime modifications entirely within the guest without requiring host-side disk mounts.
- **FR-026**: System MUST support enumerating guest processes, querying Mach services, and attaching instrumentation to selected compatible guest system daemons on `Inferno`.
- **FR-027**: System MUST support kernel debugging operations—including pause, resume, breakpoint management, single-stepping, inspecting registers and memory, controlled test state modification, and state restoration—on reference configurations supporting kernel debug interfaces across both named backends.
- **FR-028**: System MUST clearly distinguish between paused/stopped debugger states and guest boot or runtime failures, preventing false failure reports during active debug sessions.
- **FR-029**: System MUST inspect and report actual operational state truthfully (paused, running, or unknown) when a kernel debugger session disconnects while a guest is paused, providing safe recovery prompts via CLI and TUI without falsely asserting resumption.
- **FR-030**: System MUST support `GuestSecurityProfile` selection, application, and reversion with explicit guest-only policy relaxations, reporting effective guest security policies independently of root UID 0 status.
- **FR-031**: System MUST compute, record, and verify cryptographic hashes and build identities for all imported or prepared system images, kernels, and firmware components.
- **FR-032**: System MUST reject corrupt, truncated, or incompatible images prior to hypervisor initialization, treating unverified configurations as experimental requiring explicit opt-in and an available verified baseline.
- **FR-033**: System MUST verify the unique device node and volume identity of any mounted guest disk during host-side preparation, strictly prohibiting reliance on hardcoded host paths such as `/Volumes/System`.
- **FR-034**: System MUST ensure all host-side image preparation and patching operates exclusively on the target guest disk image, preserving host Sealed System Volume (SSV) and System Integrity Protection (SIP) untouched.
- **FR-035**: System MUST require explicit administrative authorization for host-side image preparation tasks requiring elevated privileges, refusing execution with structured errors during unattended automation if authorization is absent.
- **FR-036**: System MUST perform multi-stage cleanup of owned disposable resources (temporary virtual disks, mount points, scratch files) at verified safe boundaries when image preparation fails or cancels, reporting committed artifacts and unmount failures.
- **FR-037**: System MUST support an isolated, local companion virtual machine on the same macOS host to execute specialized restore and setup utilities required by `Inferno`, without requiring separate physical Linux hardware.
- **FR-038**: System MUST terminate companion VM and helper resources ONLY when no active restore workflow remains AND no live guest dependency requires the companion; cancellation requests for a shared companion MUST NOT terminate helper resources still utilized by other live dependent guests, requiring explicit authorized termination of dependent guest sessions before companion shutdown.
- **FR-039**: System MUST isolate all research control, console, kernel debug, Frida instrumentation, and companion VM communication channels to same-host access-controlled endpoints with zero external network exposure, denying unauthorized local clients and prohibiting unauthenticated public listening endpoints.
- **FR-040**: System MUST support importing, configuring, applying, and exporting versioned portable research experiment profiles—capturing base image hashes, boot parameters, kernel patches, application artifacts, and instrumentation manifests—and MUST revalidate local artifact checksums before instantiating or modifying guest environments.
- **FR-041**: System MUST automatically exclude all host credentials, private keys, and host environment secrets from exported research profiles, while exporting guest container data, logs, and filesystem artifacts only when explicitly selected and authorized by the researcher.
- **FR-042**: System MUST record every execution trial in an immutable `ExperimentRecord` capturing backend identities, upstream builds, checksums, session parameters, observed evidence, errors, and timestamps.
- **FR-043**: System MUST support restoring a guest instance to a verified RecoveryBaseline within a predeclared, configuration-specific deadline; if the baseline is absent, corrupt, or unverified, the system MUST reject restoration without performing automatic instance deletion or recreation, requiring explicitly scoped authorization prior to overwriting any persistent guest state.
- **FR-044**: System MUST expose all research lifecycle, verification, application management, instrumentation, kernel debugging, companion orchestration, and profile operations through non-interactive CLI commands with machine-parseable structured output (stdout) and separated diagnostics (stderr).
- **FR-045**: System MUST return exit code 0 for successful operations (including read-only queries of unavailable capabilities reporting structured status) and distinct non-zero exit codes for validation failures, unsupported actions, timeouts, and cancellations.
- **FR-046**: System MUST distinguish between cancellation-pending and confirmed cessation, confirming cessation only after reaching a verified safe boundary, and ensure stopping observation does not cancel guest operations.
- **FR-047**: System MUST execute each explicit imperative action exactly once per authorized invocation without silent suppression or automatic retry on failure, while ensuring that applying an identical desired research profile is idempotent and introduces no redundant re-preparation or extra side effects.
- **FR-048**: System MUST ensure full functional parity between CLI automation and interactive TUI surfaces, maintaining non-blocking UI responsiveness (navigation <= 100ms, cancellation acknowledgment <= 200ms) with zero TUI-only operational capabilities.

---

### Key Entities

- **ResearchGuestInstance**: Represents a virtualized Darwin or iOS-derived research environment. Attributes include unique identifier, display name, backend type (`darwin-vm` or `Inferno`), lifecycle state (stopped, booting, running, paused, recovering, error), base image reference, boot configuration, active session identifier, and last-verified timestamp.
- **BackendCapabilityProfile**: Defines the architectural capabilities, constraints, and supported operational layers for a specific backend. Attributes include backend type, supported guest OS families, headless console support, graphical display capability, companion VM requirement, application framework support, and supported debugging interfaces.
- **ResearchImageArtifact**: Represents a verified firmware image, kernel cache, ramdisk, or root filesystem image. Attributes include content hash, origin metadata, target backend compatibility, build version identity, verified device node / volume identifier, patch provenance, and validation status.
- **RootProofEvidence**: Encapsulates empirical evidence of guest privilege verification. Attributes include guest instance identifier, boot session identifier, iOS-derived guest build identity, input artifact checksums, configuration revision hash, verified UID, fixture execution outcome, negative control denial proof, observed kernel version and boot arguments, benign binary execution digest, and verification timestamp.
- **ApplicationArtifact**: Represents an owned, compatible iOS application package managed for research. Attributes include application identifier, bundle name, binary architecture, code signature identity, container path, sandbox entitlement manifest, and deployment status.
- **InstrumentationSession**: Represents an active dynamic instrumentation workflow (such as Frida). Attributes include session identifier, guest instance identifier, boot session identifier, target application identity and process ID, agent package version, agent integrity hash, injected script hashes, hook status, capture log stream, and attachment state.
- **GuestSecurityProfile**: Represents the observable runtime security configuration of a research guest. Attributes include profile identifier, code-signing enforcement mode, AMFI relaxation status, sandbox profile state, filesystem mount permissions (read-only vs read-write), and kernel privilege observation data.
- **CompanionEnvironment**: Represents a local helper virtual machine on the macOS host supporting restore and setup operations. Attributes include companion identifier, parent guest instance identifier, lifecycle state, assigned resource limits, access-controlled communication endpoint, registered helper forwarders, and active operation status.
- **ResearchExperimentProfile**: An editable, versioned specification of a research setup. Attributes include profile identifier, target backend, base image reference hash, kernel boot parameters, application artifacts, instrumentation scripts, guest fixture definitions, and creation timestamp.
- **ExperimentRecord**: An immutable, historical snapshot of an executed experiment. Attributes include record identifier, profile identifier, backend type, upstream build identity, artifact checksums, execution timestamps, observed RootProofEvidence, InstrumentationSession logs, and environmental telemetry.
- **RecoveryBaseline**: A verified, immutable reference state used for guest rollback. Attributes include baseline identifier, instance identifier, base disk checksum, kernel configuration hash, and declared recovery deadline.

---

### Reference Acceptance Set

Validation of this specification requires executing a finite, deterministic reference evaluation cohort on a single macOS Apple Silicon host workstation:

- **Cohort Composition**: 4 distinct guest instances (2 under `darwin-vm` and 2 under `Inferno`), executable sequentially to manage host compute and memory footprints. All percentage-based success criteria are strictly scoped to this 4-guest cohort.
- **Reference Configurations**: At least one verified iOS-derived root and kernel debug configuration per backend; and at least one reference `Inferno` iOS userland guest configuration supporting application execution and Frida dynamic instrumentation.
- **Execution Repetition**: Root proof re-verification executed across 3 consecutive cold boots per root instance; sample iOS application native and Objective-C hook validation executed across 3 consecutive trials on the reference `Inferno` guest; and system daemon Frida tracing executed across 3 consecutive trials.
- **Complete Operational Coverage**: Every supported operational family (lifecycle, privilege verification, app deployment, Frida instrumentation, kernel debugging, image preparation, companion orchestration, profile export/import, and baseline recovery) must be exercised through both CLI and TUI surfaces within this cohort.
- **Critical Negative Cases**: The evaluation suite must execute each of the following 16 negative/falsification cases at least once per applicable backend within the cohort. Every separately named variant within a case must be exercised; satisfying one alternative does not cover the others:
  1. Missing prerequisites / unaccelerated host reporting limits, and read-only unavailable capability query reporting structured success
  2. Ambiguous instance name collision across backends
  3. Corrupt, truncated, or format-incompatible image artifact rejection
  4. Unverified experimental image missing baseline rejection, and experimental image missing required opt-in flag rejection
  5. Denied administrative authorization in unattended automation, and unauthorized local client access denied on protected research endpoints
  6. Incomplete root proof / unexpected negative control pass
  7. Stale root/Frida proof invalidation on reboot or configuration change
  8. Unsafe mount path detection and cleanup reporting residual mounts
  9. Helper failure / live-dependent timeout handling
  10. Missing or mismatched profile import / cross-backend raw state rejection
  11. Application ABI, signature, or missing framework rejection
  12. Frida version mismatch, script syntax error, or target process exit
  13. Debugger disconnection reporting actual paused, running, or unknown state truthfully without silent resumption
  14. Concurrent conflicting mutation rejection, protecting other concurrent guests from interference
  15. Wait timeout reporting actual state before commit and after partial commit + cancellation-pending vs confirmed cessation + observation window disconnect without task cancellation
  16. Repeated application of an identical desired state returning documented successful status (code 0) without redundant mutation, alongside failed explicit imperative action returning non-zero without automatic retries
- **Frozen Baselines**: Host resources, upstream backend revisions, artifact hashes, test applications, instrumentation scripts, and finite timeout/recovery/helper cleanup deadlines are predeclared and frozen BEFORE evaluation commences. Limits cannot be adjusted after a trial failure.
- **Baseline Equality Definition**: Baseline environment equivalence is evaluated strictly by backend identifier, guest build version, input artifacts, applied `GuestSecurityProfile`, and observed execution evidence—not opaque host/guest memory equality.
- **Non-Regression Baselines**: Legacy Android AVD and macOS iOS Simulator operations are evaluated across 20 fixed lifecycle trials before and after research backend execution, preserving constitution performance budgets (8ms polling, <150ms startup, <=50ms details, <=10ms log streaming).

---

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Across the 4-guest reference cohort, 100% of tested root guest configurations (at least one per named backend) achieve verifiable root proof across 3 consecutive cold boots, with privilege status successfully re-proven and negative controls confirmed on each boot (referencing User Story 2 Acceptance Scenarios 1 and 3).
- **SC-002**: On the reference `Inferno` iOS userland guest, 100% of trials across 3 consecutive cold boots successfully install an owned sample iOS application, deploy and start Frida, attach to the target process, intercept both native and Objective-C hooks with an untouched control process, detach cleanly without process termination, and stop/remove Frida cleanly (referencing User Story 3 Acceptance Scenarios 1–3).
- **SC-003**: 100% of queries on backend configurations lacking application frameworks accurately report application-layer frameworks as unavailable and refuse application deployment with structured errors, without substituting user-space simulation (referencing User Story 3 Acceptance Scenario 5).
- **SC-004**: On reference configurations for both named backends (`darwin-vm` minimal kernel debug configuration and `Inferno` system research configuration), 100% of trials across the evaluation cohort successfully execute kernel pause, register inspection, memory inspection, breakpoint triggering, single-stepping, controlled test state modification, and state restoration without triggering guest boot failure, false crash status, or session corruption (referencing User Story 4 Acceptance Scenarios 1–2).
- **SC-005**: On the reference `Inferno` guest, 100% of trials successfully attach dynamic instrumentation (Frida) to a selected compatible guest system daemon, capturing trace events from system service calls with an uninstrumented control daemon remaining unaffected, and detaching cleanly without daemon termination (referencing User Story 4 Acceptance Scenario 5).
- **SC-006**: When a kernel debugger disconnects from a guest instance on either `darwin-vm` or `Inferno`, 100% of disconnect events preserve observed execution states, report status truthfully as paused, running, or unknown without falsely claiming resumption or silent execution modification, and present safe recovery prompts via CLI and TUI (referencing User Story 4 Acceptance Scenario 3).
- **SC-007**: 100% of guest root execution, in-guest filesystem writes, and instrumentation operations across the cohort result in zero unauthorized privilege elevation, zero unauthorized filesystem modifications, and zero configuration alterations on the macOS host in declared containment fixtures (referencing User Story 2 Acceptance Scenario 4 and User Story 4 Acceptance Scenario 4).
- **SC-008**: Across 20 fixed legacy lifecycle trials (launch, inspect, log stream, stop) evaluated before and after research backend execution, standard Android AVD and macOS iOS Simulator workflows demonstrate 100% behavioral parity with zero observed functional or performance regressions against constitution budgets (input polling <= 8ms, cold startup < 150ms, device details <= 50ms, log streaming <= 10ms), and all eight Android 001 specification artifacts remain byte-for-byte preserved (referencing User Story 1 Acceptance Scenario 3).
- **SC-009**: 100% of corrupted, truncated, or incompatible system images and firmware artifacts evaluated in the cohort are detected and rejected prior to hypervisor launch, and unverified experimental images require explicit opt-in and verified baseline (referencing User Story 5 Acceptance Scenarios 1–2).
- **SC-010**: 100% of host-side guest disk mounting and preparation workflows verify isolated mounted image identity and device nodes, with zero reliance on hardcoded host paths such as `/Volumes/System` and zero modification to host SSV or SIP (referencing User Story 5 Acceptance Scenario 3).
- **SC-011**: 100% of companion VM instances and associated helper resources (ports, forwarders) terminate cleanly within declared configuration deadlines ONLY when no active restore workflow remains AND no live dependent guest sessions remain, leaving zero orphaned background processes (referencing User Story 6 Acceptance Scenarios 2–3).
- **SC-012**: 100% of authorized baseline recovery operations restore an altered guest to its clean RecoveryBaseline within a predeclared configuration-specific deadline recorded prior to measurement, and missing baselines safely refuse restoration (referencing User Story 7 Acceptance Scenario 3).
- **SC-013**: Research profiles exported from an Apple Silicon workstation successfully reproduce an identical operational research baseline on a fresh guest instance on the same or independent compatible workstation in 100% of tested valid imports with zero host secrets leaked, with at least one fresh same-backend trial successfully reproducing live root proof and one Inferno trial successfully reproducing real application launch and Frida dynamic hook execution rather than merely validating metadata (referencing User Story 7 Acceptance Scenarios 1 and 4).
- **SC-014**: 100% of in-scope research operations across the cohort are executable via non-interactive CLI commands with structured, machine-parseable output (stdout), separated diagnostics (stderr), appropriate outcome exit codes, and full semantic parity with the interactive TUI (referencing User Story 8 Acceptance Scenarios 1 and 5).
- **SC-015**: Interactive TUI navigation maintains an input latency of <= 100ms across 50 consecutive navigation events, and acknowledges user cancellation requests within <= 200ms across 10 consecutive cancellation trials during active background operations, with zero UI thread freezes (referencing User Story 8 Acceptance Scenario 5).
- **SC-016**: 100% of caller-bounded timeout events in the cohort accurately report the current operational status (continuing, stopped, or unknown) without falsely claiming task termination (referencing User Story 8 Acceptance Scenario 2).
- **SC-017**: 100% of guest instances sharing identical display names across distinct backends are uniquely resolved, addressed, and controlled without unintended cross-instance side effects (referencing User Story 1 Acceptance Scenario 1).
- **SC-018**: 100% of executed research trials generate an immutable `ExperimentRecord` capturing full environmental and evidence parameters, preserving historical records from subsequent modifications (referencing User Story 7 Acceptance Scenario 2).
- **SC-019**: 100% of the 16 critical negative cases defined in the Reference Acceptance Set are evaluated at least once per applicable backend within the cohort, returning documented outcome-appropriate classifications (including exit code 0 with structured unavailable state for read-only queries and already-satisfied desired profiles, and distinct non-zero error codes for failures, rejections, and cancellations) without unhandled exceptions or state corruption.

---

### Acceptance Coverage Matrix

| Functional Requirement Group              | In-Scope FRs     | Primary User Stories | Positive Test Oracle                                                                                                      | Negative / Falsification Oracle                                                            |
| :---------------------------------------- | :--------------- | :------------------- | :------------------------------------------------------------------------------------------------------------------------ | :----------------------------------------------------------------------------------------- |
| **Host Preflight & Backend Separation**   | FR-001 to FR-004 | Story 1              | Acceleration & permissions verified; backends isolated; legacy non-regression                                             | Host lacking CPU acceleration reports limits; ambiguous names rejected                     |
| **Instance Lifecycle & Identity**         | FR-005 to FR-009 | Story 1              | Unique immutable identifier assigned; create/register/inspect/start/stop/restart/delete succeed; destructive confirmation | Ambiguous instance ID rejected; unconfirmed deletion aborted                               |
| **Root Proof & Falsification**            | FR-010 to FR-015 | Story 2              | Root fixture executes; negative control denied; kernel/binary evidence recorded                                           | Negative control passes unexpectedly or fixture denied => unverified status                |
| **App Lifecycle & Frida Instrumentation** | FR-016 to FR-024 | Story 3              | App installs & runs; Frida deploys/starts/stops/removes; native & ObjC hooks captured; read/export scoped app files       | Incompatible ABI/signature blocked; script error reports observed target status truthfully |
| **Deep System & Kernel Debugging**        | FR-025 to FR-030 | Story 4              | Kernel pause/resume/step/registers; test state edit & restore; daemon Frida trace; read/export scoped system files        | Debugger disconnect reports actual paused/running/unknown state without silent resume      |
| **Image Prep & Host Security**            | FR-031 to FR-036 | Story 5              | Image hash verified; device node confirmed; disposable cleanup succeeds                                                   | Unattended elevation refused; unverified image without baseline blocked                    |
| **Companion VM Orchestration**            | FR-037 to FR-039 | Story 6              | Companion launches; same-host access control; retained while guest dependent                                              | Companion terminated only when no active workflow AND no dependent guest                   |
| **Profiles & Experiment Records**         | FR-040 to FR-043 | Story 7              | Profile exported w/o secrets; baseline restored; immutable records preserved; revalidated import                          | Missing baseline refuses recovery; imported profile unverified until fresh live run        |
| **Automation CLI & TUI Parity**           | FR-044 to FR-048 | Story 8              | Non-interactive CLI parity; structured stdout/stderr; TUI responsive <=100ms                                              | Timeout reports actual continuing state; cancel-pending distinguishes cessation            |

---

## Assumptions

- Target host hardware is Apple Silicon Mac (`aarch64`) running a supported version of macOS with applicable hypervisor or virtualization permissions enabled. A single Apple Silicon Mac workstation is sufficient for all reference cohort trials and import validations.
- Base firmware files, IPSW restore bundles, kernel caches, recovery assets, and test applications are legally obtained and provided by the researcher; Emu does not bundle, host, or distribute proprietary Apple software or copyrighted firmware.
- The project draws architectural inspiration from publicly documented Corellium research models ([Corellium iOS Architecture & Controls](https://support.corellium.com/devices/ios), [Corellium Frida Feature Guide](https://support.corellium.com/features/frida/), [Corellium Kernel Debugging](https://www.corellium.com/blog/debug-kernel)) and the [Frida iOS Documentation](https://frida.re/docs/ios/) strictly as an architectural benchmark and long-term research aspiration; in Corellium's published architecture, "root is built into their controlled virtual stack, no exploit chain required." Emu's exact backend preparation is an independent implementation; Emu has zero Corellium account dependencies, commercial integrations, or mandatory commercial feature parity, and does not port KernelSU to iOS.
- Upstream project references include [jprx/darwin-vm](https://github.com/jprx/darwin-vm) for minimal Darwin root shells and [ChefKissInc/Inferno](https://github.com/ChefKissInc/Inferno) / [chefkiss.dev](https://chefkiss.dev/applehax/inferno/) for iOS research environments.
- Verification of guest root privileges (UID 0) within an iOS-derived environment confirms research execution capability and does not imply arbitrary kernel code execution or bypass of unmodeled silicon security processors.
- Dynamic instrumentation via Frida operates in an authorized guest environment with relaxed security policies; Frida Gadget injection into debuggable non-jailbroken applications is not treated as equivalent to root VM access.
- Graphical display capabilities (e.g., SpringBoard in `Inferno`) are evaluated as observable research features where supported, without promising consumer-grade app parity, iCloud integration, App Store functionality, or biometric simulation.
- Specialized restore workflows for `Inferno` are accommodated via local companion virtualization on the same macOS host, eliminating any requirement for external physical Linux hardware.
- Android research backend specifications and implementation plans from feature 001 remain fully preserved and decoupled, ready for execution when a Linux environment becomes available.
- Guest filesystem modifications executed in volatile ramdisk-based root environments reset across cold boots unless explicitly committed to an underlying persistent guest disk image prepared via backend-supported image preparation workflows.
