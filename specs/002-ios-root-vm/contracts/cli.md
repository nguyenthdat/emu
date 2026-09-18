# CLI Contract: iOS Root VM & Darwin Security Research Automation Interface

**Version**: 1.0.0  
**Feature Branch**: `002-ios-root-vm`  
**Status**: Complete  
**Authority**: `specs/002-ios-root-vm/research.md`, `specs/002-ios-root-vm/spec.md`, and `specs/002-ios-root-vm/data-model.md`

---

## 1. Executive Summary & Invocation Architecture

### 1.1 Single-Binary Command Routing (Decision D-04)

The `emu` project maintains a single binary architecture. Interactive TUI workflows launch when `emu` is invoked without subcommands on an interactive terminal. Headless, non-interactive research automation is completely routed through the `emu research` subcommand hierarchy:

```text
emu [GLOBAL_FLAGS] research <FAMILY> <SUBCOMMAND> [COMMAND_FLAGS]
```

- **Headless Execution**: Every command under `emu research ...` bypasses interactive TUI initialization (`App::new()`), preventing raw terminal allocation, mouse tracking, or UI thread polling.
- **Private Supervisor Execution (Decision D-04)**: Running guest virtual machines are supervised by a private child process invocation:
  ```text
  emu __supervise --vm-id <INSTANCE_ID>
  ```
  The supervisor process is not a public CLI subcommand or persistent system daemon (`launchd`). It is an unprivileged child process dedicated to managing one guest's QEMU process, QMP socket, GDB chardev, console PTY, and advisory file lock.
- **Private Worker Execution (Decision D-04)**: Long-running asynchronous operations (image preparation, offline patching, baseline restoration) are executed via:
  ```text
  emu __worker --operation-id <OPERATION_ID>
  ```

### 1.2 Stream Separation (FR-044)

The automation interface strictly separates machine-readable data payloads from operational progress and diagnostic telemetry:

- **`stdout`**: Reserved exclusively for the finite JSON output contract (`OutputEnvelope`). When `--json` is supplied, `stdout` outputs exactly one valid, complete JSON object upon command completion.
- **`stderr`**: Used for diagnostic progress, operational status updates, and warning messages. When `--json` is active, `stderr` outputs append-only JSONL log records (`StreamLogEnvelope`), ensuring automated log parsers never parse human-formatted text.

---

## 2. Process Exit Code Contract

Exit codes are standardized, deterministic, and categorized across all operational families (FR-045, SC-014, SC-019):

| Exit Code | Outcome Identifier                                           | Classification                       | Description & Trigger Scenarios                                                                                                                                                                                                                                                                                                                                                             |
| :-------: | :----------------------------------------------------------- | :----------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
|  **`0`**  | `"completed"` / `"already_satisfied"` / `"proposal_created"` | **Success**                          | • Operational mutation completed successfully.<br>• Read-only inspection evaluated truthfully (including queries reporting that a capability is unavailable or unsupported).<br>• Applying a desired profile that matches current active guest state exactly (idempotent no-op; FR-047).<br>• Command invoked with `--dry-run` generated a valid `MutationProposal` with `proposal_digest`. |
|  **`1`**  | `"execution_failed"`                                         | **Runtime Failure**                  | • Guest hypervisor crash, QMP protocol error, or unhandled supervisor fault.<br>• Root proof probe failure (positive write failed or kernel evidence missing).<br>• Injected Frida script syntax error or target process crash.<br>• Kernel debug RSP packet timeout or protocol mismatch.                                                                                                  |
|  **`2`**  | `"invalid_input"`                                            | **Invalid Input**                    | • Missing required command-line arguments or invalid flag parameters.<br>• Target `display_name` is ambiguous across backends without unique identifier or `--backend` qualification (FR-007).<br>• Referenced image artifact, profile file, or application package not found or corrupted (FR-031, FR-032).<br>• Incompatible application binary architecture (non-ARM64 Mach-O).          |
|  **`3`**  | `"unsupported"`                                              | **Capability Unsupported**           | • Invoking iOS research backends on non-macOS or non-Apple Silicon hosts (FR-001).<br>• Requesting application installation or Frida app spawning on minimal `darwin-vm` backend (`app_frameworks_supported = false`; FR-023, SC-003).                                                                                                                                                      |
|  **`4`**  | `"auth_refused"`                                             | **Authorization Refused**            | • Destructive mutation (disk wipe, instance deletion, baseline rollback) invoked without `--authorize sha256:<digest>` (FR-008).<br>• Host-side image preparation requiring elevated privileges executed in unattended automation mode without credentials (FR-035).<br>• Unauthorized attempt to export application container data or private system paths (FR-024).                       |
|  **`5`**  | `"conflict"`                                                 | **Concurrency / State Conflict**     | • Conflicting concurrent mutations against the same guest instance (instance run lock `<id>.run.lock` held; FR-047, D-04).<br>• Conflicting operation lock (`<op_id>.op.lock`) or device lock held.<br>• Kernel debug lease held by another active debugger client (D-05).                                                                                                                  |
| **`124`** | `"timeout"` / `"cancellation_pending"`                       | **Timeout / Pending Safe Cessation** | • Caller wait deadline elapsed before task reached completion; actual guest state is reported as continuing, stopped, or unknown without stopping background work (FR-046, SC-016).<br>• Cancellation requested, but safe transaction boundary was not yet reached before caller wait deadline expired.                                                                                     |
| **`130`** | `"cancelled"`                                                | **Cancelled at Safe Boundary**       | • Explicit user cancellation signal (SIGINT / Ctrl+C or `operation cancel`) successfully halted the operation at a verified safe transaction boundary with disposable cleanup (FR-046, SC-015).                                                                                                                                                                                             |

---

## 3. Standard JSON Envelopes

### 3.1 OutputEnvelope (`stdout`)

Every command executed with `--json` outputs exactly one `OutputEnvelope` on `stdout`:

```json
{
  "$schema": "https://emu.rs/schemas/v1/research.schema.json#/definitions/OutputEnvelope",
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
  "data": {
    "instance_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "backend": "darwin-vm",
    "lifecycle_state": "running"
  },
  "error": null
}
```

#### Field Definitions:

- `status` (`string`): High-level classification:
  - `"success"`: Operation completed successfully or read query evaluated.
  - `"already_satisfied"`: Desired profile already active; zero redundant actions taken.
  - `"accepted"`: Dry-run accepted and mutation proposal generated.
  - `"failed"`: Runtime or probe verification failure.
  - `"rejected"`: Refused preflight due to input, validation, or support constraints.
  - `"timed_out"`: Caller wait deadline elapsed.
  - `"cancelled"`: Halted at safe transaction boundary.
- `outcome` (`string`): Exact outcome identifier matching the Exit Code Contract:
  `"completed"` | `"already_satisfied"` | `"proposal_created"` | `"auth_refused"` | `"invalid_input"` | `"unsupported"` | `"conflict"` | `"timeout"` | `"cancelled"` | `"execution_failed"`
- `operation_id` (`string`): Tracking ID prefixed with `op_`.
- `data` (`object | null`): Command-specific payload. Null on terminal error.
- `error` (`ErrorRecord | null`): Structured error information, or null on success:
  ```json
  {
    "code": "AUTH_REQUIRED",
    "message": "Destructive operation requires explicit two-step authorization",
    "details": {
      "target_instance_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
      "affected_paths": [
        "/Users/researcher/Library/Application Support/emu/research/instances/e4b1c2d3.json"
      ],
      "proposal_digest": "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069",
      "hint": "Re-run with --authorize sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069"
    }
  }
  ```

### 3.2 StreamLogEnvelope (`stderr`)

When `--json` is active, `stderr` emits newline-delimited JSON objects:

```json
{
  "timestamp": "2026-09-17T14:32:01.104Z",
  "level": "INFO",
  "event": "QMP_STATUS_CHANGED",
  "phase": "booting",
  "message": "Virtual CPU running; awaiting guest bootstrap console ready",
  "operation_id": "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0"
}
```

---

## 4. Two-Step Authorization Contract for Destructive Actions

Destructive operations (instance deletion, disk wiping, baseline rollback, in-guest security policy modification) require explicit confirmation identifying the affected operation, target instance ID, backend, and persistent data paths (FR-008):

### Step 1: Proposal Generation (`--dry-run`)

```sh
emu research guest delete --id e4b1c2d3-4567-89ab-cdef-0123456789ab --dry-run --json
```

**Stdout (Exit Code 0, outcome: `"proposal_created"`)**:

```json
{
  "status": "accepted",
  "outcome": "proposal_created",
  "operation_id": "op_01J8DELPROP01",
  "data": {
    "proposal_id": "prop_01J8DEL001",
    "proposal_digest": "sha256:a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0",
    "operation_type": "instance_delete",
    "target_instance_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "backend": "darwin-vm",
    "affected_paths": [
      "/Users/researcher/Library/Application Support/emu/research/instances/e4b1c2d3-4567-89ab-cdef-0123456789ab.json",
      "/Users/researcher/Library/Application Support/emu/research/disks/darwin_vm_root_e4b1c2d3.raw"
    ],
    "destructive": true,
    "expires_at": "2026-09-17T14:47:00.000Z"
  },
  "error": null
}
```

### Step 2: Authorized Execution

```sh
emu research guest delete --id e4b1c2d3-4567-89ab-cdef-0123456789ab \
  --authorize sha256:a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0 --json
```

**Stdout (Exit Code 0, outcome: `"completed"`)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8DELCONF01",
  "data": {
    "instance_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "deleted": true,
    "reclaimed_paths": [
      "/Users/researcher/Library/Application Support/emu/research/instances/e4b1c2d3-4567-89ab-cdef-0123456789ab.json",
      "/Users/researcher/Library/Application Support/emu/research/disks/darwin_vm_root_e4b1c2d3.raw"
    ]
  },
  "error": null
}
```

_Triggering without `--authorize` or `--dry-run` returns Exit Code 4 (`auth_refused`), outputting the proposal and instructions._

---

## 5. Command Family Specifications

### Family 1: Backend Discovery & Preflight (`backend`)

#### `emu research backend preflight`

Non-interactively evaluates host CPU architecture, `Hypervisor.framework` availability, permissions, and toolchain dependencies (FR-001, FR-002).

```sh
emu research backend preflight [--json]
```

**Expected JSON Response (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8PREFLIGHT01",
  "data": {
    "host_platform": {
      "os": "macos",
      "arch": "aarch64",
      "os_version": "Darwin 25.6.0",
      "hypervisor_support": true,
      "hardware_acceleration": "apple_silicon_hvf"
    },
    "profiles": [
      {
        "backend": "darwin-vm",
        "support_status": "supported",
        "supported_guest_families": ["darwin-minimal"],
        "app_frameworks_supported": false,
        "required_binaries": [
          {
            "binary_name": "qemu-system-aarch64",
            "found": true,
            "resolved_path": "/opt/homebrew/bin/qemu-system-aarch64"
          }
        ],
        "remediation_steps": []
      },
      {
        "backend": "Inferno",
        "support_status": "supported",
        "supported_guest_families": ["ios-14", "ios-15"],
        "app_frameworks_supported": true,
        "required_binaries": [
          {
            "binary_name": "qemu-system-aarch64",
            "found": true,
            "resolved_path": "/opt/homebrew/bin/qemu-system-aarch64"
          },
          {
            "binary_name": "ideviceinstaller",
            "found": true,
            "resolved_path": "/opt/homebrew/bin/ideviceinstaller"
          }
        ],
        "remediation_steps": []
      }
    ]
  },
  "error": null
}
```

#### `emu research backend list`

Lists supported research backends and operational capability summaries.

```sh
emu research backend list [--json]
```

---

### Family 2: Guest Lifecycle Management (`guest`)

#### `emu research guest create`

Registers and initializes a new research guest instance with a globally unique UUIDv4 (FR-005, FR-006).

```sh
emu research guest create \
  --name <DISPLAY_NAME> \
  --backend <darwin-vm|Inferno> \
  --image <ARTIFACT_DIGEST> \
  [--kernel-args <ARGS>] \
  [--json]
```

**Expected JSON Response (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8CREATE01",
  "data": {
    "id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "display_name": "ios-sec-lab",
    "backend": "darwin-vm",
    "lifecycle_state": "stopped",
    "guest_arch": "arm64",
    "base_image_ref": "sha256:d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456",
    "observed_privilege": "unverified",
    "created_at": "2026-09-17T14:30:00.000Z"
  },
  "error": null
}
```

#### `emu research guest list`

Enumerates registered research guest instances with backend badges and lifecycle states.

```sh
emu research guest list [--backend <darwin-vm|Inferno>] [--json]
```

#### `emu research guest inspect`

Returns complete descriptor for a single guest instance. Disambiguates identical display names (FR-007).

```sh
emu research guest inspect (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--json]
```

#### `emu research guest start`

Boots a registered guest instance, launching the dedicated supervisor process (FR-006, D-04).

```sh
emu research guest start (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--timeout <SECS>] [--json]
```

**Expected JSON Response (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8START01",
  "data": {
    "id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "display_name": "ios-sec-lab",
    "backend": "darwin-vm",
    "lifecycle_state": "running",
    "boot_session_id": "9a8b7c6d-5e4f-3a2b-1c0d-ef9876543210",
    "qmp_socket_path": "/tmp/emu-a1b2c3d4/qmp.sock",
    "console_socket_path": "/tmp/emu-a1b2c3d4/console.sock",
    "observed_privilege": "unverified"
  },
  "error": null
}
```

#### `emu research guest stop`

Stops a running guest instance cleanly via QMP shutdown (FR-006).

```sh
emu research guest stop (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--force] [--json]
```

#### `emu research guest restart`

Performs clean guest reboot. Invalidates active root and Frida evidence (FR-015, FR-019).

```sh
emu research guest restart (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--json]
```

#### `emu research guest delete`

Destructive command requiring two-step authorization (FR-008).

```sh
emu research guest delete (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--dry-run] [--authorize <DIGEST>] [--json]
```

---

### Family 3: Root Proof & Privilege Verification (`root`)

#### `emu research root verify`

Executes empirical root proof workflow: runs positive probe fixture (`/private/var/root/.emu_probe`) as UID 0, executes negative control check as UID 501 (`mobile`), and captures kernel evidence (FR-010, FR-012, FR-013).

```sh
emu research root verify (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--test-binary <PATH>] [--json]
```

**Expected JSON Response (Exit Code 0, Positive + Negative pass)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8ROOTVERIFY01",
  "data": {
    "guest_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "boot_session_id": "9a8b7c6d-5e4f-3a2b-1c0d-ef9876543210",
    "verified_uid": 0,
    "positive_probe": {
      "path": "/private/var/root/.emu_probe",
      "status": "success",
      "output": "emu_root_verified"
    },
    "negative_control": {
      "target_user": "mobile",
      "uid": 501,
      "status": "denied",
      "expected_denial": true,
      "output": "Permission denied"
    },
    "observed_kernel_version": "Darwin Kernel Version 20.0.0: root:xnu-7195.0.0~1/RELEASE_ARM64_T8030",
    "observed_boot_args": "debug=0x144 amfi=0xff -v",
    "benign_binary_digest": "sha256:5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b",
    "verification_state": "verified",
    "verified_at": "2026-09-17T14:35:10.000Z"
  },
  "error": null
}
```

_Negative Control Failure Scenario_: If the unprivileged negative control action unexpectedly succeeds, the command returns Exit Code 1 (`failed`), sets `verification_state: "unverified"`, and emits diagnostic warnings (FR-014).

#### `emu research root status`

Returns desired vs observed privilege state for a guest instance (FR-014).

```sh
emu research root status (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--json]
```

#### `emu research root console`

Connects an interactive terminal or streams commands to the guest root bootstrap console (FR-011).

```sh
emu research root console (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--command <CMD>] [--json]
```

---

### Family 4: Application Lifecycle Management (`app`)

_Supported on `Inferno`. Rejected with Exit Code 3 (`unsupported`) on `darwin-vm` (FR-023, SC-003)._

#### `emu research app import`

Validates Mach-O headers (ARM64 architecture) and registers an owned application artifact (FR-022, FR-031).

```sh
emu research app import --package <PATH_TO_IPA_OR_APP> [--json]
```

#### `emu research app install`

Installs application onto running `Inferno` guest via `InstallationProxy` / `ideviceinstaller` (FR-016, D-03).

```sh
emu research app install (--id <GUEST_ID> | --name <GUEST_NAME>) --app-id <APP_ID> [--json]
```

#### `emu research app list`

Lists installed applications and container paths on target guest.

```sh
emu research app list (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research app launch`

Spawns target application via Frida `Device.spawn()` or SpringBoard launch and verifies container existence (D-03).

```sh
emu research app launch (--id <GUEST_ID> | --name <GUEST_NAME>) --bundle-id <BUNDLE_ID> [--json]
```

#### `emu research app inspect`

Inspects running application process PID, sandbox container metadata, and entitlements (FR-024).

```sh
emu research app inspect (--id <GUEST_ID> | --name <GUEST_NAME>) --bundle-id <BUNDLE_ID> [--json]
```

#### `emu research app stop`

Terminates running application process.

```sh
emu research app stop (--id <GUEST_ID> | --name <GUEST_NAME>) --bundle-id <BUNDLE_ID> [--json]
```

#### `emu research app remove`

Uninstalls application and cleans up sandboxed container.

```sh
emu research app remove (--id <GUEST_ID> | --name <GUEST_NAME>) --bundle-id <BUNDLE_ID> [--json]
```

#### `emu research app container-read`

Reads an authorized file path inside target application data container (FR-024).

```sh
emu research app container-read (--id <GUEST_ID> | --name <GUEST_NAME>) --bundle-id <BUNDLE_ID> --path <RELATIVE_PATH> [--json]
```

#### `emu research app container-export`

Exports container directory to host destination under explicit researcher authorization (FR-024).

```sh
emu research app container-export (--id <GUEST_ID> | --name <GUEST_NAME>) --bundle-id <BUNDLE_ID> --destination <HOST_DIR> [--authorize-export] [--json]
```

---

### Family 5: Frida Dynamic Instrumentation (`frida`)

_Managed via private child executable `emu-frida-worker` linking official `frida-core` 17.18.0 C devkit (D-02)._

#### `emu research frida prepare`

Stages and validates in-guest Frida agent package matching pinned version `"17.18.0"` (FR-017).

```sh
emu research frida prepare (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research frida install`

Installs agent binary into guest runtime.

```sh
emu research frida install (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research frida start`

Starts agent daemon inside guest, binding strictly to loopback bridge (D-02, FR-039).

```sh
emu research frida start (--id <GUEST_ID> | --name <GUEST_NAME>) [--port 27042] [--json]
```

#### `emu research frida attach`

Attaches dynamic instrumentation probes to target application or daemon with user script (FR-018, FR-020). Verifies probe specificity against control process (FR-021, SC-002).

```sh
emu research frida attach (--id <GUEST_ID> | --name <GUEST_NAME>) \
  (--pid <PID> | --bundle-id <BUNDLE_ID> | --daemon <NAME>) \
  --script <SCRIPT_PATH> \
  [--control-pid <CONTROL_PID>] \
  [--json]
```

**Expected JSON Response (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8FRIDAATTACH01",
  "data": {
    "session_id": "sess_01J8ATTACH001",
    "guest_id": "f5c2d3e4-5678-9abc-def0-123456789abc",
    "target_pid": 412,
    "target_bundle_id": "com.example.researchapp",
    "attachment_state": "attached",
    "hooks_active": {
      "native_c_hooks": 1,
      "objc_method_hooks": 1
    },
    "control_process_id": 510,
    "control_process_hooked": false,
    "observed_events": [
      {
        "type": "native",
        "symbol": "open",
        "arg0": "/etc/hosts",
        "timestamp": "2026-09-17T14:36:01.200Z"
      },
      {
        "type": "objc",
        "class": "NSURLSession",
        "method": "- dataTaskWithRequest:",
        "timestamp": "2026-09-17T14:36:01.350Z"
      }
    ]
  },
  "error": null
}
```

#### `emu research frida inspect`

Inspects active hook telemetry, intercepted event counts, and target process health (FR-017).

```sh
emu research frida inspect --session-id <SESSION_ID> [--json]
```

#### `emu research frida detach`

Detaches instrumentation probes cleanly without terminating target process (FR-018, SC-002).

```sh
emu research frida detach --session-id <SESSION_ID> [--json]
```

#### `emu research frida stop`

Stops in-guest Frida agent process.

```sh
emu research frida stop (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research frida remove`

Cleans up deployed agent artifacts from guest environment (SC-002).

```sh
emu research frida remove (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

---

### Family 6: Deep System & Kernel Debugging (`debug`)

_Operates under an exclusive `KernelDebugLease` over private GDB RSP chardev socket (D-05, FR-027)._

#### `emu research debug pause`

Halts virtual CPU execution cleanly via QMP `stop` or GDB halt packet. Transitions runstate to `paused` (FR-027, FR-028).

```sh
emu research debug pause (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research debug resume`

Resumes guest CPU execution (GDB `c` packet or QMP `cont`). Permitted only if debug lease allows (D-05).

```sh
emu research debug resume (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research debug registers`

Reads or edits 64-bit general-purpose registers (`x0`-`x30`, `sp`, `pc`, `pstate`) (D-05, SC-004).

```sh
emu research debug registers (--id <GUEST_ID> | --name <GUEST_NAME>) [--write <REG>=<HEX_VALUE>] [--json]
```

**Expected JSON Response (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8DEBUGREGS01",
  "data": {
    "guest_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "pc": "0xfffffe0007812000",
    "sp": "0xfffffe001b345000",
    "pstate": "0x60000005",
    "general_registers": {
      "x0": "0x0000000000000000",
      "x1": "0xfffffe001b345010",
      "x16": "0xfffffe000789abcd",
      "x30": "0xfffffe0007811fe0"
    }
  },
  "error": null
}
```

#### `emu research debug memory`

Reads or writes guest virtual/physical memory addresses with bounded size (D-05, SC-004).

```sh
emu research debug memory (--id <GUEST_ID> | --name <GUEST_NAME>) \
  --address <HEX_ADDR> --length <BYTES> [--write-hex <HEX_DATA>] [--json]
```

#### `emu research debug breakpoint`

Sets (`Z0`) or clears (`z0`) software breakpoints at target kernel virtual addresses (FR-027).

```sh
emu research debug breakpoint (--id <GUEST_ID> | --name <GUEST_NAME>) \
  (--set <HEX_ADDR> | --clear <HEX_ADDR>) [--json]
```

#### `emu research debug step`

Executes exactly one single instruction step (`s` packet) while guest is paused (FR-027, SC-004).

```sh
emu research debug step (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research debug status`

Queries active debug lease, current CPU halt status, and breakpoint list.

```sh
emu research debug status (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research debug disconnect`

Disconnects debugger client. Queries actual runstate: if paused, preserves paused status truthfully without silent resumption (FR-029, SC-006).

```sh
emu research debug disconnect (--id <GUEST_ID> | --name <GUEST_NAME>) [--action <preserve-paused|resume>] [--json]
```

**Expected JSON Response (Exit Code 0, preserve-paused)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8DISCONN01",
  "data": {
    "guest_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "lease_released": true,
    "observed_guest_runstate": "paused",
    "silently_resumed": false,
    "recovery_options": [
      "emu research debug resume --id e4b1c2d3-4567-89ab-cdef-0123456789ab",
      "emu research guest restart --id e4b1c2d3-4567-89ab-cdef-0123456789ab",
      "emu research guest stop --id e4b1c2d3-4567-89ab-cdef-0123456789ab --force"
    ]
  },
  "error": null
}
```

---

### Family 7: Image Preparation & Artifact Management (`image`)

#### `emu research image register`

Computes SHA-256 digest, extracts build identity from plist, and registers artifact (FR-031).

```sh
emu research image register --file <PATH> --type <TYPE> --backend <darwin-vm|Inferno> [--allow-experimental] [--json]
```

#### `emu research image prepare`

Prepares minimal root disk or patches ramdisk using verified unique device nodes (FR-033, FR-034, D-07). Requires administrative authorization if elevated permissions are necessary (FR-035).

```sh
emu research image prepare \
  --source <PATH> \
  --target-backend <darwin-vm|Inferno> \
  --output <DEST_PATH> \
  [--unattended] \
  [--json]
```

_Unattended Failure_: If elevated host privileges are required and executed with `--unattended` without pre-authorized credentials, exits immediately with Exit Code 2 (`invalid_input` / `MissingAuthorization`) (FR-035).

#### `emu research image verify-mount`

Verifies mounted guest disk identity and volume UUID via `diskutil info -plist` (FR-033).

```sh
emu research image verify-mount --mount-path <PATH> [--json]
```

#### `emu research image cleanup`

Triggers deterministic cleanup stack of temporary virtual disk attachments and mount points (FR-036).

```sh
emu research image cleanup [--operation-id <OP_ID>] [--json]
```

#### `emu research image inspect`

Returns artifact metadata, content hash, and verified provenance.

```sh
emu research image inspect --digest <SHA256_DIGEST> [--json]
```

#### `emu research image list`

Lists registered research image artifacts.

```sh
emu research image list [--backend <darwin-vm|Inferno>] [--json]
```

---

### Family 8: Companion VM Orchestration (`companion`)

_Manages local helper Linux VM on macOS Apple Silicon for `Inferno` restore and USB-over-IP workflows (FR-037, D-08)._

#### `emu research companion start`

Launches companion VM with declared CPU and memory resource limits (FR-037).

```sh
emu research companion start --parent-guest-id <GUEST_ID> [--cpus 2] [--memory-mb 2048] [--json]
```

#### `emu research companion inspect`

Reports companion lifecycle status, resource consumption, active restore tasks, and live dependent count (FR-038).

```sh
emu research companion inspect --parent-guest-id <GUEST_ID> [--json]
```

#### `emu research companion stop`

Terminates companion VM. Fails if live dependent guest sessions remain bound (FR-038, SC-011).

```sh
emu research companion stop --parent-guest-id <GUEST_ID> [--force] [--json]
```

**Rejection Scenario (Exit Code 5 `conflict`, live dependents exist)**:

```json
{
  "status": "rejected",
  "outcome": "conflict",
  "operation_id": "op_01J8COMPSTOP01",
  "data": null,
  "error": {
    "code": "DEPENDENT_GUEST_ACTIVE",
    "message": "Cannot terminate companion VM: 1 live guest session depends on companion forwarders",
    "details": {
      "companion_id": "comp_01J8COMPANION01",
      "live_dependents_count": 1,
      "hint": "Terminate dependent guest sessions before shutting down companion, or supply --force"
    }
  }
}
```

#### `emu research companion status`

Returns overall companion environment health across all active instances.

```sh
emu research companion status [--json]
```

---

### Family 9: Profile & Baseline Management (`profile` / `baseline`)

#### `emu research profile apply`

Applies declared `ResearchExperimentProfile` to a guest instance. Idempotent: returns Exit Code 0 with `"already_satisfied"` if active configuration already matches (FR-047).

```sh
emu research profile apply (--id <GUEST_ID> | --name <GUEST_NAME>) --profile <PROFILE_FILE> [--json]
```

#### `emu research profile export`

Exports versioned research profile. Automatically strips all host environment secrets, SSH keys, and credentials (FR-041).

```sh
emu research profile export (--id <GUEST_ID> | --name <GUEST_NAME>) --output <PATH> [--include-guest-data] [--json]
```

#### `emu research profile import`

Imports portable profile, validating local artifact hashes against manifest before registering (FR-040).

```sh
emu research profile import --file <PATH> [--json]
```

#### `emu research baseline create`

Captures verified clean reference state for guest instance rollback (FR-043).

```sh
emu research baseline create (--id <GUEST_ID> | --name <GUEST_NAME>) --deadline-ms <MS> [--json]
```

#### `emu research baseline restore`

Restores guest to verified `RecoveryBaseline`. Destructive: requires two-step authorization (FR-043).

```sh
emu research baseline restore (--id <GUEST_ID> | --name <GUEST_NAME>) [--dry-run] [--authorize <DIGEST>] [--json]
```

#### `emu research baseline inspect`

Inspects baseline verification status and declared recovery SLA.

```sh
emu research baseline inspect (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

---

### Family 10: Operation Status & Cancellation (`operation`)

#### `emu research operation status`

Queries current execution progress, phase, and state of a long-running operation.

```sh
emu research operation status --id <OPERATION_ID> [--json]
```

#### `emu research operation cancel`

Requests immediate cancellation. Acknowledges within <= 200ms with `"cancellation_pending"` and confirms `"cancelled"` upon reaching verified safe boundary (FR-046, SC-015).

```sh
emu research operation cancel --id <OPERATION_ID> [--json]
```

#### `emu research operation wait`

Blocks until operation completes, fails, or timeout elapses. If timeout expires, returns Exit Code 124 (`timed_out`), reporting actual running state without terminating background task (FR-046, SC-016).

```sh
emu research operation wait --id <OPERATION_ID> --timeout <SECS> [--json]
```

---

## 6. Supervisor IPC Interface Contract (`supervisor.sock`)

The supervisor process (`emu __supervise --vm-id <ID>`) listens on `/tmp/emu-<short_uuid>/supervisor.sock` using length-prefixed or line-delimited JSON RPC records:

### 6.1 Supervisor Request Schema

```json
{
  "seq": 101,
  "action": "query_runstate" | "pause" | "resume" | "poweroff" | "acquire_debug_lease" | "release_debug_lease" | "query_privilege",
  "arguments": {}
}
```

### 6.2 Supervisor Response Schema

```json
{
  "seq": 101,
  "status": "ok" | "error",
  "result": {},
  "error": null | {
    "code": "LEASE_HELD" | "INVALID_RUNSTATE" | "QMP_ERROR",
    "message": "Operation blocked by active KernelDebugLease"
  }
}
```

### 6.3 Supervisor Event Notifications

```json
{
  "event": "RUNSTATE_CHANGED",
  "timestamp": "2026-09-17T14:32:00.000Z",
  "data": {
    "previous_state": "running",
    "new_state": "paused",
    "reason": "qmp_stop"
  }
}
```
