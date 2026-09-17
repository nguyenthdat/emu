# CLI Contract: Android Research Backends & Automation Interface

**Version**: 1.0.0  
**Feature Branch**: `specs/001-add-android-research-backends`  
**Status**: Draft  
**Authority**: `specs/001-add-android-research-backends/research.md` and `specs/001-add-android-research-backends/spec.md`

---

## 1. Executive Summary & Invocation Architecture

### 1.1 Single-Binary Command Routing (Decision D-01)

The `emu` project maintains a single binary. Interactive TUI workflows launch when `emu` is invoked without subcommands. Headless, non-interactive research automation is completely routed through the `emu research` subcommand hierarchy.

```
emu [GLOBAL_FLAGS] research <FAMILY> <SUBCOMMAND> [COMMAND_FLAGS]
```

- **Headless Bypassing**: `emu research ...` subcommands instantiate concrete backend managers directly and strictly bypass `App::new()`, preventing background UI thread spawning, polling loops, or raw terminal allocation.
- **Private Worker Execution (Decision D-05)**: Long-running asynchronous operations (kernel flashing, boot monitoring, multi-stage recovery) are executed via a private, finite child process invocation:
  ```
  emu __worker --operation-id <OPERATION_ID>
  ```
  The worker is not a public CLI subcommand or long-running system daemon; it is a single-purpose child process spawned by the coordinator holding an exclusive operation lock.

### 1.2 Stream Separation (FR-034)

The automation interface strictly separates structured machine-readable results from asynchronous diagnostics, guest log output, and progress telemetry:

- **`stdout`**: Reserved exclusively for the finite JSON output contract (`OutputEnvelope`). When `--json` is specified, `stdout` produces exactly one valid JSON object upon process termination.
- **`stderr`**: Used for diagnostic progress, warning messages, and operational events. When `--json` is active, `stderr` emits append-only JSONL envelopes (`StreamLogEnvelope`), ensuring automated log parsers never conflict with terminal formatting or ANSI escape codes.

---

## 2. Process Exit Code Contract

Exit codes are deterministic, categorized, and standardized across all 8 operational families (FR-034, SC-015):

| Exit Code | Classification                                   | Trigger Scenarios                                                                                                                                                                                                                                                                                                                                                                                      |
| :-------: | :----------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  **`0`**  | **`success` / `already_satisfied` / `accepted`** | - Operational mutation completed successfully.<br>- Read-only inspection query evaluated truthfully (including queries reporting that a backend or tool is unsupported/unavailable).<br>- Declarative desired-state command applied to a device whose observed state already matches (FR-037; zero redundant mutations).<br>- `--dry-run` generated a valid `MutationProposal` with `proposal_digest`. |
|  **`1`**  | **`failed` (Runtime Failure)**                   | - Runtime execution error (guest command crash, ADB socket broken, unhandled supervisor error).<br>- Active verification probe failed (e.g. Frida hook failed to capture telemetry, supercall node not found).                                                                                                                                                                                         |
|  **`2`**  | **`invalid_input`**                              | - Missing required flags or invalid argument types.<br>- Target display name is ambiguous across backends without unique identifier disambiguation (FR-033).<br>- Referenced file, profile, or artifact does not exist.                                                                                                                                                                                |
|  **`3`**  | **`unsupported`**                                | - Attempting to execute an operation on an unsupported host (e.g. launching Cuttlefish on macOS/Windows; FR-004).<br>- Hardware-dependent capability requested on virtual target (e.g. Baseband Guard on Emulator; FR-030).                                                                                                                                                                            |
|  **`4`**  | **`auth_refused`**                               | - Destructive operation (wipe, delete, kernel swap, baseline restore) invoked without valid `--authorize sha256:<digest>` (FR-006, FR-035).<br>- Provided authorization digest is expired or does not match calculated proposal.                                                                                                                                                                       |
|  **`5`**  | **`conflict`**                                   | - Mutually conflicting capabilities selected in profile (e.g. KowSU + SUSFS, NoMount + Mountify; FR-028).<br>- Multiple competing root mechanisms attempted in a single profile (FR-010).<br>- Target port or isolated instance directory collision (Gate G-07).                                                                                                                                       |
| **`124`** | **`timeout` / `cancellation_pending`**           | - Operation deadline expired before completion; underlying guest state is `stopped`, `continuing`, or `unknown` (FR-036).<br>- Cancellation requested but safe transaction boundary was not reached before caller wait deadline.                                                                                                                                                                       |
| **`130`** | **`cancelled`**                                  | - Explicit cancellation signal (SIGINT / Ctrl+C or `operation cancel`) successfully halted the operation at a verified safe transaction boundary (FR-036).                                                                                                                                                                                                                                             |

---

## 3. Standard JSON Envelopes

### 3.1 OutputEnvelope (`stdout`)

Every command invoked with `--json` outputs exactly one `OutputEnvelope` object on `stdout`:

```json
{
  "$schema": "https://emu.rs/schemas/v1/research.schema.json#/definitions/OutputEnvelope",
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
  "data": {
    "device_id": "emulator:avd-sec-01",
    "lifecycle_state": "running",
    "adb_serial": "emulator-5554"
  },
  "error": null
}
```

#### Field Specifications:

- `status`: High-level operational classification:
  - `"success"`: Request achieved its goal (mutated state, read query, or already satisfied).
  - `"already_satisfied"`: Declarative request matched observed state; 0 actions taken.
  - `"accepted"`: Dry-run accepted and proposal digest generated.
  - `"failed"`: Runtime or verification failure.
  - `"rejected"`: Refused preflight due to validation, input, auth, or support constraints.
  - `"timed_out"`: Execution deadline expired.
  - `"cancelled"`: Halted at safe transaction boundary.
- `outcome`: Distinguishable machine-readable outcome identifier:
  - `"completed"` | `"already_satisfied"` | `"proposal_created"` | `"auth_refused"` | `"invalid_input"` | `"unsupported"` | `"conflict"` | `"timeout"` | `"cancelled"` | `"execution_failed"`
- `operation_id`: Unique tracking ID prefixed with `op_`. For synchronous queries, represents the transient evaluation ID.
- `data`: Command-specific result payload (object). Null or empty on terminal errors.
- `error`: Structured error object, or `null` on success:
  ```json
  {
    "code": "AUTH_REQUIRED",
    "message": "Destructive operation requires explicit authorization",
    "details": {
      "target_device_id": "emulator:avd-sec-01",
      "affected_resources": ["storage:/data/userdata.img"],
      "proposal_digest": "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069",
      "hint": "Re-run with --authorize sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069"
    }
  }
  ```

### 3.2 StreamLogEnvelope (`stderr`)

When `--json` is active, asynchronous events and diagnostic logs stream to `stderr` as append-only JSONL envelopes:

```json
{"timestamp":"2026-09-17T14:32:01.104Z","level":"INFO","event":"phase_transition","phase":"staging","message":"Staging custom bzImage to isolated instance runtime","operation_id":"op_01J8F9W2Z0K4M1N5P6Q7R8S9T0"}
{"timestamp":"2026-09-17T14:32:03.421Z","level":"INFO","event":"probe_sent","phase":"verifying","message":"Executing in-guest supercall probe to query [ksu_driver]","operation_id":"op_01J8F9W2Z0K4M1N5P6Q7R8S9T0"}
```

#### Fields:

- `timestamp`: RFC 3339 / ISO 8601 UTC timestamp.
- `level`: `"DEBUG"`, `"INFO"`, `"WARN"`, `"ERROR"`.
- `event`: Machine event tag (`"phase_transition"`, `"lock_acquired"`, `"probe_sent"`, `"reboot_detected"`).
- `phase`: Current operation phase (`"preflight"`, `"staging"`, `"booting"`, `"verifying"`, `"settled"`).
- `message`: Human-readable description.
- `operation_id`: Correlating operation identifier.

---

## 4. Two-Step Destructive Authorization Contract (FR-006, FR-035)

Any operation modifying persistent storage, replacing kernels, wiping guest disks, deleting devices, or reverting baselines requires explicit two-step authorization.

### Step 1: Request Proposal via `--dry-run`

The caller issues the command with `--dry-run`:

```bash
emu research device wipe --device emulator:avd-sec-01 --dry-run --json
```

**Stdout Response (Exit Code 0)**:

```json
{
  "status": "accepted",
  "outcome": "proposal_created",
  "operation_id": "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
  "data": {
    "proposal": {
      "proposal_digest": "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069",
      "action": "device.wipe",
      "target_device_id": "emulator:avd-sec-01",
      "affected_resources": ["storage:/data/userdata.img"],
      "risk_tier": "destructive_irreversible",
      "requires_reboot": true,
      "invalidates_overlays": true,
      "lossless": false,
      "expires_at": "2026-09-17T14:47:01.000Z"
    }
  },
  "error": null
}
```

### Step 2: Execute with `--authorize <DIGEST>`

The caller extracts `proposal_digest` and passes it via `--authorize`:

```bash
emu research device wipe --device emulator:avd-sec-01 --authorize sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069 --json
```

- If `--authorize` is omitted or contains an unknown/expired digest, execution **safely refuses** before modifying state. Exit code is `4` (`auth_refused`).

---

## 5. Complete Command Reference: All 8 Operational Families

### 5.1 Family 1: Backend & Capability Discovery (`backend`)

#### 5.1.1 `emu research backend preflight`

Evaluates local host operating system, virtualization hypervisor availability, and binary dependencies.

- **Flags**:
  - `--backend <emulator|cuttlefish>`: Optional target filter. If omitted, checks all backends.
  - `--json`: Emit JSON `OutputEnvelope`.
- **Output Data**: `BackendCapabilityProfile` per backend.
- **Exit Codes**: `0` (checks completed truthfully), `2` (invalid input), `3` (selected backend unsupported on this host).

#### 5.1.2 `emu research backend capabilities`

Returns the machine-readable inventory of supported features, guest architectures, and kernel injection support.

- **Flags**: `--backend <emulator|cuttlefish>`, `--json`.

#### 5.1.3 `emu research backend list`

Lists available virtualization backends on this host with status badges (`supported`, `unsupported`, `experimental`).

---

### 5.2 Family 2: Device Lifecycle Management (`device`)

#### 5.2.1 `emu research device list`

Lists all managed virtual research devices across all backends.

- **Flags**: `--backend <emulator|cuttlefish>`, `--state <stopped|running|error>`, `--json`.
- **Output Data**: `Vec<ResearchDevice>` with distinct `id`, `backend`, `display_name`, and `lifecycle_state`.

#### 5.2.2 `emu research device inspect`

Inspects complete runtime state, ports, ADB serial, and active profile of a specific device.

- **Flags**:
  - `--device <QUALIFIED_ID>`: Stable unique identifier (e.g. `cuttlefish:cf-x86-01`). Required.
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (found), `2` (not found or invalid ID).

#### 5.2.3 `emu research device create`

Creates and initializes a new research virtual device from user-supplied system images.

- **Flags**:
  - `--backend <emulator|cuttlefish>`: Required.
  - `--name <DISPLAY_NAME>`: User-assigned name. Required.
  - `--image-path <PATH>`: Path to system image directory or SDK image. Required.
  - `--arch <arm64|x86_64>`: CPU architecture.
  - `--api-level <N>`: Android API level.
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (created), `2` (invalid arguments / missing image), `3` (unsupported backend on host).

#### 5.2.4 `emu research device register`

Registers an existing locally prepared Cuttlefish instance directory into the research inventory.

- **Flags**:
  - `--backend cuttlefish`: Required.
  - `--name <DISPLAY_NAME>`: Required.
  - `--instance-dir <PATH>`: Path to isolated runtime root containing `cvd-host_package`. Required.
  - `--json`: Emit JSON.

#### 5.2.5 `emu research device start`

Launches the virtual device process supervisor.

- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--wait-ready`: Block until guest `sys.boot_completed=1`.
  - `--timeout <SECS>`: Execution deadline in seconds (default: 180s).
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (started / running), `2` (device not found), `124` (boot timed out; reports `execution_state: continuing`).

#### 5.2.6 `emu research device stop`

Gracefully halts the virtual device.

- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--force`: Escalate to SIGKILL if graceful shutdown exceeds timeout.
  - `--timeout <SECS>`: Timeout in seconds (default: 30s).
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (stopped), `2` (invalid ID), `124` (stop timed out).

#### 5.2.7 `emu research device restart`

Restarts the virtual device guest OS.

- **Flags**: `--device <QUALIFIED_ID>`, `--timeout <SECS>`, `--json`.

#### 5.2.8 `emu research device wipe` _(Destructive)_

Wipes user data and guest storage partitions.

- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--dry-run`: Generate `MutationProposal`.
  - `--authorize <DIGEST>`: Explicit authorization digest.
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (proposal generated or wipe completed), `4` (auth refused).

#### 5.2.9 `emu research device delete` _(Destructive)_

Purges virtual device registration and local instance files.

- **Flags**: `--device <QUALIFIED_ID>`, `--dry-run`, `--authorize <DIGEST>`, `--json`.
- **Exit Codes**: `0` (proposal generated or deleted), `4` (auth refused).

---

### 5.3 Family 3: Artifact Inspection & Research Profiles (`artifact`, `profile`)

#### 5.3.1 `emu research artifact validate`

Calculates SHA-256 fingerprint, checks ELF/APK headers, and validates architecture and KMI compatibility.

- **Flags**:
  - `--path <PATH>`: Local file path to artifact. Required.
  - `--type <kernel_image|manager_apk|daemon_binary|xposed_module|container_runtime_bundle>`: Required.
  - `--expected-digest <sha256:HEX>`: Optional user-supplied expected hash.
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (valid / verified), `1` (digest mismatch corrupt; G-03), `2` (invalid format).

#### 5.3.2 `emu research profile list`

Lists stored declarative research profiles.

#### 5.3.3 `emu research profile show`

Displays full JSON definition of a research profile by ID or file path.

#### 5.3.4 `emu research profile validate`

Evaluates profile compatibility against a target device without mutating guest state.

- **Flags**:
  - `--profile <PROFILE_ID_OR_PATH>`: Required.
  - `--target-device <QUALIFIED_ID>`: Required.
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (compatible), `2` (syntax error), `5` (capability conflict; FR-028).

#### 5.3.5 `emu research profile export`

Extracts the settled configuration of a device as an exportable `ResearchProfile` JSON document.

#### 5.3.6 `emu research profile import`

Validates and imports a research profile JSON file into the local registry.

#### 5.3.7 `emu research profile apply` _(Idempotent / Destructive if Kernel Swapped)_

Applies a declarative research profile to a target device.

- **Repeat Semantics (FR-037)**: If the target device already matches all declarative desired states, returns `status: "success"`, `outcome: "already_satisfied"`, and exit code `0` with **0 duplicate mutations, 0 redundant kernel flashes, and 0 unnecessary reboots**.
- **Destructive Gating**: If applying the profile requires swapping the kernel or wiping overlays, `--dry-run` and `--authorize` are required.
- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--profile <PROFILE_ID_OR_PATH>`: Required.
  - `--dry-run`: Generate `MutationProposal` if destructive changes are needed.
  - `--authorize <DIGEST>`: Pass authorization digest for destructive changes.
  - `--wait`: Block until all post-apply verification checks settle.
  - `--timeout <SECS>`: Timeout duration (default: 300s).
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (completed or already satisfied), `4` (auth refused), `5` (conflict), `124` (timed out).

---

### 5.4 Family 4: Toolchain & Capability Lifecycle (`tool`, `capability`)

#### 5.4.1 `emu research tool list`

Lists all research tools, privilege mechanisms, hooking frameworks, and extended capabilities on a device, with desired and observed states.

- **Flags**: `--device <QUALIFIED_ID>`, `--json`.

#### 5.4.2 `emu research tool inspect`

Queries detailed driver information, socket nodes, and active verification details for a tool.

- **Flags**: `--device <QUALIFIED_ID>`, `--tool <TOOL_ID>`, `--json`.

#### 5.4.3 `emu research tool enable`

Enables a tool or capability on a device.

- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--tool <TOOL_ID>`: Required (e.g. `root:kernelsu_next`, `frida`, `framework:vector`, `cap:susfs`).
  - `--artifact <PATH>`: Path to user-supplied artifact if staging is needed.
  - `--dry-run`: Required if kernel swap is needed.
  - `--authorize <DIGEST>`: Required if kernel swap is needed.
  - `--json`: Emit JSON.

#### 5.4.4 `emu research tool disable`

Deactivates a tool while preserving installed files (e.g. boots baseline kernel for root, or disables Vector daemon).

- **Flags**: `--device <QUALIFIED_ID>`, `--tool <TOOL_ID>`, `--dry-run`, `--authorize <DIGEST>`, `--json`.

#### 5.4.5 `emu research tool remove` _(Destructive)_

Completely uninstalls tool components from the guest.

- **Flags**: `--device <QUALIFIED_ID>`, `--tool <TOOL_ID>`, `--dry-run`, `--authorize <DIGEST>`, `--json`.

#### 5.4.6 `emu research tool scope`

Configures application mediation scope for Xposed-compatible modules (FR-017).

- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--module <MODULE_ID>`: Package name of the Xposed module. Required.
  - `--target-app <PACKAGE>`: Application granted scoped modification. Required.
  - `--control-app <PACKAGE>`: Independent application verified to retain baseline behavior. Optional.
  - `--json`: Emit JSON.

#### 5.4.7 `emu research capability catalog`

Queries the 9-family extended capability catalog.

- **Flags**: `--family <1..9>`, `--json`.

---

### 5.5 Family 5: Scoped Experiments & Diagnostic Evidence (`experiment`)

#### 5.5.1 `emu research experiment run-test`

Executes an authorized, benign guest-scoped verification probe.

- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--test-type <root_identity|frida_probe|vector_scope|capability_verify>`: Required.
  - `--target-app <PACKAGE>`: Required for `frida_probe` and `vector_scope`.
  - `--timeout <SECS>`: Execution deadline (default: 30s).
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (test passed; telemetry received), `1` (test assertion failed), `124` (timed out).

#### 5.5.2 `emu research experiment trace`

Attaches dynamic tracing to a target application and streams telemetry.

- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--target-app <PACKAGE>`: Required.
  - `--script <PATH>`: User-supplied Frida tracing script. Required.
  - `--follow`: Stream telemetry events until interrupted.
  - `--duration <SECS>`: Bounded tracing duration.
  - `--json`: Emit JSON.
- **Observation Separation Invariant (SC-015)**: Stopping an active trace (via Ctrl+C or duration expiry) halts only host observation. It **never cancels or halts the underlying guest target application** and is never reported as an operation failure. Exit code is `0`.

#### 5.5.3 `emu research experiment logs`

Streams filtered guest logs (`logcat` / `kmsg`).

- **Flags**: `--device <QUALIFIED_ID>`, `--follow`, `--lines <N>`, `--filter <REGEX>`, `--json`.

---

### 5.6 Family 6: Provenance & Profile Reproduction (`provenance`)

#### 5.6.1 `emu research provenance capture`

Captures an immutable audit snapshot of the active device environment.

- **Flags**: `--device <QUALIFIED_ID>`, `--label <TEXT>`, `--json`.
- **Output Data**: `ExperimentProvenanceRecord` with canonical `environment_digest`.

#### 5.6.2 `emu research provenance export`

Exports a captured provenance record to a standalone JSON file.

- **Flags**: `--id <PROVENANCE_ID>`, `--out <PATH>`, `--json`.

#### 5.6.3 `emu research provenance inspect`

Inspects an exported provenance JSON file, displaying all artifact hashes and launch parameters.

#### 5.6.4 `emu research provenance compare`

Compares two provenance records or compares a provenance file against a live device, outputting exact structural diffs.

- **Flags**: `--file-a <PATH>`, `--file-b <PATH_OR_DEVICE>`, `--json`.

---

### 5.7 Family 7: Baseline Recovery (`baseline`)

#### 5.7.1 `emu research baseline create`

Records a verified working baseline snapshot for a device.

- **Flags**: `--device <QUALIFIED_ID>`, `--label <TEXT>`, `--json`.

#### 5.7.2 `emu research baseline inspect`

Inspects the baseline configuration and confirms image files exist on disk.

#### 5.7.3 `emu research baseline verify`

Boots the baseline snapshot to confirm functional readiness.

#### 5.7.4 `emu research baseline restore` _(Destructive)_

Restores the device disk and kernel state to the designated baseline after failure or bootloop (FR-021, SC-005).

- **Safe Refusal Invariant (FR-021)**: If the baseline is missing or unverified, the system **safely refuses** recovery without deleting the device. Exit code is `1` (`failed`) or `2` (`invalid_input`).
- **Flags**:
  - `--device <QUALIFIED_ID>`: Required.
  - `--dry-run`: Generate `MutationProposal`.
  - `--authorize <DIGEST>`: Pass authorization digest.
  - `--timeout <SECS>`: Restoration deadline (default: 180s per SC-005).
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (restored successfully within deadline), `4` (auth refused), `1` (baseline missing / unverified).

---

### 5.8 Family 8: Operation Status & Cancellation (`operation`)

#### 5.8.1 `emu research operation list`

Lists all active and recent background operations.

- **Flags**: `--device <QUALIFIED_ID>`, `--state <in_progress|completed|failed|cancelled|timed_out>`, `--json`.

#### 5.8.2 `emu research operation inspect`

Queries detailed status, checkpoints, and partial modifications of an operation.

- **Flags**: `--operation-id <ID>`, `--json`.

#### 5.8.3 `emu research operation wait`

Blocks until an operation finishes or until a caller-defined timeout expires.

- **Flags**:
  - `--operation-id <ID>`: Required.
  - `--timeout <SECS>`: Caller wait deadline. Required.
  - `--json`: Emit JSON.
- **Exit Codes**: `0` (operation settled to `completed`), `1` (settled to `failed`), `124` (wait deadline expired before settlement; reports actual operational state `continuing` or `unknown`), `130` (operation cancelled).

#### 5.8.4 `emu research operation cancel`

Sends an explicit cancellation request to an in-progress operation.

- **Safe Transaction Boundary Invariant (FR-036)**: The worker acknowledges cancellation and halts at the nearest safe transaction checkpoint.
- **Flags**:
  - `--operation-id <ID>`: Required.
  - `--wait-safe-boundary`: Block until safe boundary reached.
  - `--timeout <SECS>`: Deadline for waiting on safe boundary (default: 30s).
  - `--json`: Emit JSON.
- **Exit Codes**:
  - `130` (`cancelled`): Safe transaction boundary reached; underlying task stopped.
  - `124` (`cancellation_pending`): Wait deadline expired before safe boundary reached; actual state reported as `continuing` or `unknown` without falsely claiming completion.

---

## 6. Deterministic Command-to-Exit-Code Mapping Matrix

| Command Invocations                                       | Precondition / Input Condition               | Output `status`       | Output `outcome`                | Exit Code |
| :-------------------------------------------------------- | :------------------------------------------- | :-------------------- | :------------------------------ | :-------: |
| `backend preflight --backend emulator`                    | Host supports KVM / Hypervisor.Framework     | `"success"`           | `"completed"`                   |   **0**   |
| `backend preflight --backend cuttlefish`                  | Host is macOS or Windows (unsupported)       | `"success"`           | `"completed"` (truthful report) |   **0**   |
| `backend preflight --backend cuttlefish`                  | Host lacks `/dev/kvm` and `cvd` tools        | `"rejected"`          | `"unsupported"`                 |   **3**   |
| `device inspect --device cuttlefish:cf-01`                | Device exists                                | `"success"`           | `"completed"`                   |   **0**   |
| `device inspect --device cuttlefish:cf-01`                | Device ID not found                          | `"rejected"`          | `"invalid_input"`               |   **2**   |
| `device stop --device "research-pixel"`                   | Name matches both Emulator and Cuttlefish    | `"rejected"`          | `"invalid_input"`               |   **2**   |
| `device wipe --device emulator:avd-01`                    | Destructive call without `--authorize`       | `"rejected"`          | `"auth_refused"`                |   **4**   |
| `device wipe --device emulator:avd-01 --dry-run`          | Dry-run call generating proposal             | `"accepted"`          | `"proposal_created"`            |   **0**   |
| `device wipe --device emulator:avd-01 --authorize <hash>` | Valid, unexpired authorization hash          | `"success"`           | `"completed"`                   |   **0**   |
| `profile apply --device ... --profile ...`                | Device already in desired profile state      | `"already_satisfied"` | `"already_satisfied"`           |   **0**   |
| `profile apply --device ... --profile <conflicting>`      | Profile contains KowSU + SUSFS               | `"rejected"`          | `"conflict"`                    |   **5**   |
| `profile apply --device ... --profile <unknown_root>`     | Attempting multiple roots or unknown flavor  | `"rejected"`          | `"conflict"`                    |   **5**   |
| `tool enable --device ... --tool cap_baseband_guard`      | Virtual device lacking physical baseband     | `"rejected"`          | `"unsupported"` (reports N/A)   |   **3**   |
| `experiment run-test --device ... --timeout 5`            | Guest kernel panic or freeze during test     | `"timed_out"`         | `"timeout"`                     |  **124**  |
| `operation cancel --operation-id ...`                     | Safe boundary reached within timeout         | `"cancelled"`         | `"cancelled"`                   |  **130**  |
| `operation cancel --operation-id ...`                     | Wait deadline elapses before safe checkpoint | `"timed_out"`         | `"cancellation_pending"`        |  **124**  |
| `experiment trace --device ... --duration 10`             | Bounded trace finishes                       | `"success"`           | `"completed"`                   |   **0**   |
