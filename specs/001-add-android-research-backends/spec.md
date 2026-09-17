# Feature Specification: Android Research Backends and Tooling

**Feature Branch**: `None (no feature branch created; no before_specify hook was registered at invocation start)`

**Created**: 2026-09-17

**Status**: Draft

**Input**: User description: "mình cần develop Android Emulator và Cuttlefish, hỗ trợ KernelSUNext, frida, xposed v.v.v"

**Additional User Context**:

- "https://github.com/WildKernels/GKI_KernelSU_SUSFS ví 1 ví dụ các tool có trong này"
- "bổ sung 1 cái là các cli command cần phát triển đầy đủ hơn để cho automation tui là cho người dùng"
- "ưu tiên phát tiển theo tứ tự trc ví dụ android sẽ làm trc IOS vì google đã cung cấp khá đầy đủ các tool để làm"

> **Notice**: The capabilities specified in this document represent a required future delivery
> target and development scope for the Emu project. They do not constitute an active implementation
> claim of existing repository capabilities. In accordance with project governance, full support
> requires empirical verification on controlled, compatible real environments before being declared
> supported, while automated CI continues to rely on decoupled unit and mock tests.

## Clarifications

### Session 2026-09-17

- Q: Which mobile platform should be prioritized for development, and why? → A: Develop Android research capabilities first, covering Android Emulator and Cuttlefish, then new iOS/Apple research capabilities in a later phase; the priority reflects the user's assessment that Google's Android tooling is more readily available.

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Backend Selection, Lifecycle Management, Instance Identity, and Standard Device Non-Regression (Priority: P1)

A mobile security researcher selects between Android Emulator and Cuttlefish virtualization
backends to manage virtual Android devices for security research. The researcher runs non-interactive
local preflight diagnostics to inspect host virtualization compatibility without launching graphical
interfaces. The researcher creates or registers virtual devices with user-supplied system images,
inspects device state, and controls standard lifecycle operations (start, stop, restart, delete). When
multiple instances across different backends share identical or similar user-assigned display names,
the system clearly differentiates them in all views and commands. Routine lifecycle commands proceed
without redundant prompts, while destructive operations (such as storage wipes or device deletions)
require explicit authorization that identifies the affected resources. Furthermore, standard Android
virtual devices operating without research tools continue to run with full parity and zero regressions.

**Why this priority**: Core virtualization lifecycle control, truthful host capability verification,
unambiguous instance addressing, and preservation of existing standard device workflows form the
foundation for all research work. Without reliable device creation and non-regression guarantees,
researchers cannot deploy privilege and instrumentation tooling.

**Independent Test**: On a host workstation with Android SDK tools and/or Cuttlefish installed,
register an Android Emulator device named "research-pixel" and a Cuttlefish device named
"research-pixel". Run preflight diagnostics to verify host compatibility reporting, start and stop each
device independently verifying distinct backend instance targeting, execute a deletion workflow
verifying explicit confirmation, and run an existing standard Android virtual device verifying
unaffected baseline operation.

**Acceptance Scenarios**:

1. **Given** a host workstation with compatible Android Emulator and Cuttlefish prerequisites,
   **When** the researcher creates an Android Emulator device named "research-pixel" and registers a
   Cuttlefish device named "research-pixel",
   **Then** both devices appear in the inventory with distinct backend badges and unique system
   identifiers, and starting "research-pixel" under Android Emulator boots only the Emulator instance
   while the Cuttlefish instance remains stopped.
2. **Given** a host workstation lacking required virtualization capabilities (such as running
   Cuttlefish on macOS or Windows, or on a Linux environment lacking hardware virtualization support),
   **When** the researcher executes preflight checks or attempts to launch a Cuttlefish device,
   **Then** the system explicitly reports the missing prerequisites and marks the backend as
   unsupported on this host, refusing launch without crashing or hanging.
3. **Given** a research device with persistent storage data,
   **When** the researcher requests a storage wipe or device deletion,
   **Then** the system requires explicit user confirmation that names the specific device identifier
   and storage resources affected before executing the destructive action.
4. **Given** an existing standard Android virtual device configured for regular application testing
   without research tools,
   **When** the researcher lists, starts, inspects, and stops the standard device,
   **Then** the device operates with full functional parity compared to baseline behavior, with no
   research tool overlays, no unexpected privilege modifications, and no performance degradation.

---

### User Story 2 - Unattended Research Automation and Interactive Human Verification (Priority: P1)

A mobile security researcher or test automation engineer executes complete end-to-end Android security
research workflows in unattended, automated environments (such as CI/CD pipelines, regression test harnesses,
or batch evaluation scripts) without an interactive terminal or graphical interface. The researcher
discovers available backend capabilities, device inventories, and research profiles through self-describing
automation commands, addresses specific virtual devices unambiguously via stable unique identifiers, and
executes lifecycle, tool deployment, and experiment tasks using purely non-interactive invocations. The
automation interface produces structured, machine-readable outcomes with separated diagnostics and
documented process termination classifications, allowing automated scripts to unambiguously differentiate
between operations that completed successfully (including successful guest mutations, already-satisfied
declarative configurations, and truthful read-only inspection queries, all yielding a documented successful
process status with distinguishable outcome properties in the structured payload) and non-success outcomes
(including rejected attempts to execute unsupported actions, invalid or missing inputs, authorization
refusals, runtime execution failures, operation cancellations, and execution timeouts). Destructive
operations requested without explicit automation authorization safely refuse execution before any persistent
resource is modified, while requests carrying explicit, resource-scoped authorization proceed predictably
without interactive confirmation prompts. When a long-running operation reaches its execution deadline or is
cancelled, the system reports the actual known operational state and any partial committed modifications
without falsely claiming success. When declarative desired-state configurations are repeatedly applied, the
system converges without repeating destructive guest mutations or artifact deployments. Finally, when
evaluating the environment across both interfaces, research profiles created in either interface can be read
and used by the other, observing equivalent target identities, semantic operational states, diagnostic
meaning, and safety boundaries.

**Why this priority**: Unattended automation is essential for continuous mobile security testing,
reproducible vulnerability research, and reliable CI test harnesses. Treating automation as a first-class
interface alongside the human interactive surface ensures that research environments can be provisioned,
evaluated, and torn down deterministically at scale, while guaranteeing that human and programmatic workflows
share identical capability limits and safety guarantees.

**Independent Test**: On a host workstation with prepared compatible virtualization and artifact fixtures,
execute an entire end-to-end research workflow (capability discovery, device creation, research profile
deployment, elevated test verification, and teardown) purely via the non-interactive automation interface
without an interactive terminal session or TUI navigation. Verify that all operations produce parseable
structured results and documented exit classifications, with successful mutations, already-satisfied
declarative states, and capability inspection queries sharing a successful process status while unsupported
action attempts, invalid inputs, verification failures, cancellations, and timeouts yield non-success
classifications. Test that missing explicit authorization for a destructive operation safely refuses before
modification while authorized execution succeeds. Test that repeated declarative profile application
reports already-satisfied without redundant guest mutations. Test that a caller-bounded timeout reports
actual known operational state and partial changes without falsely claiming cessation. Then inspect the
resulting environment via the interactive TUI, verifying that target instance identification, semantic tool
states, diagnostic meaning, and safety boundaries match the automation results at the settled reference
state; and verify bi-directional profile reuse by applying a TUI-configured research profile through the
automation interface.

**Acceptance Scenarios**:

1. **Given** a host workstation with prepared compatible virtualization backends and research artifacts,
   **When** the researcher executes an end-to-end research workflow (discovering host capabilities, registering
   a virtual research device, applying a research profile with privilege and instrumentation tools, executing
   a benign guest verification check, and stopping the instance) entirely through non-interactive automation
   commands,
   **Then** the entire sequence executes to completion without requiring an interactive terminal session,
   keystroke emulation, or screen scraping, and each step returns a structured machine-readable result
   indicating successful execution.
2. **Given** an environment containing multiple research devices across different backends, including
   instances sharing identical user-assigned display names,
   **When** the researcher queries the automation interface for available commands, operational parameters,
   and device inventory, and then targets an operation using a specific unique device identifier,
   **Then** the system provides complete, discoverable input and capability metadata, and directs the
   requested operation exclusively to the targeted instance without ambiguity, accidental cross-instance
   misdirection, or reliance on active interactive selection.
3. **Given** an automated testing harness invoking research management operations,
   **When** commands are executed across distinct operational outcomes (a successful guest modification, an
   already-satisfied declarative configuration, a read-only query inspecting an unavailable capability, an
   attempt to execute an action on an unsupported host or backend, invalid or missing arguments, a failed
   guest verification check, an explicit cancellation request, and an operation exceeding its deadline),
   **Then** the invocation produces a structured machine-readable payload and documented process exit
   classification where successful modifications, already-satisfied declarative states, and truthful
   read-only inspection queries share a documented successful process status (with distinguishable outcome
   properties in the structured payload), while unsupported action attempts, invalid inputs, verification
   failures, cancellations, and timeouts yield documented non-success process classifications, with
   human-readable diagnostics separated from the structured result.
4. **Given** an active research device containing persistent research data and guest modifications,
   **When** an unattended automation command requests a destructive operation (such as a storage wipe or
   device deletion) without explicit, resource-scoped authorization,
   **Then** the system safely refuses execution before modifying any storage or guest state, reports an
   authorization failure in its structured outcome, and does not hang waiting for interactive input; and
   when the same command is executed with explicit resource-specific authorization identifying the target
   instance, the destructive action proceeds to completion without prompting.
5. **Given** a long-running research operation in progress (such as large artifact staging, kernel
   replacement, or guest startup),
   **When** the caller stops waiting due to an expired execution deadline, or explicitly submits an
   operation cancellation request,
   **Then** in the case of a deadline expiration, the system returns a timeout outcome reporting the actual
   known operational state (stopped, continuing, or unknown) and any partial committed changes without
   claiming the underlying guest task has ceased; and in the case of an explicit cancellation request, the
   system acknowledges the cancellation request and attempts to reach a verified safe boundary, reporting the
   operation as cancelled only after actual cessation at that boundary is confirmed, or reporting
   cancellation-pending with actual known operational state (continuing or unknown) if the caller's bounded
   wait deadline expires before the safe boundary is reached, without falsely claiming immediate termination
   or promising automatic reversal of authorized committed steps.
6. **Given** an automated pipeline managing devices where an Android Emulator instance and a Cuttlefish
   instance share the display name "ci-research-worker",
   **When** automation commands inspect, modify, and stop one of the instances using its stable system
   identifier,
   **Then** only the designated instance is modified while the sibling instance with the identical display
   name remains in its original state, and the structured response reflects the exact targeted backend and
   system identifier.
7. **Given** a research device with an already-applied, verified research profile in ready state,
   **When** an automation workflow repeatedly applies the same declarative research profile specification,
   **Then** the system detects that the desired state is already satisfied, reports the already-satisfied
   outcome without redundant kernel flashes, guest reboots, or artifact restaging; whereas if an explicit
   action command (such as device restart, storage wipe, or dynamic tracing session start) is invoked, the
   action executes as an explicit operation without silent suppression.
8. **Given** a research environment and research profiles managed across unattended automation and
   interactive human surfaces,
   **When** a researcher creates or modifies a profile in the automation interface and inspects or executes
   equivalent business actions in the interactive TUI, and conversely exports a research profile configured in
   the interactive TUI and applies it via the unattended automation interface,
   **Then** both interfaces observe the same target instance identity, identical semantic tool operational
   states (including pending-reboot and ready conditions), equivalent diagnostic meaning, and the same safety
   authorization policies at the settled reference state, with profiles created in either interface fully
   usable and actionable in the other in both directions without requiring hidden or active interactive TUI
   selections.

---

### User Story 3 - KernelSU Next Privilege Escalation and Verification (Priority: P2)

A mobile security researcher configures an Android research device with KernelSU Next to conduct
kernel-level and operating system privilege research. The researcher supplies a compatible custom
kernel artifact with KernelSU Next integration alongside the user-space manager component. Ordinary
production Play Store system images that prohibit custom kernels need not permit research root. The
researcher triggers privilege preparation, boots the device with the custom kernel, and verifies that
elevated guest privileges are active through a controlled, guest-scoped test command that confirms
KernelSU Next identity specifically (distinguishing it from generic root debugging access on userdebug
builds). The system ensures that guest privilege escalation remains strictly confined to the virtual
machine without altering host system permissions, and provides the ability to revert the kernel back
to a standard baseline or completely remove KernelSU Next components based on explicit user choice.

**Why this priority**: Kernel-level privilege management is essential for operating system security
research and enabling non-standard instrumentation. KernelSU Next provides a modern, kernel-assisted
root mechanism whose distinct identity must be strictly maintained without silent substitution.

**Independent Test**: On a compatible running research device, provide an authorized KernelSU Next
kernel artifact and manager package. Trigger kernel configuration, restart the guest, execute an
elevated test command within the guest to confirm root execution and KernelSU Next version reporting,
and verify that host operating system permissions remain strictly unaffected. Then test both disabling
(reverting kernel only) and complete removal (reverting kernel and removing manager components),
verifying accurate state reporting.

**Acceptance Scenarios**:

1. **Given** an active research device booted with a user-authorized KernelSU Next-enabled kernel
   artifact and manager component,
   **When** the researcher enables privilege management and executes an authorized elevated test
   command in the guest,
   **Then** the command executes with root privilege, the system verifies and reports KernelSU Next
   identity and version in the tool status view, and host system permissions remain untouched.
2. **Given** an active research device running an incompatible kernel build or missing required kernel
   hook symbols,
   **When** the researcher attempts to enable KernelSU Next,
   **Then** the system refuses activation, displays a diagnostic explaining the kernel incompatibility,
   and does not silently fall back to an unrequested alternative root mechanism or substitute tools.
3. **Given** a research device currently operating with an active KernelSU Next kernel,
   **When** the researcher chooses to disable privilege management while retaining installed manager
   components,
   **Then** the system restores the baseline kernel artifact, updates the observed status to inactive,
   and boots into the standard privilege state with manager components remaining installed but
   non-functional.
4. **Given** a research device currently operating with KernelSU Next,
   **When** the researcher chooses to completely remove KernelSU Next,
   **Then** the system restores the baseline kernel, purges the user-space manager components from the
   guest, updates observed status to uninstalled/removed, and boots cleanly into the standard baseline
   state.

---

### User Story 4 - Frida Dynamic Instrumentation Workflow (Priority: P3)

A mobile security researcher deploys, starts, monitors, stops, and removes Frida dynamic
instrumentation on a running research device. The researcher supplies a compatible Frida tool package
matching the guest CPU architecture. The system deploys the tool package and reports its state as
installed and available awaiting verification. The researcher attaches an authorized instrumentation
script to a designated sample application and observes real-time function execution and telemetry; only
upon successful observation of the running application function does the system mark the tool status as
ready. The researcher can stop the session, which halts dynamic tracing and leaves the application
running normally, or completely remove the Frida package without affecting unrelated tools or user
application data.

**Why this priority**: Dynamic binary instrumentation and runtime tracing are daily activities in
mobile application security analysis. Researchers require dependable, observable agent deployment and
demonstrable function tracing to evaluate application security controls.

**Independent Test**: On a running research device, deploy an authorized Frida tool package matching
the guest architecture, verify that status transitions to installed/available, attach an authorized
instrumentation script to a benign sample application, verify function interception telemetry and the
transition to ready status, stop the session, and execute a complete removal verifying clean deletion.

**Acceptance Scenarios**:

1. **Given** a running research device and an authorized sample application installed on the guest,
   **When** the researcher deploys a compatible Frida tool package and starts the instrumentation
   runtime,
   **Then** the system deploys the package, reports observed tool status as available awaiting
   verification (not ready), and displays active tool package details (version and target
   architecture).
2. **Given** an active Frida runtime reported as available awaiting verification on a research device,
   **When** the researcher attaches an authorized tracing script targeting a specific function in the
   sample application and triggers that function,
   **Then** the function execution event is captured and displayed in the telemetry log, the sample
   application continues executing normally, and observed tool status transitions to ready.
3. **Given** an active instrumentation session,
   **When** the researcher stops the Frida service,
   **Then** the system cleanly halts the instrumentation process, closes host communication channels,
   updates observed status to stopped/inactive, and leaves the target application operating without
   active hooks.
4. **Given** a research device running an arm64 guest architecture,
   **When** the researcher supplies an x86_64 Frida tool package,
   **Then** the system detects the package architecture mismatch prior to deployment, marks observed
   status as failed with an explanatory diagnostic, and avoids deploying the incompatible package.
5. **Given** an installed Frida tool package on a research device,
   **When** the researcher chooses to remove Frida,
   **Then** the system halts any running instrumentation, uninstalls the tool package files from the
   guest, updates observed status to removed, and preserves all unrelated tools and user application
   data.

---

### User Story 5 - Xposed-Compatible Framework and Scoped Module Management (Priority: P4)

A mobile security researcher deploys, configures, and manages an Xposed-compatible framework and its
modules on an Android research device under a compatible provider to mediate runtime application
behavior. The researcher prepares the framework, verifies its operational readiness through supported
framework test checks (never assuming readiness merely from a guest reboot), installs user-supplied
module packages, and assigns execution scope strictly to designated target applications. The
researcher confirms that visible behavioral changes occur exclusively in the selected target
application, while non-target control applications and other guest environments retain baseline
behavior. When framework or module activation or deactivation requires a guest restart under the
selected provider's rules, the system unambiguously reports observed state as `pending-reboot` while
preserving the user's requested desired state (`enabled`, `disabled`, or `removed`). If the selected
provider does not require a reboot, the state transitions immediately. After any guest restart,
readiness is re-verified before ready status is reported. The researcher can also disable or completely
remove modules and the framework.

**Why this priority**: Runtime application mediation requires framework-level management. Enforcing
strict application-level scoping prevents unintended side effects, and transparent pending-reboot
status tracking eliminates confusion regarding when modifications take effect.

**Independent Test**: On a research device, deploy a compatible Xposed-compatible framework, verify its
activation via capability test, install a benign sample module scoped exclusively to an authorized
sample application, observe pending-reboot status if required by the provider, restart the device if
required, verify framework and module readiness via test checks, verify that the module modifies
behavior only in the target application while leaving an independent control application untouched,
then disable and remove the module and framework, verifying accurate state transitions.

**Acceptance Scenarios**:

1. **Given** a research device without an active hooking framework,
   **When** the researcher prepares and enables a compatible Xposed-compatible framework,
   **Then** the system records desired state as enabled, tracks observed state as pending-reboot if
   the selected provider requires a reboot (or verifying if immediate), and marks observed state as
   ready only after a post-startup framework capability test confirms functional readiness.
2. **Given** a research device with an active Xposed-compatible framework,
   **When** the researcher installs a user-supplied module and scopes it strictly to an authorized
   sample application,
   **Then** the system records desired state as enabled, marks observed state as pending-reboot if a
   restart is required by the provider, and verifies the module's behavior after any required restart
   (or without a restart when none is needed), reporting ready only after that check succeeds.
3. **Given** a research device operating with an active scoped Xposed-compatible module,
   **When** the researcher runs both the targeted sample application and an independent non-target
   control application,
   **Then** the module's behavioral modifications are observed exclusively within the targeted
   application, the control application behaves identically to its baseline, and non-target guest
   environments remain unaffected.
4. **Given** an active Xposed-compatible module,
   **When** the researcher disables the module,
   **Then** the system records desired state as disabled, transitions observed state to pending-reboot
   if a restart is required (or verifying otherwise), and confirms after any required restart and
   verification that the target application has no active modifications while the module remains
   installed but inactive.
5. **Given** an installed Xposed-compatible module or framework,
   **When** the researcher uninstalls or removes the component,
   **Then** the system records desired state as removed, reports pending-reboot when the provider
   requires a restart (or verifying otherwise), and reports the component as removed only after
   any required restart and verification confirm that it is no longer installed or active.

---

### User Story 6 - Full Toolchain Coexistence, Provenance Reuse, and Baseline Recovery (Priority: P5)

A mobile security researcher runs a comprehensive security evaluation where KernelSU Next, Frida, and
a compatible Xposed-compatible framework operate simultaneously in a single virtual guest device per
backend (evaluated separately on Android Emulator and Cuttlefish) without functional collision. The
researcher exports an immutable experiment provenance record capturing all device parameters, kernel
builds, tool versions, and module fingerprints. The researcher then reuses this saved research profile
to reconstruct an identical research environment on a new compatible device instance of the same
backend, verifying that identical benign tool behaviors are reproduced. If the researcher attempts to
reuse a profile on an incompatible instance, the system displays differences and refuses incompatible
reuse. If an experimental kernel configuration or module causes guest instability or boot failure, the
researcher rapidly restores the device back to an identified working baseline when a verified baseline
exists, while missing baselines safely refuse recovery without data loss.

**Why this priority**: Comprehensive research requires combining root privilege, dynamic tracing, and
runtime application mediation. Ensuring non-interfering coexistence, reproducible provenance for
published findings, and rapid recovery from destructive changes prevents research workflow paralysis.

**Independent Test**: On a single Android research device under each backend, activate KernelSU Next,
start Frida, and enable an Xposed-compatible module. Run a combined validation script exercising root
access, Frida function tracing, and scoped module mediation concurrently on an authorized sample
application. Export the provenance record, apply it to a new compatible device instance of the same
backend, verify that all three tools deploy and reproduce declared benign outcomes, test refusal on an
incompatible target instance, and test recovery from boot failure.

**Acceptance Scenarios**:

1. **Given** an active Android research device under Android Emulator or Cuttlefish with KernelSU Next,
   Frida dynamic instrumentation, and a compatible Xposed-compatible framework configured,
   **When** the researcher executes a composite evaluation workload exercising guest root commands,
   Frida function tracing, and scoped application behavior mediation simultaneously on an authorized
   test app,
   **Then** all three tools function without mutual interference, crashing, or deadlock, and all three
   report observed status as ready in that guest environment.
2. **Given** a research device configured with multiple research tools and modules,
   **When** the researcher exports the experiment provenance record,
   **Then** the system generates a structured record containing the backend type, guest OS release,
   kernel build identifier, tool versions, module list with cryptographic fingerprints, and launch
   parameters.
3. **Given** an exported research profile provenance record from an existing research environment,
   **When** the researcher imports and applies this profile to a newly created compatible research
   device instance of the same backend,
   **Then** the system validates prerequisite compatibility, stages the specified artifacts, deploys
   the toolchain, and reproduces the declared benign outcomes across KernelSU Next, Frida, and the
   Xposed-compatible framework.
4. **Given** an exported research profile requiring a specific guest architecture or kernel release,
   **When** the researcher attempts to apply this profile to a device instance with mismatched
   architecture or missing host prerequisites,
   **Then** the system highlights the specific missing or mismatched requirements, safely refuses to
   apply the profile, and leaves the target device in its existing state.
5. **Given** an experimental module or kernel modification that prevents the guest device from
   completing its boot sequence within the documented operation deadline,
   **When** the system detects the boot failure and the researcher initiates a baseline recovery with
   a verified usable baseline image,
   **Then** the system restores the guest disk and kernel state to the designated working baseline,
   successfully boots the device, and marks the failed experimental configuration as reverted.

---

### User Story 7 - Extended Kernel Research Profile Selection, Capability Verification, and Control Comparison (Priority: P6)

A mobile security researcher defines, configures, and applies an extended kernel research profile to
a compatible virtual research device, incorporating optional kernel research capabilities and alternative
root privilege implementations drawn from the reference capability catalog (inspired by the user's
reference toolset). The researcher inspects declared versus observed capabilities and prerequisite
dependencies before making changes, selects a compatible set of user-supplied custom kernel artifacts,
modules, and runtime components, and provides explicit user authorization prior to guest modification.
To validate the profile, the researcher runs a controlled, benign, guest-scoped verification experiment
comparing an enabled optional capability against a no-root or feature-disabled control baseline. When
evaluating optional components, the researcher can explicitly choose an alternative root implementation
(such as upstream KernelSU, ReSukiSU, or source-specific opt-ins SukiSU-Ultra or KowSU) without altering
default KernelSU Next profiles on other devices, while strictly enforcing the invariant of exactly one
active root mechanism per guest profile. If a requested profile contains mutually conflicting features
(such as KowSU combined with SUSFS root visibility research) or targets a custom kernel with an
incompatible kernel family or mismatched Kernel Module Interface (KMI, where applicable) based on declared
interface evidence (even when the guest Android OS release matches), the system rejects the configuration
before modifying the guest. When an extended capability relies on physical hardware partitions absent from
the target virtual device (such as Baseband Guard partition protection), the system reports the feature as
not applicable without simulating artificial support or modifying host partitions. When underlying
kernel-level capability support is absent from an image, introducing it requires deploying a compatible
replacement image or kernel artifact and performing a guest restart where required by the backend; once
the requisite support is present in the kernel, researchers can exercise supported runtime enable,
configure, or disable controls without the system pretending that absent compiled-in support can be
activated via live toggle alone. If an experimental kernel capability or module causes guest instability,
kernel panic, or interferes with active dynamic instrumentation or application hooking, the system
strictly isolates the failure to protect the host and other guests, retains available diagnostic
information, avoids reporting false-ready state, and allows the researcher to restore the device to its
verified baseline. Finally, the researcher can disable the capability via its supported mechanism or
restore the verified baseline, and reproduce the verified profile on a fresh compatible instance.

**Why this priority**: Advanced kernel security research requires specialized kernel features (such as
root visibility research, mount mediation, advanced networking, extended filesystem attributes,
eBPF/BTF observability, synchronization primitives, and container runtimes) alongside root selection
flexibility. Bounding these extended capabilities with strict dependency gating, interface compatibility
checks, truthful hardware nonapplicability reporting, and control comparisons ensures researchers can
systematically investigate kernel behaviors without risking host isolation or masking guest failures.

**Independent Test**: On a compatible research device with an established baseline, configure an
extended research profile selecting an alternative root (or KernelSU Next with an optional capability
such as SUSFS root visibility research or eBPF observability). Inspect declared versus observed
dependencies, verify rejection of a conflicting profile (KowSU+SUSFS), verify rejection of an artifact
with mismatched kernel family or applicable KMI evidence, verify nonapplicability reporting when
target virtual hardware lacks physical baseband partitions, verify that absent kernel support requires
a compatible kernel artifact and restart while present capabilities can be configured at runtime, run a
benign guest-scoped experiment comparing against a control baseline, disable or restore baseline, and
export and reproduce the profile on a compatible target.

**Acceptance Scenarios**:

1. **Given** a running research device and a user-supplied custom kernel artifact built with an
   alternative root implementation (such as upstream KernelSU or ReSukiSU) alongside its manager
   component,
   **When** the researcher explicitly selects this alternative root implementation in a new research
   profile and authorizes the change,
   **Then** the system deploys the artifact, verifies and reports the specific chosen root identity
   (distinguishing it from KernelSU Next and generic debugging access), confirms elevated execution via
   an authorized guest test command, and leaves default KernelSU Next profiles on other devices
   unaltered.
2. **Given** a researcher configuring an extended research profile,
   **When** the researcher attempts to combine conflicting catalog capabilities (such as selecting
   KowSU alongside SUSFS root visibility research),
   **Then** the system detects the documented incompatibility, displays an explanatory diagnostic
   detailing the mutual conflict, safely refuses to stage or deploy the profile, and leaves the guest
   device unmodified.
3. **Given** an active research device running a specific guest Android release and kernel family,
   **When** the researcher supplies a custom kernel artifact with a known incompatible kernel family,
   mismatched Kernel Module Interface (KMI, where applicable), or conflicting interface evidence (even
   if targeted for the same Android OS release),
   **Then** the preflight check rejects the incompatible artifact before modifying the guest, displays
   specific diagnostic details explaining the interface mismatch, and does not treat an upstream build
   success or label alone as proof of virtual machine boot compatibility.
4. **Given** an extended research profile requesting physical-hardware-dependent partition protection
   (such as Baseband Guard),
   **When** the researcher evaluates or applies this profile to a virtual device lacking physical
   cellular baseband hardware or radio partitions,
   **Then** the system reports the capability condition as not applicable to the target device, records
   this state truthfully without simulating artificial hardware availability, and makes zero attempts to
   access host partitions or modify host security configurations.
5. **Given** an active research device where an extended kernel capability (such as an advanced
   networking protocol or extended filesystem attribute support) is absent from the running kernel,
   **When** the researcher requests this capability in a research profile,
   **Then** the system requires deploying a compatible kernel artifact and restarting the guest
   (reporting pending-reboot until post-restart verification confirms capability presence), and refuses
   to report activation via live toggle alone; and when the capability is already present in the running
   kernel, the system applies supported runtime configuration controls without requiring an
   unnecessary kernel swap or reboot.
6. **Given** a research device operating with an enabled extended capability (such as SUSFS guest-scoped
   root visibility research or eBPF tracing) alongside active instrumentation tools,
   **When** the researcher runs an authorized, benign verification experiment comparing the active guest
   against a no-root or feature-disabled control baseline,
   **Then** the system records observable behavioral differences strictly within the guest; and if an
   experimental component interferes with dynamic tracing or induces guest instability, the system
   detects the failure, updates observed status to failed or uncertain (avoiding false-ready reports),
   retains available diagnostics, preserves host and other-guest isolation, and provides a
   user-authorized path to revert the component or restore the working baseline.
7. **Given** a research device running an extended research profile,
   **When** the researcher chooses to disable the extended capability via its supported mechanism or
   triggers a restoration to the verified baseline,
   **Then** the system deactivates or reverts the components, confirms post-reversion clean baseline
   operation, and allows exporting and reproducing the extended profile on a fresh compatible device
   instance of the same backend.

### Edge Cases

- **Missing Host Virtualization Prerequisites**: When host virtualization capabilities (such as
  virtualization support on Linux, virtualization entitlements on macOS, or hypervisor features on
  Windows) are missing, disabled, or inaccessible due to permission constraints, preflight checks
  explicitly list the missing components, required capabilities, and unsupported status, refusing
  device launch without crashing or hanging.
- **Known Incompatible vs. Unverified Combinations**: When a researcher attempts to combine an Android
  OS release, CPU architecture, or kernel version that is known to be incompatible with KernelSU Next,
  Frida, or the Xposed-compatible framework, the system blocks activation before any protected guest
  mutation occurs. When a combination is unverified, the system visibly marks it as unverified/experimental,
  requiring explicit user authorization, valid host prerequisites, and a verified recovery baseline
  before proceeding, and never marks it as supported until active verification checks succeed.
- **Corrupted, Mismatched, or Unverified Artifact Packages**: When supplied kernel binaries, APK files,
  or native tool binaries fail cryptographic fingerprint checks against supplied expected values, the
  system rejects the artifact and reports the mismatch before deployment. When an artifact lacks
  expected trust metadata, the system records its fingerprint and reports its status as unverified
  trust rather than known corrupt, allowing user-authorized experimental workflows to proceed under
  guarded policy with baseline recovery available.
- **Device Disconnection or Host Communication Loss During Operations**: If a virtual device stops
  unexpectedly, crashes, or loses communication while an artifact transfer, module installation, or
  instrumentation session is active, the system marks the active operation as interrupted/failed,
  terminates host-side monitoring tasks, cleans up temporary staging files, and prevents orphaned
  background processes.
- **Pending Reboot Status and Re-verification After Restarts**: When an operation requires a guest
  restart under the selected provider's rules (including introducing absent kernel-level capability
  support), the system preserves the user's requested desired state (`enabled`, `disabled`, or `removed`)
  while reporting observed state as `pending-reboot`. Readiness does not survive a restart without
  re-verification; after any device restart, tool operational states transition through verification
  before ready status is reported. Already-present capabilities that support live runtime configuration
  transition immediately without an unnecessary reboot.
- **Single Root Selection and Multiple Privilege Mechanism Collision**: Exactly one root privilege
  implementation (or No Root as a control baseline) is permitted per guest profile. If a guest image
  contains existing competing privilege mechanisms or multiple hook engines, or if a user attempts to
  co-install multiple root tools simultaneously, the system detects the collision, warns of instability,
  and refuses silent co-installation or overwrite. Switching root implementations requires an explicit
  compatible kernel and image replacement with baseline recovery available, and the system never
  silently substitutes KernelSU Next or any selected root mechanism.
- **Guest Boot Failure and Safe Baseline Restoration**: If an activated module, custom kernel, or
  configuration causes the guest to fail to boot within the documented operation deadline, the system
  flags a boot failure, preserves diagnostic logs, and provides an option to restore the device to its
  identified working baseline.
- **User Cancellation of In-Progress Operations and Partial State Handling**: If a user cancels an
  in-progress artifact transfer, kernel preparation, or device configuration task, the system halts the
  operation at its documented safe boundary, reports any partial or uncertain committed state, and
  offers safe recovery to a known baseline rather than assuming uncommitted changes can automatically
  unwind.
- **Missing or Unverified Baseline Recovery Materials (Safe Refusal)**: If baseline recovery is
  requested but the referenced baseline image or configuration is unreadable, missing, or unverified,
  the system safely refuses automated recovery, reports the missing baseline condition, and does not
  automatically delete or recreate the device without explicit user direction.
- **Identical Display Names Across Different Virtualization Backends**: When an Android Emulator
  instance and a Cuttlefish instance are created or registered with identical user display names, the
  system disambiguates them in all views and operations by qualifying them with backend type and unique
  system instance identifier.
- **Conflicting Extended Capabilities or Incompatible Kernel Family**: If a researcher selects
  mutually incompatible capabilities from the reference catalog (such as KowSU combined with SUSFS)
  or supplies a custom kernel artifact whose kernel family, architecture, or applicable Kernel Module
  Interface (KMI) mismatches declared interface evidence (even if targeting the same Android OS release),
  the system identifies the conflict or interface incompatibility during preflight gating and refuses
  deployment before modifying the guest.
- **Physical Hardware Dependent Capabilities on Virtual Hardware (Baseband Guard)**: When an extended
  profile includes capabilities designed for physical device hardware partitions (such as Baseband Guard
  partition protection), virtual research devices lacking physical cellular baseband hardware or radio
  partitions report the capability as not applicable. The system never fakes hardware presence, never
  reports artificial readiness, and strictly prevents any interaction with host partitions.
- **Interference Between Extended Kernel Capabilities and Dynamic Instrumentation**: If an enabled
  extended capability (such as root visibility filtering, custom mount namespaces, or eBPF tracing) alters
  system call or filesystem behaviors in a manner that interferes with active Frida tracing or
  Xposed-compatible hooks, experimental kernels may experience instability or guest crashes. The system
  strictly isolates the failure from affecting the host or other guest instances, detects the failure
  without reporting false-ready status, preserves available diagnostics identifying the potential
  interaction, and allows the researcher to selectively disable the conflicting component or restore
  the verified baseline.
- **Non-Interactive Invocation with Missing Required Authorization for Destructive Actions**: When a
  destructive lifecycle or recovery operation (such as device deletion, storage wiping, or baseline
  rollback) is invoked in an unattended or non-interactive environment without explicit, resource-scoped
  authorization, the system safely refuses execution immediately before touching any resources, records
  an authorization-required failure in its structured output and exit status, and avoids hanging or waiting
  for interactive terminal input.
- **Non-Interactive Invocation with Ambiguous Target Display Name**: When an automation command references
  a display name that matches more than one virtual device across backends without supplying the backend
  qualification or unique system identifier, the system safely refuses the operation before modifying any
  state, emits an unambiguous resolution error listing the candidate unique identifiers, and avoids
  guessing or defaulting to an arbitrary instance.
- **Stream Separation of Machine-Readable Results from Progress and Guest Logs**: When an automation command
  runs concurrently with progress reporting, diagnostic tracing, or streaming guest log output, the
  system strictly isolates structured machine-readable result payloads from asynchronous progress
  messages, terminal escape sequences, and guest log streams, ensuring automated consumers can parse
  result objects directly without filtering terminal formatting noise.
- **Operation Timeout or Early Cancellation Following Partial State Modification**: When a multi-stage
  operation (such as staging artifacts, applying a custom kernel, and initiating guest reboot) reaches
  its execution deadline or is cancelled by explicit caller request after initial steps have been committed,
  the system returns a documented non-success outcome without falsely claiming complete or clean baseline
  state. When a deadline expires, the system returns a timeout outcome reporting the actual known operation
  state (stopped, continuing, or unknown) and the specific partial modifications committed, without claiming
  underlying guest execution has ceased. When cancellation is explicitly requested, the system acknowledges the
  request and attempts to reach a verified safe transaction boundary; it confirms actual cessation only after
  that safe boundary is verified, or reports cancellation-pending with actual known operational state
  (continuing or unknown) if the caller's bounded wait deadline elapses before the safe boundary is reached,
  reporting partial committed state without promising automatic reversal of committed side effects, while
  reporting that baseline restoration is available.
- **Repeated Execution of Desired-State Configurations vs. Explicit Action Commands**: When an automation
  script repeatedly executes a declarative desired-state command (such as applying an already active
  research profile or ensuring a tool is enabled) against a device that already matches that state, the
  system returns an already-satisfied outcome without repeating guest reboots, file writes, or artifact
  staging; conversely, when an explicit action command (such as device reboot, storage wipe, or guest
  test execution) is invoked, the system executes the requested action on each invocation and refuses to
  silently suppress the action or automatically retry a failed attempt.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The system MUST support management of Android virtual research devices on both Android
  Emulator and Cuttlefish backends.
- **FR-002**: The system MUST uniquely identify and address each virtual device instance across all
  backends using a backend-qualified system identifier, preventing cross-device command misdirection
  even when user-assigned display names are identical.
- **FR-003**: The system MUST provide local preflight diagnostics that inspect host operating system
  compatibility, required virtualization entitlements, and binary dependencies without launching
  graphical interfaces or hanging.
- **FR-004**: When host prerequisites for a selected backend are missing or unsupported (including
  Cuttlefish execution outside supported local Linux environments meeting virtual machine requirements,
  or running on macOS or Windows), the system MUST report the specific missing prerequisites and mark
  the backend capability as unsupported.
- **FR-005**: The system MUST support standard lifecycle operations (create, register, start, stop,
  restart, delete) for virtual research devices across supported backends.
- **FR-006**: The system MUST require explicit user authorization that identifies affected resources
  before executing destructive operations (including disk wiping, partition flashing, and device
  deletion), while allowing non-destructive lifecycle actions (start, stop, restart) to proceed without
  redundant prompts.
- **FR-007**: The system MUST preserve full functional parity for existing standard Android virtual
  device workflows without requiring research tool configurations or introducing operational
  regressions.
- **FR-008**: The system MUST evaluate compatibility between host OS, backend type, guest Android OS
  version, kernel release, CPU architecture, and research tool versions prior to applying
  configurations; known incompatible combinations MUST be blocked before any protected guest mutation
  occurs, while unverified combinations MUST be visibly marked as unverified/experimental, requiring
  explicit user authorization, confirmed host prerequisites, and verified available recovery material
  before preparation proceeds.
- **FR-009**: The system MUST distinctly track and report the desired state (enabled, disabled,
  removed) and observed condition (such as available awaiting verification, verifying, ready,
  pending-reboot, inactive, removed, unsupported, failed, or unknown)
  for each research tool on a device, ensuring readiness does not survive a restart without
  re-verification and that reboot requirements are conditional on the selected provider.
- **FR-010**: The system MUST strictly maintain requested tool identity for KernelSU Next (and any
  explicitly selected alternative root implementation), Frida, and a compatible Xposed-compatible
  provider, MUST NOT silently substitute KernelSU Next or any selected root mechanism with another
  tool or hooking engine, and MUST enforce that exactly one root privilege implementation (or No Root
  control) is active per research device profile.
- **FR-011**: The system MUST support configuring user-supplied custom kernel artifacts and
  user-space management components specifically for KernelSU Next on compatible research devices,
  acknowledging that ordinary production Play Store system images that prohibit custom kernels need not
  permit research root.
- **FR-012**: The system MUST verify KernelSU Next activation via an authorized guest-scoped elevated
  command that confirms root privilege execution and KernelSU Next identity specifically, while
  ensuring host system isolation is strictly preserved; privileged debugging access alone MUST NOT be
  accepted as proof of KernelSU Next activation; and the system MUST support user-selected disabling
  (transitioning observed state to inactive while preserving installed components) or complete removal
  (transitioning to removed).
- **FR-013**: The system MUST support deploying, starting, monitoring, stopping, and completely
  removing Frida dynamic instrumentation on a running research device using user-supplied compatible
  tool packages.
- **FR-014**: The system MUST verify Frida readiness through an active runtime check and demonstrable
  execution of non-destructive function tracing on an authorized sample application, reporting status
  as available awaiting verification until application observation succeeds, and MUST support complete
  removal of Frida without affecting unrelated tools or user application data.
- **FR-015**: The system MUST support deploying, enabling, observing, disabling, and removing an
  Xposed-compatible framework itself under a selected compatible provider on a research device,
  tracking reboot-dependent activation requirements conditionally, and requiring framework capability
  verification before reporting ready status.
- **FR-016**: The system MUST support installing, enabling, configuring execution scope, disabling,
  and removing Xposed-compatible modules using user-supplied module artifacts, verifying module
  capabilities before reporting ready status.
- **FR-017**: The system MUST enforce application-level scoping for Xposed-compatible modules, ensuring
  that declared behavioral modifications occur exclusively in selected target applications, while
  control applications and non-target guest environments retain baseline behavior.
- **FR-018**: The system MUST support simultaneous operation of KernelSU Next, Frida, and a
  compatible Xposed-compatible framework in a single virtual guest device per backend across both
  Android Emulator and Cuttlefish without functional collision or mutual degradation, establishing
  joint three-tool acceptance per backend.
- **FR-019**: The system MUST provide clean, user-initiated disabling and complete removal of research
  tools, modules, and frameworks, ensuring all injected hooks are deactivated, guest disk state is
  cleaned, and no orphaned host background processes remain.
- **FR-020**: The system MUST handle cancellation, device disconnection, and guest crashes during tool
  operations by gracefully terminating in-flight tasks, cleaning temporary staging files, reporting
  partial or uncertain committed state, and accurately updating tool status.
- **FR-021**: The system MUST support restoring a research device to an identified working baseline
  state following experimental failure or bootloop when a verified baseline exists and user authorization
  is granted; when baseline materials are unverified or missing, the system MUST safely refuse automated
  recovery without deleting or recreating the device.
- **FR-022**: The system MUST record, export, and import comprehensive experiment provenance records
  (including backend type, guest OS release, kernel build identifier, tool versions, module lists with
  cryptographic fingerprints, and launch parameters) to allow reconstructing an identical research
  environment on a new compatible device instance of the same backend, and MUST refuse reuse when
  prerequisites or architecture mismatch.
- **FR-023**: The system MUST record the cryptographic fingerprint and provenance for all user-supplied
  kernel images, packages, and tool binaries, compare against supplied expected fingerprints when
  available, block deployment on mismatch, and explicitly report missing trust metadata as unverified
  trust while permitting user-authorized experimental preparation under guarded recovery policy.
- **FR-024**: The system MUST declare backend capabilities and research tool compatibility based
  strictly on empirical verification on controlled real environments, clearly indicating experimental
  or unsupported status rather than claiming unverified feature parity, and establishing that support
  is a future delivery target rather than an active implementation claim.
- **FR-025**: The system MUST support defining, importing, configuring, and applying extended research
  profiles that incorporate optional kernel research capabilities and alternative root implementations
  from the reference capability catalog, utilizing user-supplied compatible kernel artifacts, modules, and
  runtime packages under explicit user authorization.
- **FR-026**: The system MUST support explicit user selection of alternative root implementations
  (including upstream KernelSU and ReSukiSU, source-specific opt-ins SukiSU-Ultra and KowSU, or No Root
  as a control baseline) as distinct profile choices alongside the default KernelSU Next baseline, MUST
  strictly verify and report the specific selected root identity, MUST permit only one selected root
  implementation per guest profile, and MUST require an explicit compatible kernel/image change with
  recovery baseline verification to switch root mechanisms.
- **FR-027**: The system MUST evaluate kernel family and, where applicable, Kernel Module Interface
  (KMI) compatibility for user-supplied custom kernel artifacts based on declared interface evidence
  rather than Android OS release version alone; MUST reject known kernel family, architecture, or KMI
  mismatches before modifying the guest; MUST treat unknown or missing interface evidence as
  unverified/experimental under guarded policy (requiring explicit authorization and baseline recovery
  per FR-008); and MUST NOT treat a successful upstream compilation or build label as proof of virtual
  machine boot compatibility.
- **FR-028**: The system MUST enforce dependency and mutual conflict constraints among catalog
  capabilities prior to deployment (including rejecting documented conflicting combinations such as KowSU
  with SUSFS), displaying explanatory conflict diagnostics and refusing to stage or deploy conflicting
  configurations.
- **FR-029**: The system MUST distinguish between capabilities requiring absent kernel-level support
  and capabilities with support already present in the running guest image; when required kernel-level
  support is absent, the system MUST require a compatible kernel or image replacement and guest restart
  (tracking `pending-reboot` state) before reporting the capability as available, and MUST NOT claim
  absent compiled capabilities can be enabled via runtime toggles; when the requisite support is already
  present, the system MUST allow exercising supported runtime enable, configuration, or disable
  controls according to provider capabilities.
- **FR-030**: When an extended research profile includes physical-hardware-dependent features (such as
  Baseband Guard partition protection) on a virtual research device lacking the target physical hardware
  or cellular baseband partitions, the system MUST report the capability condition as not applicable,
  MUST NOT simulate artificial hardware availability, and MUST NOT access host storage partitions or
  modify host security policies.
- **FR-031**: The system MUST verify enabled optional capabilities through authorized, benign, guest-scoped
  observable experiments compared against a no-root or feature-disabled control baseline, MUST record the
  configuration, observed status, artifact fingerprints, and dependency state of all selected extended
  capabilities in the experiment provenance record, and MUST support clean deactivation or restoration to
  a verified baseline.
- **FR-032**: The system MUST provide complete command-line automation coverage for 100% of in-scope
  research operations (including backend preflight diagnostics, device lifecycle management, research profile
  configuration and application, artifact validation, toolchain and extended capability deployment, scoped
  experiment verification, provenance export/import, baseline recovery, and status inspection), providing
  discoverable operational metadata and a machine-readable capability inventory without requiring an
  interactive terminal session or TUI execution.
- **FR-033**: The automation interface MUST accept all required operational parameters through
  terminal-independent, non-interactive invocation inputs and address virtual devices and artifacts using
  stable, unique identifiers; when required inputs are missing, ambiguous, or reference non-existent resources
  (such as duplicate display names without unique identifier disambiguation), the system MUST safely reject the
  invocation before modifying any state and without prompting for interactive terminal input.
- **FR-034**: The automation interface MUST provide structured, machine-readable output representations for all
  operations, accompanied by a stable documented machine-result and process outcome contract: successful
  mutations, already-satisfied declarative configurations, and truthful read-only inspection queries (including
  queries reporting that a capability is unavailable or unsupported) MUST yield a documented successful
  process status with distinguishable outcome properties in the structured payload; whereas attempts to
  execute unsupported actions, invalid or missing inputs, authorization refusals, runtime execution failures,
  operation cancellations, and timeouts MUST yield documented non-success process classifications;
  human-readable diagnostic wording MAY evolve without altering or breaking machine-readable outcome contracts;
  and the system MUST separate diagnostic, logging, and progress streams from structured operational results so
  that human-readable text or terminal formatting never corrupts machine parsing.
- **FR-035**: The system MUST enforce identical safety policies across automation and interactive surfaces,
  permitting destructive operations (including device deletion, storage wiping, partition flashing, and
  baseline restoration) to be executed non-interactively only when the caller supplies explicit, resource-scoped
  authorization identifying the specific target instance and action; in the absence of explicit authorization,
  the system MUST safely refuse the operation without modifying state and without blocking on interactive prompts.
- **FR-036**: The system MUST track and report discrete operation states (including in-progress, completed,
  failed, partially-committed, cancellation-pending, cancelled, and timed-out) with caller-bounded execution
  deadlines; when an execution deadline expires, the system MUST report a timeout outcome accompanied by the
  actual known operational state (stopped, continuing, or unknown) and any partial changes; when cancellation
  is explicitly requested, the system MUST acknowledge the cancellation request, attempt to reach a verified safe
  boundary, and confirm actual cessation only after reaching that safe boundary (or report cancellation-pending with
  actual known state if caller-bounded waiting expires before the boundary is reached); stopping or disconnecting an
  observation or monitoring task MUST NOT cancel the underlying guest task or report it as stopped; and partial-state
  reporting MUST accurately reflect committed guest modifications without promising automated reversal of committed side
  effects.
- **FR-037**: The system MUST define and enforce deterministic repeat semantics across operations: applying
  an identical declarative desired-state research profile to an instance whose observed condition already
  matches the specification MUST report an already-satisfied outcome without performing redundant kernel
  flashes, guest restarts, or destructive file operations; whereas explicit action commands (including device
  reboot, storage wipe, and experiment execution) MUST execute as distinct operations and MUST NOT be silently
  repeated or retried by the system upon failure.
- **FR-038**: The system MUST provide an interactive human terminal user interface (TUI) that exposes the same
  underlying research capabilities, operational statuses, validation rules, and safety boundaries as the
  automation interface, providing human-oriented navigation, real-time visual progress indication,
  pending-reboot status tracking, and explicit interactive confirmations for destructive actions.
- **FR-039**: The system MUST enforce observable cross-interface consistency between automation and interactive
  interfaces, ensuring that research profiles, provenance records, and configurations created or modified
  through either interface can be read and used by the other; equivalent operations across interfaces MUST
  observe the same actual guest state and safety policy, with zero requirement for hidden or active TUI
  selections.

### Operational Surface Coverage

The following table specifies the operational coverage across both first-class interfaces, ensuring that
every in-scope research capability has a complete, non-interactive automation path (CLI) and an equivalent
interactive human-facing surface (TUI) sharing the same underlying rules, states, and safety guarantees:

| Operational Family                           | Scope & Core Operations                                                                                                                                                                                                                                           | Unattended Automation (CLI) Purpose                                                                                                                                                 | Interactive Human (TUI) Purpose                                                                                                                                         |
| :------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Backend & Capability Discovery**           | Local host preflight diagnostics, hypervisor capability detection, backend readiness reporting, and machine-readable capability inventory.                                                                                                                        | Non-interactive environment validation, pipeline prerequisite gating, structured capability introspection, and automated skipping of unsupported backends.                          | Visual host compatibility summary, missing prerequisite guidance, interactive backend status indicators, and contextual setup advice.                                   |
| **Device Lifecycle Management**              | Instance creation, registration, startup, graceful/forced stop, restart, storage wipe, deletion, and detailed inspection.                                                                                                                                         | Headless provisioning, deterministic teardown, scriptable batch lifecycle transitions, explicit resource-scoped authorization for wipes/deletions, and stable unique ID addressing. | Interactive device listing, lifecycle controls, instance inspection, clear backend indication, and explicit confirmations for destructive actions.                      |
| **Artifact Inspection & Research Profiles**  | Cryptographic fingerprinting, interface compatibility validation, profile selection, declarative configuration, import, export, and desired-state apply.                                                                                                          | Automated artifact integrity verification, pipeline profile deployment, idempotent desired-state application, and export/import without manual entry.                               | Interactive artifact selection, visual compatibility and fingerprint inspection, profile configuration, and clear distinction of desired vs. observed state.            |
| **Toolchain & Capability Lifecycle**         | Privilege mechanisms (KernelSU Next / alternative roots), Frida dynamic instrumentation, Xposed-compatible frameworks and modules, and extended catalog capabilities (prepare, enable, configure, start, stop, disable, remove, and observed condition tracking). | Headless toolchain orchestration, scriptable module scoping, conditional reboot detection, non-interactive runtime configuration, and post-restart verification gating.             | Interactive toolchain navigation, module scope selection, pending-reboot notification, real-time operational state display, and guided removal workflows.               |
| **Scoped Experiments & Diagnostic Evidence** | Authorized guest-scoped test execution, Frida function tracing observation, scoped module behavioral verification, guest log streaming, and diagnostic extraction.                                                                                                | Automated assertion of guest root privileges, machine-readable tracing telemetry collection, structured test verdict output, and exit status classification.                        | Live telemetry monitoring, interactive tracing observation, control vs. target comparison, and diagnostic inspection.                                                   |
| **Provenance & Profile Reproduction**        | Complete environment audit capture, cryptographic artifact recording, immutable provenance record export, and reproducible re-application on compatible target instances.                                                                                         | CI/CD test artifact archival, programmatic environment cloning, and automated regression testing across identical environments.                                                     | Provenance summary display, difference inspection between target instance and imported profile, and interactive environment replication.                                |
| **Baseline Recovery**                        | Working baseline identification, verification check, post-failure disk/kernel restoration, safe refusal on missing baseline, and recovery verification.                                                                                                           | Unattended automated recovery after failed experiment runs, pipeline reset without device recreation, and structured refusal when baselines are missing.                            | Interactive recovery guidance, baseline selection, progress tracking during restoration, and safety warnings when baseline materials are missing or unverified.         |
| **Operation Status & Cancellation**          | Bounded operation execution, progress observation, caller-defined timeouts, cancellation signals, partial state reporting, and safe boundary termination.                                                                                                         | Non-blocking or caller-bounded waiting, deterministic exit codes on timeout/cancellation, truthful reporting of partial state, and separation of machine results from diagnostics.  | Visual progress tracking, non-blocking navigation during long operations, cancellation requests, visual notification of partial state, and cancellation acknowledgment. |

### Key Entities _(include if feature involves data)_

- **Research Device**: A virtual Android instance managed under either the Android Emulator or
  Cuttlefish backend. Represents the primary operational target, characterized by a unique system
  identifier, backend type, user display name, guest OS version, CPU architecture, lifecycle state, and
  associated research profile.
- **Backend Capability Profile**: A declarative specification of virtualization capabilities, host
  operating system constraints, hypervisor prerequisites, and tooling compatibility for a specific
  backend.
- **Research Profile**: A composite specification applied to a device, defining the desired state for
  privilege management (selecting exactly one implementation: KernelSU Next as default baseline, or an
  explicit choice of KernelSU, ReSukiSU, SukiSU-Ultra, KowSU, or No Root as a control baseline), dynamic
  instrumentation (Frida), application behavior mediation (Xposed-compatible framework and modules),
  optional extended kernel research capabilities selected from the reference catalog (such as root
  visibility research, mount mediation, advanced networking, extended filesystem attributes, eBPF/BTF
  observability, synchronization, or container runtimes), and associated target application scopes.
- **Research Tool State**: An entity tracking an individual research tool, framework, or extended kernel
  capability's operational condition on a device. It distinguishes the user's intended outcome from
  verified availability, activity, pending restart, not applicable (when target virtual hardware or
  partitions are absent), inactive, removed, or failure, together with observed version, interface details,
  and diagnostics.
- **Artifact Package**: A user-supplied file used in research workflows (custom kernel binary with specific
  compiled capabilities, tool package, framework component, module package, or container runtime bundle).
  Attributes include package type, intended capability, cryptographic fingerprint, target CPU architecture,
  target Android API range, interface compatibility metadata, and local file path.
- **Reference Capability**: A declarative entry in the reference capability catalog defining an optional
  kernel or guest research feature, its delivery type (user-installable tool/module, compiled-in kernel
  feature, or in-guest runtime component), required prerequisites and interface constraints, mutual
  conflict rules, virtual hardware applicability, and benign verification criteria.
- **Recovery Baseline**: A designated reference point for a research device (such as a base system
  disk image or initial clean state) that allows the device to be restored to a known-good configuration
  after an experimental failure.
- **Experiment Provenance Record**: An immutable audit log detailing the complete environment
  configuration used in an experiment, including backend identifier, guest OS build, kernel version,
  active tools and optional capabilities, module fingerprints, launch parameters, and execution timestamp.

## Success Criteria _(mandatory)_

### Measurable Outcomes

> **Reference Acceptance Set Definition**: All measurable outcomes in this section are evaluated
> against a defined reference acceptance cohort of 10 virtual research device instances (5 per
> backend; simultaneous execution is not required). Each backend includes at least one pinned joint
> baseline toolchain configuration (kernel privilege escalation, dynamic instrumentation, and a compatible
> application behavior mediation provider operating together) and a verified usable baseline. Across
> compatible reference guest instances within the cohort, selected extended research profiles demonstrate
> an alternative kernel privilege escalation implementation and at least one capability per included
> non-hardware-specific optional family (features are not required to run on both backends if a backend
> is explicitly unsupported for that feature, and unsupported pairs are explicitly documented). A dedicated
> compatibility-negative test set exercises incompatible privilege implementations, mismatched kernel
> family/interface versions, missing prerequisites, attempts to activate absent kernel capabilities
> without required kernel replacement and restart, and physical-hardware-dependent features on virtual
> hardware lacking required partitions. Host resource baselines, image/artifact fingerprints, and operation
> deadlines are fixed and recorded prior to measurement. An automation acceptance matrix exercises all
> 8 operational families across supported backend configurations in an unattended environment, covering all
> listed applicable operations per cell and paired with the 20-case negative evaluation set and repeated
> execution test cases. Within this reference cohort:
>
> - Each listed Edge Case is exercised at least once per applicable backend.
> - Each reboot-required and no-reboot activation, deactivation, and removal path is exercised.
> - Each baseline-present and baseline-missing recovery path is exercised.
> - Each non-hardware-specific extended capability family is demonstrated on at least one compatible instance.
> - Each operational family in the automation matrix is executed via both unattended and interactive surfaces.

- **SC-001**: Across the reference acceptance cohort of 10 virtual research device instances
  spanning both virtualization backends, 100% of devices display distinct, unambiguous backend and
  instance identifiers with zero naming collisions or cross-device command misdirection.
- **SC-002**: In the validated reference configuration for each virtualization backend, researchers can
  instantiate and operate a combined security research environment containing kernel privilege
  escalation, dynamic binary instrumentation, and modular application behavior mediation working
  simultaneously, achieving a 100% success rate on non-interfering benign test workloads across 5
  consecutive trial runs, and successfully repeating the declared benign outcomes after profile reuse on
  a fresh compatible instance of the same backend.
- **SC-003**: During 100% of guest privilege and application hooking evaluation scenarios in the
  reference cohort, all privileged operations and dynamic interceptions remain strictly contained
  within the designated guest virtual machine and targeted application, producing zero unauthorized host
  privilege escalations, zero host security policy modifications, zero writes outside approved research
  directory resources, and zero observable behavioral changes in non-target control applications.
- **SC-004**: In 100% of evaluated preflight incompatibility or integrity check scenarios in the
  reference cohort, rejection occurs before any protected guest mutation is initiated; in 100% of
  runtime failure scenarios, the system reports failed or uncertain status by the disclosed operation
  deadline, with zero silent fallbacks and zero unhandled process crashes.
- **SC-005**: Following an induced guest boot failure in a reference device with a verified working
  baseline, 100% of attempted recovery operations with user authorization successfully restore the
  device to its baseline state within 3 minutes (timing measured from explicit user authorization to
  operational readiness of the baseline guest); when baseline materials are missing or unverified, 100%
  of recovery attempts are safely refused without automated deletion or data loss.
- **SC-006**: In 100% of tool and framework lifecycle transitions that require a guest restart under
  their provider rules, the interface accurately reflects pending-reboot status until a verified
  post-restart check confirms the requested active, inactive, or removed state; transitions without
  a restart immediately reflect the verified outcome, with zero false-ready reports.
- **SC-007**: Across a test sample of 50 consecutive interactive navigation commands and 10 stop or
  cancellation requests per backend executed while background preparation, staging, or device launching
  is underway, navigation inputs receive visual acknowledgment within 100 milliseconds and stop or
  cancellation requests receive visual acknowledgment within 200 milliseconds, while the underlying
  operation halts at its documented safe boundary.
- **SC-008**: Across 20 reference lifecycle trial runs on existing standard virtual devices operating
  without research tooling, device listing, startup, execution, inspection, and shutdown proceed
  identically to baseline operation with zero functional regressions or performance degradation.
- **SC-009**: In a user evaluation sample of 5 representative mobile security researchers, at least 4
  out of 5 successfully complete the defined preparation, verification, observation, module scoping,
  profile reuse, and recovery tasks on prepared reference environments using product guidance without
  requiring manual out-of-band host intervention.
- **SC-010**: Across compatible virtual guest reference instances, 100% of tested extended research
  profiles incorporating an explicitly selected alternative kernel privilege escalation mechanism
  (distinct from the default baseline) verify and report the selected implementation's identity
  specifically, achieve elevated execution on an authorized guest test command, and successfully
  restore the baseline kernel state upon profile deactivation, with zero silent substitution.
- **SC-011**: For each included non-hardware-specific optional capability family (privilege visibility
  research, mount mediation, advanced networking, extended filesystem attributes/access control lists,
  in-kernel observability/tracing, kernel synchronization primitives, and in-guest container runtime),
  at least one capability is demonstrated on a compatible reference guest instance, producing a verified
  benign observable effect without destabilizing guest execution, and restoring the verified baseline
  state upon deactivation or removal with a 100% success rate across trial runs.
- **SC-012**: In 100% of evaluation scenarios involving physical-hardware-dependent partition protection
  features evaluated on virtual research devices lacking the underlying physical hardware partitions,
  the system reports the capability condition as not applicable with zero simulated false-readiness,
  zero attempts to access host storage partitions, and zero host security policy modifications.
- **SC-013**: In 100% of trials within the reference compatibility-negative test set (exercising
  incompatible privilege tool combinations, mismatched kernel family or module interface versions,
  missing declared dependencies, attempts to activate absent kernel capabilities without required kernel
  replacement and restart, and absent target hardware), the system blocks or gates the operation before
  guest modification occurs, requires compatible artifacts with user approval and restart for supported
  kernel-level changes, provides an explanatory diagnostic detailing the incompatibility or prerequisite,
  and explicitly documents unsupported backend or component pairs without silent substitution.
- **SC-014**: Across a finite automation acceptance matrix evaluating all 8 operational families across both
  supported backends (forming 16 distinct family/backend evaluation cells), 100% of applicable operations
  listed in the Operational Surface Coverage table are exercised at least once through the unattended interface
  without requiring an interactive terminal session, keystroke simulation, or interactive screen scraping; each
  cell successfully demonstrates every in-scope operational capability (including preflight checks, lifecycle
  transitions, artifact validation, profile configuration/application, baseline three-tool deployment and
  observation, scoped experiments, provenance export/import, baseline recovery, and bounded waiting/cancellation);
  explicitly unsupported component/backend pairs yield truthful refusal outcomes and do not count as successful
  capability implementations; known-good finite reference workflows reach their expected completed state within
  declared completion deadlines; and bounded observation or cancellation waits return truthful outcome and current
  operational state by the caller's wait deadline without fabricating task completion.
- **SC-015**: In 100% of trials across a defined 20-case negative evaluation set (consisting of exactly 10
  enumerated scenarios evaluated on each backend: (1) missing destructive authorization, (2) missing required
  input, (3) ambiguous target display name, (4) unavailable host or backend prerequisite, (5) incompatible
  artifact package, (6) failed guest verification check, (7) timeout before guest modification, (8) timeout
  after partial modification, (9) stopped observation, and (10) explicit operation cancellation), the unattended
  interface returns within its bounded wait deadline without hanging or awaiting interactive terminal input,
  produces parseable structured outcome data with the corresponding documented outcome-appropriate process status
  (where rejected, failed, timed-out, or cancelled operations yield non-success process classifications, while
  deliberate completion of an observation task yields successful process status), and produces zero unauthorized
  or unintended modifications to protected guest or host resources; for stopped observation, the underlying guest
  operation is not required to stop and MUST NOT be reported as stopped or cancelled; and for operation cancellation,
  the system acknowledges the request, confirms actual cessation only after reaching a verified safe boundary (or
  reports cancellation-pending with actual known state if the wait deadline expires prior to that boundary),
  without promising automatic reversal of authorized committed steps.
- **SC-016**: In cross-surface parity evaluations across all reference configurations, comparing equivalent
  research workflows through both the unattended interface and the interactive interface on the same settled
  reference state confirms identical target instance identification, equivalent semantic toolchain operational
  states, matching diagnostic meaning, and consistent safety authorization decisions; research profiles created
  or exported via either interface are 100% reusable and actionable in the other interface in both directions;
  across 10 trial repetitions of declarative desired-state profile applications on already-matching devices
  (5 trials per backend), 100% of runs report an already-satisfied outcome with a documented successful process
  status, zero duplicate guest mutations, zero redundant kernel flashes, and zero unnecessary reboots; and
  explicit action commands execute exactly once per authorized invocation and are never automatically retried
  upon failure.

These outcome targets supplement, and do not relax, the constitution's responsiveness requirements.

## Assumptions

- **Scope Boundaries & Defaults**:
  - **Future Delivery Target**: Support for the research backends and tooling described in this
    specification represents a development target for upcoming implementation iterations. It is not an
    active claim of current repository capabilities.
  - **Development Priority — Android First**: Android Emulator and Cuttlefish research capabilities,
    including complete unattended CLI and human-facing TUI workflows, take development priority
    before new iOS/Apple research backends. iOS work is not a dependency or acceptance gate for
    this Android feature. This ordering does not narrow or relax either Android backend's existing
    baseline or extended-capability acceptance. The rationale is the user's tooling-availability priority,
    not verified universal compatibility.
  - **Locally Managed Backends**: Research devices are hosted and managed locally on the researcher's
    workstation. Remote, cloud-hosted, or multi-host orchestration is excluded from this feature.
  - **Host Platform Support Boundaries**: Android Emulator is supported across Linux, macOS, and
    Windows workstations matching established platform criteria. Cuttlefish is supported exclusively on
    local Linux workstations (supporting x86_64 and arm64 architectures) providing native hardware
    virtualization capabilities and required host packages as a virtual machine environment (not native
    macOS or Windows, and not simply containerized).
  - **Dual Surface Architecture & Interface Roles**: The command-line interface (CLI) serves as the
    complete, unattended automation interface for headless execution, scripting, CI pipelines, and
    programmatic orchestration, exposing all in-scope research operations without requiring an interactive
    terminal session or TUI navigation. The terminal user interface (TUI) serves as the interactive
    human-facing surface for visual discovery, interactive navigation, real-time progress monitoring, and
    guided safety confirmations. Both surfaces operate against the exact same underlying operational
    capabilities, validation rules, status models, error outcomes, and safety policies. Unattended automation
    workflows must never depend on TUI keystroke emulation, terminal sessions, or screen scraping, while
    interactive human workflows must not have exclusive access to research management capabilities.
  - **User-Supplied Artifacts & Integrity Policy**: Researchers provide compatible guest system images,
    custom kernel binaries, tool packages, and module packages. When artifacts lack expected trust
    metadata, they are treated as unverified trust (not corrupt) and permitted to proceed under guarded
    policy with explicit user authorization and verified baseline recovery available. The system validates,
    stages, and deploys these artifacts, but does not provide automated kernel compilation, source build
    farms, or third-party module repositories.
  - **User Reference Context & Capability Scope**: The repository `https://github.com/WildKernels/GKI_KernelSU_SUSFS`
    (specifically documentation in `README.md` at `https://github.com/WildKernels/GKI_KernelSU_SUSFS/blob/main/README.md`
    and `docs/build-from-fork.md` at `https://github.com/WildKernels/GKI_KernelSU_SUSFS/blob/main/docs/build-from-fork.md`)
    serves as reference context and technical inspiration for the extended research capabilities requested by the
    user (`mình cần develop Android Emulator và Cuttlefish, hỗ trợ KernelSUNext, frida, xposed v.v.v`). This reference
    illustrates kernel features, root flavors, and guest tools researchers commonly evaluate together. Citing this
    repository defines the scope of managed research capabilities; it does not indicate that upstream prebuilt binaries
    are validated for virtual devices, nor does it incorporate external build automation. Successful compilation
    does not guarantee successful guest boot, and compatibility requires matching kernel families and interfaces
    where applicable rather than userspace Android releases alone.
  - **Baseline Toolchain and Extended Profile Architecture**: The baseline requirement for both Android
    Emulator and Cuttlefish remains establishing at least one validated configuration where KernelSU
    Next, Frida dynamic instrumentation, and a compatible Xposed-compatible framework operate
    simultaneously without conflict. This baseline cannot be silently substituted or bypassed. In
    addition, the system supports extended research profiles incorporating optional kernel research
    capabilities and alternative root implementations from the reference capability catalog. Exactly one
    root implementation (or No Root as a control baseline) is permitted per guest profile; changing root
    implementations requires an explicit kernel/image replacement with baseline recovery available, never
    silent co-installation. Arbitrary unlisted tools or generic plugin marketplaces remain excluded, but
    the explicitly referenced tool families and their necessary supporting runtime prerequisites are in
    scope as managed research capabilities.
  - **Reference Extended Capability Catalog**:
    The following capability catalog outlines optional research feature families inspired by the reference
    repository, describing what researchers select and observe under bounded constraints:
    - **Root Implementation Flavors**: Researcher chooses between KernelSU Next (default baseline), upstream
      KernelSU, ReSukiSU, source-specific opt-ins (SukiSU-Ultra, KowSU), or No Root (control baseline). Delivered
      via compatible kernel support and user-space management tools; strictly one active root implementation per
      profile; switching roots requires an explicit compatible kernel replacement and guest restart under user
      authorization.
    - **Root Visibility & Privilege Concealment (SUSFS)**: Enables guest-scoped root-visibility mediation,
      ptrace leak mitigations, and path concealment research (susfs4ksu). Delivered via kernel-level capability
      support paired with in-guest configuration utilities; verified by observing guest privilege visibility
      differences against a control; gated against incompatible roots (e.g., KowSU); does not guarantee universal
      evasion of external detection.
    - **Mount Mediation Metamodules (NoMount / Mountify)**: Provides advanced filesystem mount mediation and overlay
      management for guest research modules. Delivered as in-guest runtime modules operating atop compatible root
      hooks; verified through guest mount behavior and module overlay isolation.
    - **Partition Protection (Baseband Guard)**: Provides write-protection for cellular modem and radio flash
      partitions. Delivered via dedicated partition protection logic; when the target virtual device lacks
      cellular baseband hardware or radio partitions, the system truthfully reports the feature as not applicable,
      avoiding simulated readiness or host partition access.
    - **Advanced Networking Subsystems (WireGuard, BBR, IPSet, CIFS)**: Supports in-kernel VPN tunneling
      (WireGuard), congestion control algorithms (BBR), packet filtering tables (IPSet), and network filesystem
      shares (CIFS). Delivered through kernel-level networking support and guest network utilities; absent kernel
      support requires a compatible kernel replacement and restart, while present capabilities allow runtime
      configuration and observable guest network behavior.
    - **Extended Filesystem Attributes & Access Control (TMPFS xattr / POSIX ACLs)**: Supports extended attribute
      storage and POSIX access control lists on temporary in-memory filesystems. Delivered through kernel
      filesystem capability support; absent support requires a compatible kernel replacement, while present
      support enables runtime attribute assignment and observable permission enforcement in the guest.
    - **In-Kernel Observability & Tracing (BTF, eBPF, FUSE-BPF)**: Provides kernel data structure introspection
      (BTF), in-kernel event tracing (eBPF), and user-space filesystem mediation (FUSE-BPF). Delivered via kernel
      observability subsystems and user-space tracing tools; verified by observing guest event telemetry; failures
      during experimental tracing are isolated to protect host and other guests while retaining diagnostics.
    - **Kernel-Assisted Synchronization (NTSync)**: Provides specialized synchronization primitives for
      multithreaded guest workloads. Delivered via kernel synchronization subsystem support; verified by running
      guest synchronization workloads and observing thread coordination performance.
    - **In-Guest Container Runtimes (DroidSpaces)**: Provides isolated application containerization and execution
      spaces inside the Android guest. Delivered as an in-guest user-space runtime utilizing available guest
      isolation capabilities; verified through container creation and observed application containment within the
      guest.
  - **Dual Backend Validation Target**: Both Android Emulator and Cuttlefish must have at least one
    validated configuration in which all three baseline research tool families can run together in a guest
    instance.
  - **Non-Regression & Image Permissions Invariant**: Existing standard Android virtual device and
    non-research workflows remain completely functional without requiring research features. Ordinary Play
    Store system images need not permit research root or custom kernels, and privileged debugging access
    alone does not satisfy research root verification.
- **Explicit Exclusions**:
  - **Apple iOS Platforms**: iOS devices, iOS simulators, and new darwin-vm/Inferno integrations are
    outside this Android research feature. New iOS/Apple research capabilities are deferred to a later
    development phase rather than removed from long-term project direction. Existing iOS Simulator
    workflows supported by the project remain unchanged by this Android-first sequencing.
  - **Physical Devices & Flashing**: Physical tethered Android devices and physical device partition
    flashing are out of scope; the feature governs virtual Android environments only.
  - **Host Privilege Modification**: Host operating system rooting, kernel patching, hypervisor security
    tampering, or disabling host security controls are strictly prohibited.
  - **Remote Cloud Orchestration**: Multi-node cloud orchestration, remote device farms, and distributed
    hypervisors are excluded.
  - **Broad Hardware Emulation Fidelity**: Simulation of specialized physical hardware peripherals (e.g.,
    physical cellular baseband modems, hardware secure elements, physical SIM cards, or proprietary
    sensors) is out of scope. Features targeting physical hardware partitions (such as Baseband Guard) must
    report not applicable on virtual devices lacking required partitions rather than simulating physical hardware.
  - **Automated Kernel Compilation & Build Orchestration**: This feature does not run source-level kernel
    compilation pipelines, upstream build workflows, or build farms. Researchers provide compatible artifacts for
    the new research profiles; existing standard device image acquisition workflows remain unchanged.
  - **Automated Exploit Generation & External Attacks**: Automated exploit synthesis, vulnerability fuzzing
    pipelines, and launching attacks against unowned external targets are strictly excluded. Authorized guest-scoped
    root-visibility, namespace separation, and privilege concealment research (such as configuring and evaluating
    SUSFS in a controlled guest environment) is in scope as a research capability, but does not constitute a
    guarantee to defeat arbitrary, evolving third-party anti-tamper or anti-root detection mechanisms.
  - **Unlisted Third-Party Tools & Marketplaces**: Generic plugin marketplaces, arbitrary unverified
    package repositories, commercial proprietary tools, and unlisted rooting tools outside the explicitly
    supported catalog (e.g., Magisk or APatch) are excluded. The explicitly referenced tool families in the
    capability catalog and their strictly required supporting prerequisites are in scope as managed
    research capabilities.
- **Dependencies**:
  - **Host Virtualization Capabilities**: Functional local hardware virtualization on Linux; host
    virtualization entitlements on macOS; Windows hypervisor platform features for Android Emulator.
  - **Device Communication Prerequisite**: The host and selected guest must provide working
    communication needed for the requested management and verification operations.
  - **Artifact Availability**: Availability of user-supplied kernel images matching the guest Android
    version, kernel family, and architecture, compatible Frida tool packages, and compatible
    Xposed-compatible framework and module packages.
  - **Guest System Compatibility**: Android releases and system images possessing compatible kernel and
    runtime structures for the selected root mechanisms, extended kernel capabilities, and
    Xposed-compatible frameworks.
