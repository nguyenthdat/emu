# CLI Contract: iOS Root VM & Darwin Security Research Automation Interface

**Version**: 1.0.0  
**Feature Branch**: `002-ios-root-vm`  
**Status**: Complete  
**Canonical Schema Identifier**: `https://emu.rs/schemas/v1/research-ios.schema.json`  
**Target Host Platform**: macOS Apple Silicon (`aarch64`, Darwin 24.x/25.x, macOS 15+ candidate)  
**Toolchain Baseline**: Rust 2024 edition, pinned to `rustc 1.88.0` (`.tool-versions`); `bun 1.2.17`  
**Authority**: `specs/002-ios-root-vm/research.md`, `specs/002-ios-root-vm/spec.md`, and `specs/002-ios-root-vm/data-model.md`

---

## 1. Executive Summary & Invocation Architecture

### 1.1 Single-Binary Command Routing (Decision D-04)

The `emu` project maintains a single public binary architecture. Interactive TUI workflows launch when `emu` is invoked without subcommands on an interactive terminal. Headless, non-interactive research automation is completely routed through the `emu research` subcommand hierarchy:

```text
emu [GLOBAL_FLAGS] research <FAMILY> <SUBCOMMAND> [COMMAND_FLAGS]
```

- **Headless Execution**: Every command under `emu research ...` executes directly in a dedicated headless asynchronous routing branch in `src/main.rs`. It completely bypasses interactive TUI initialization (`App::new()`), preventing raw terminal allocation, alternate screen buffer creation, mouse tracking, or UI background polling.
- **Private Supervisor Execution (Decision D-04)**: Running guest virtual machines are supervised by a private child process invocation:
  ```text
  emu __supervise --vm-id <INSTANCE_ID>
  ```
  The supervisor process is not a public CLI subcommand or persistent system daemon (`launchd`). It is an unprivileged child process dedicated to managing one guest's QEMU process, QMP control socket, GDB chardev, console PTY, and advisory file lock. The supervisor initiates the GDB RSP chardev socket upon the first explicit debug entry (`debug pause`) and does not terminate when a guest halts or pauses; it terminates ONLY when the underlying hypervisor process has been reaped AND zero dependent helper resources or workflows remain.
- **Private Worker Execution (Decision D-04)**: Long-running asynchronous operations (image preparation, offline patching, baseline restoration) are executed via:
  ```text
  emu __worker --operation-id <OPERATION_ID>
  ```
  The worker executes bounded mutating steps under an exclusive operation lock (`operations/locks/<op_id>.op.lock`), journals progress atomically, and releases locks upon clean exit or safe boundary recovery.
- **Isolated Frida Worker Binary (`emu-frida-worker`, Decision D-02)**: Dynamic instrumentation is managed via an explicitly separate native helper package located at `tools/frida-worker/` outside the root workspace and default Cargo target graph. This ensures root `--all-targets --all-features` CI runs never require the native `frida-core` C devkit. For lab environments where dynamic instrumentation is exercised, the helper is built via:
  ```sh
  cargo build --manifest-path tools/frida-worker/Cargo.toml --release
  ```
  with the `FRIDA_CORE_DEVKIT` environment variable pointing to the validated devkit directory. The main `emu` executable contains zero Frida C symbols and communicates with `emu-frida-worker` strictly over bounded stdio JSON streams.

### 1.2 Stream Separation & Framing (FR-044)

The automation interface strictly separates machine-readable data payloads from operational progress and diagnostic telemetry:

- **`stdout`**: Reserved exclusively for the finite JSON output contract (`OutputEnvelope`). When `--json` is supplied, `stdout` outputs exactly one valid, complete JSON object upon command completion or terminal error.
- **`stderr`**: Used for diagnostic progress, operational status updates, and warning messages. When `--json` is active, `stderr` outputs append-only, newline-delimited JSON records (`StreamLogEnvelope`), ensuring automated log parsers never parse human-formatted text.
- **IPC Socket Framing (Decision D-04)**: All internal Unix Domain Socket communication between `emu` and child processes (e.g. `supervisor.sock`) uses strictly **newline-delimited UTF-8 JSON** (`\n`). Sockets enforce an explicit maximum frame size of 1 MiB (1,048,576 bytes) per JSON record and a maximum pending queue depth of 256 messages. Oversized frames (>1 MiB) trigger immediate connection termination with a framing error. Length-prefixed framing alternatives are rejected.

---

## 2. Process Exit Code Contract

Exit codes are standardized, deterministic, and categorized across all operational families (FR-045, SC-014, SC-019):

| Exit Code | Outcome Identifier                                           | Classification                       | Description & Trigger Scenarios                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| :-------: | :----------------------------------------------------------- | :----------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  **`0`**  | `"completed"` / `"already_satisfied"` / `"proposal_created"` | **Success**                          | • Operational mutation completed successfully.<br>• Read-only inspection evaluated truthfully (including queries reporting that an optional capability is unavailable or unsupported).<br>• Applying a desired profile that matches current active guest state exactly (idempotent no-op; FR-047).<br>• Command invoked with `--dry-run` generated a valid `MutationProposal` with `proposal_digest`.                                                                                                                             |
|  **`1`**  | `"execution_failed"`                                         | **Runtime Failure**                  | • Guest hypervisor crash, QMP protocol error, or unhandled supervisor fault.<br>• Root proof probe failure (positive write failed or kernel evidence missing).<br>• Injected Frida script syntax error or target process crash.<br>• Kernel debug RSP packet timeout or protocol mismatch.<br>• Companion VM boot or usbmuxd forwarding crash.                                                                                                                                                                                    |
|  **`2`**  | `"invalid_input"`                                            | **Invalid Input**                    | • Missing required command-line arguments or invalid flag parameters.<br>• Invalid flag combinations (e.g. `--json` with interactive `root console`, or `--json` with `--follow` on `operation events`).<br>• Target `display_name` is ambiguous across backends without unique identifier or `--backend` qualification (FR-007).<br>• Referenced image artifact, profile file, or application package not found, truncated, or corrupted (FR-031, FR-032).<br>• Incompatible application binary architecture (non-ARM64 Mach-O). |
|  **`3`**  | `"unsupported"`                                              | **Capability Unsupported**           | • Invoking iOS research backends on non-macOS or non-Apple Silicon hosts (FR-001).<br>• Requesting application installation or Frida app spawning on minimal `darwin-vm` backend (`app_frameworks_supported = false`; FR-023, SC-003).                                                                                                                                                                                                                                                                                            |
|  **`4`**  | `"auth_refused"`                                             | **Authorization Refused**            | • Destructive mutation (disk wipe, instance deletion, baseline rollback, security policy revert) invoked without `--authorize sha256:<digest>` (FR-008).<br>• Host-side image preparation requiring elevated privileges executed in unattended automation mode without pre-authorized credentials (FR-035).<br>• Unauthorized attempt to export application container data or private system paths (FR-024).                                                                                                                      |
|  **`5`**  | `"conflict"`                                                 | **Concurrency / State Conflict**     | • Conflicting concurrent mutations against the same guest instance (instance run lock `instances/locks/<id>.run.lock` held; FR-047, D-04).<br>• Conflicting operation lock (`operations/locks/<op_id>.op.lock`) or device lock held.<br>• Kernel debug lease held by another active debugger client (D-05).<br>• Companion VM stop rejected because live dependent guest sessions remain bound (FR-038, SC-011).                                                                                                                  |
| **`124`** | `"timeout"` / `"cancellation_pending"`                       | **Timeout / Pending Safe Cessation** | • Caller wait deadline elapsed before task reached completion; actual guest state is reported as continuing, stopped, or unknown without stopping background work (FR-046, SC-016).<br>• Cancellation requested, but safe transaction boundary was not yet reached before caller wait deadline expired.                                                                                                                                                                                                                           |
| **`130`** | `"cancelled"`                                                | **Cancelled at Safe Boundary**       | • Explicit user cancellation signal (`emu research operation cancel --id <ID>`) successfully halted the operation at a verified safe transaction boundary with disposable cleanup (FR-046, SC-015).                                                                                                                                                                                                                                                                                                                               |

### 2.1 Observer Detach vs. Operation Cancellation (FR-046, Decision D-05)

- **Interactive Observer Detach (`Ctrl+C` / SIGINT)**: When an interactive CLI command or TUI session receives `Ctrl+C` or disconnects, the client process **detaches its local observer only**. The detached client NEVER sends RSP `D` (detach), `c` (continue), or `k` (kill) to QEMU, and NEVER issues QMP `stop`. The underlying guest and supervisor/worker processes continue running unhindered in the background.
- **Cooperative Operation Cancellation**: Terminating an in-flight operation requires explicitly invoking:
  ```sh
  emu research operation cancel --id <OPERATION_ID>
  ```
  The supervisor or worker acknowledges the request within `<= 200ms` by setting `outcome: "cancellation_pending"`. The executing worker halts only upon reaching the next verified safe transaction boundary, executes deterministic multi-stage disposable cleanup, records any partial commits or residual resources, and confirms `outcome: "cancelled"` with Exit Code 130.

---

## 3. Standard JSON Envelopes & Execution Rules

### 3.1 OutputEnvelope (`stdout`)

Every command executed with `--json` outputs exactly one `OutputEnvelope` on `stdout`:

```json
{
  "$schema": "https://emu.rs/schemas/v1/research-ios.schema.json#/definitions/OutputEnvelope",
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

- `$schema` (`string`): Schema reference pointing to `https://emu.rs/schemas/v1/research-ios.schema.json#/definitions/OutputEnvelope`.
- `status` (`string`): High-level operational status classification:
  - `"success"`: Operation completed successfully or read query evaluated.
  - `"already_satisfied"`: Desired profile already active; zero redundant actions taken.
  - `"accepted"`: Dry-run accepted and mutation proposal generated.
  - `"failed"`: Runtime or probe verification failure.
  - `"rejected"`: Refused preflight due to input, validation, authorization, or support constraints.
  - `"timed_out"`: Caller wait deadline elapsed before task settled.
  - `"cancelled"`: Halted at safe transaction boundary.
- `outcome` (`string`): Exact outcome identifier matching the Exit Code Contract:
  `"completed"` | `"already_satisfied"` | `"proposal_created"` | `"auth_refused"` | `"invalid_input"` | `"unsupported"` | `"conflict"` | `"timeout"` | `"cancellation_pending"` | `"cancelled"` | `"execution_failed"`
- `operation_id` (`string`): Tracking ID matching the pattern `^op_[0-9A-Za-z]+$`.
- `data` (`object | null`): Command-specific payload projection. On runtime error or cancellation, `data` MAY carry partial committed artifacts, reclaimed paths, or residual resource records rather than being forced to null.
- `error` (`ErrorRecord | null`): Structured error information on non-zero exit, or `null` on success:

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

When `--json` is active, `stderr` emits newline-delimited JSON objects representing operational progress:

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

#### Field Definitions:

- `timestamp` (`string`): RFC 3339 UTC timestamp.
- `level` (`string`): `"DEBUG"` | `"INFO"` | `"WARN"` | `"ERROR"`.
- `event` (`string`): Machine-readable event tag (e.g. `"QMP_STATUS_CHANGED"`, `"DISK_PREPARED"`, `"PROBE_EVALUATED"`).
- `phase` (`string`): Current execution phase (`"initializing"`, `"booting"`, `"executing"`, `"recovering"`, `"cleaning"`).
- `message` (`string`): Diagnostic progress description.
- `operation_id` (`string`): Tracking operation ID.

### 3.3 Valid & Invalid Flag Combinations, Deadlines, and Envelope Invariants

#### Exactly-One-Envelope Rule on `stdout`

To ensure automated tools can reliably pipe `stdout` directly into `jq` or JSON parsers without streaming boundary corruption:

1. `stdout` produces **strictly one** top-level JSON `OutputEnvelope` upon command termination.
2. Continuous diagnostic streaming is routed exclusively to `stderr` as `StreamLogEnvelope` JSONL records.

#### Specific Stream Rules for Interactive & Streaming Commands

- **`root console`**:
  - `emu research root console --id <ID> --command <CMD> [--json]`: Valid. Runs non-interactively inside the guest bootstrap shell, capturing output into `OutputEnvelope.data` (`exit_code`, `stdout`, `stderr`).
  - `emu research root console --id <ID> --json` (interactive mode without `--command`): **Invalid Flag Combination**. Interactive terminal sessions allocate a PTY and cannot emit a single finite JSON envelope. Rejected with Exit Code 2 (`invalid_input`): `details.remediation: "Interactive console requires terminal PTY; pass --command <CMD> for JSON output or omit --json for interactive terminal."`
- **`operation events`**:
  - `emu research operation events --id <OP_ID> [--cursor <N>] [--json]`: Valid. Returns a single finite `OutputEnvelope` on `stdout` containing the buffered slice of log events in `data.events` and `data.next_cursor`.
  - `emu research operation events --id <OP_ID> --follow`: Valid. Streams human-readable log lines to `stderr`.
  - `emu research operation events --id <OP_ID> --follow --json`: **Invalid Flag Combination**. Continuous streaming on `stdout` violates the single-envelope contract. Rejected with Exit Code 2 (`invalid_input`): `details.remediation: "To stream live logs, omit --json to stream to stderr or poll with 'operation events --cursor <N> --json' for machine-readable envelopes."`

#### Invalid Flag Combinations Matrix

| Command                       | Flag Combination                                 |    Outcome Exit Code    | Error Description                                                                 |
| :---------------------------- | :----------------------------------------------- | :---------------------: | :-------------------------------------------------------------------------------- |
| Any mutating command          | `--dry-run` + `--authorize <DIGEST>`             | **2** (`invalid_input`) | Mutually exclusive: `--dry-run` generates proposals; `--authorize` consumes them. |
| Any command                   | `--simulate` / `--simulate-*`                    | **2** (`invalid_input`) | Production CLI prohibits test simulation flags.                                   |
| `guest delete` / `guest wipe` | `--unattended` without `--authorize`             | **4** (`auth_refused`)  | Destructive actions in unattended mode require pre-authorized digest.             |
| `image prepare`               | `--unattended` requiring elevated sudo           | **4** (`auth_refused`)  | Unattended execution cannot prompt for interactive elevation.                     |
| `companion stop`              | with active dependent guests (without `--force`) |   **5** (`conflict`)    | Cannot terminate companion while live guest sessions depend on it.                |

#### Finite Default Waits & Task-Deadline Semantics

- **Default Wait Timeouts**: Blocking lifecycle commands enforce finite default wait deadlines when `--timeout` is omitted:
  - `guest start`: default 30 seconds.
  - `guest stop`: default 20 seconds.
  - `operation wait`: default 30 seconds.
  - `image prepare`: default 60 seconds.
- **Task-Deadline Semantics (FR-046, SC-016)**:
  - When the caller wait deadline elapses before the background operation concludes, the CLI process terminates immediately with **Exit Code 124** (`timed_out` / `timeout`).
  - **A caller timeout is NOT a terminal task status**. The underlying hypervisor, supervisor, or worker process **continues executing** unhindered in the background.
  - The returned `OutputEnvelope` on `stdout` reports `status: "timed_out"`, `outcome: "timeout"`, and `data` carrying the actual observed background state (`execution_state: "continuing"`, `current_phase`, and tracking IDs).
  - The caller can subsequently check status via `operation status --id <ID>` or resume waiting via `operation wait --id <ID> --timeout <SECS>`.

---

## 4. Two-Step Authorization Contract for Destructive Actions & Locking Invariants

Destructive operations (instance deletion, disk wiping, baseline rollback, in-guest security policy modification) require explicit confirmation identifying the affected operation, target instance ID (or target resource when pre-instance), backend, and persistent data paths (FR-008).

### 4.1 Locking Architecture & Invariants (Decision D-04)

Cross-process synchronization and mutation safety are governed by advisory file locking (`fs4 = "1.1.0"`, feature `sync`) on permanent, dedicated lockfiles:

- **Permanent Never-Unlinked Lockfiles**:
  - `instances/locks/<id>.run.lock`: Exclusive hold while guest hypervisor/supervisor is active.
  - `instances/locks/<id>.device.lock`: Exclusive hold during instance disk mutation or state wipe.
  - `operations/locks/<op_id>.op.lock`: Exclusive hold while worker executes an asynchronous operation.
  - _Lockfile Retention Invariant_: Lockfiles are permanently retained on disk and **never unlinked or deleted**, preventing inode-recycling race conditions. Ownership is governed strictly by `fs4` advisory flock semantics.
- **Non-Blanket Run Lock**: Holding an instance run lock (`<id>.run.lock`) prevents conflicting lifecycle mutations (e.g. concurrent boots or deletions), but does **not** blanket-deny valid routed mutations or read queries (e.g. debug operations, console access, or status queries).
- **Two-Step Digest Consumption**: When an authorized command with `--authorize sha256:<digest>` is received:
  1. Under the target instance lock, the coordinator verifies that the proposal exists, has not expired (15-minute validity window), matches target parameters, and matches the active configuration revision.
  2. The proposal is marked consumed **before** any disk mutations or destructive effects are initiated.
  3. If hypervisor execution crashes or an unexpected fault occurs mid-mutation, the operation is recorded as `failed` or `unknown`. **Automatic retries are strictly prohibited** (FR-047).
- **Application-Enforced Record Immutability**: Historical audit files under `records/<record_id>.json` are protected against mutation by application-level write-once checks and read-only opening flags, rather than relying on brittle host `chmod` filesystem permissions.
- **Secret Stripping on Export**: Profile exports (`profile export`) automatically allowlist configuration items while stripping host environment variables, SSH private keys, and authentication credentials (FR-041). Guest container data and logs are exported only upon explicit user authorization (`--include-guest-data`).
- **Authoritative Companion Sets**: The companion environment tracks authoritative live reference sets: `live_guest_ids` (dependent guest UUIDs) and `active_operation_ids` (active restore tasks). Numerical counts (`live_dependents_count`, `active_workflows_count`) are strictly derived from these sets (FR-038).

### 4.2 Proposal Generation (`--dry-run`)

```sh
emu research guest delete --id e4b1c2d3-4567-89ab-cdef-0123456789ab --dry-run --json
```

**Stdout (Exit Code 0, outcome: `"proposal_created"`)**:

```json
{
  "$schema": "https://emu.rs/schemas/v1/research-ios.schema.json#/definitions/OutputEnvelope",
  "status": "accepted",
  "outcome": "proposal_created",
  "operation_id": "op_01J8DELPROP01",
  "data": {
    "proposal_id": "prop_01J8DEL001",
    "proposal_digest": "sha256:a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0",
    "operation_type": "instance_delete",
    "target_instance_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "target_resource": null,
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

### 4.3 Authorized Execution

```sh
emu research guest delete --id e4b1c2d3-4567-89ab-cdef-0123456789ab \
  --authorize sha256:a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0 --json
```

**Stdout (Exit Code 0, outcome: `"completed"`)**:

```json
{
  "$schema": "https://emu.rs/schemas/v1/research-ios.schema.json#/definitions/OutputEnvelope",
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

_Triggering destructive actions without `--authorize` or `--dry-run` aborts immediately with Exit Code 4 (`auth_refused`), outputting the required proposal and instructions._

---

## 5. Command Family Specifications

### Family 1: Backend Discovery & Preflight (`backend`)

#### `emu research backend preflight`

Non-interactively evaluates host CPU architecture (`aarch64`), `Hypervisor.framework` availability, permissions, and toolchain dependencies (FR-001, FR-002).

```sh
emu research backend preflight [--json]
```

#### `emu research backend list`

Lists supported research backends and operational capability summaries.

```sh
emu research backend list [--json]
```

---

### Family 2: Guest Lifecycle Management (`guest`)

#### `emu research guest create`

Registers and initializes a new research guest instance with a globally unique UUIDv4 (FR-005, FR-006). Supports profile-driven creation or discrete boot artifact role mappings (`--root-disk`, `--kernelcache`, `--devicetree`, `--trustcache`, `--ramdisk`, `--sptm-firmware`, `--sep-firmware`, `--nvram-template`):

```sh
# Option A: Create via registered profile
emu research guest create --name <NAME> --profile <PROFILE_ID_OR_PATH> [--json]

# Option B: Create via backend and discrete artifact digests
emu research guest create \
  --name <NAME> \
  --backend <darwin-vm|Inferno> \
  --root-disk <ARTIFACT_DIGEST> \
  [--kernelcache <DIGEST>] \
  [--devicetree <DIGEST>] \
  [--trustcache <DIGEST>] \
  [--ramdisk <DIGEST>] \
  [--kernel-args <ARGS>] \
  [--json]
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

#### `emu research guest wipe`

Wipes guest user disk and volatile modifications back to initial registered baseline state (`DeviceManager::wipe_device` semantics). Destructive action requiring two-step authorization.

```sh
emu research guest wipe (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--dry-run] [--authorize <DIGEST>] [--json]
```

---

### Family 3: Root Proof & Privilege Verification (`root`)

#### `emu research root verify`

Executes empirical root proof workflow: runs positive probe fixture (`/private/var/root/.emu_probe`) as UID 0, executes negative control check dropping to non-zero UID asserting `EACCES`/`EPERM`, and captures kernel evidence (FR-010, FR-012, FR-013).

```sh
emu research root verify (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--test-binary <PATH>] [--json]
```

#### `emu research root status`

Returns desired vs observed privilege state for a guest instance (FR-014).

```sh
emu research root status (--id <UUID> | --name <NAME> [--backend <BACKEND>]) [--json]
```

#### `emu research root console`

Connects an interactive terminal or executes commands on the guest root bootstrap console (FR-011).

```sh
# Interactive session (PTY required; rejects --json with exit code 2)
emu research root console (--id <UUID> | --name <NAME> [--backend <BACKEND>])

# Non-interactive command execution (emits single finite OutputEnvelope)
emu research root console (--id <UUID> | --name <NAME> [--backend <BACKEND>]) --command <CMD> [--json]
```

#### `emu research root fs-read`

Reads an authorized file path directly from the in-guest root filesystem via the bootstrap bridge without host disk mounting (FR-025).

```sh
emu research root fs-read --guest-id <UUID> --path <GUEST_PATH> [--json]
```

#### `emu research root fs-write`

Writes a local file to the in-guest root filesystem without host disk mounting (FR-025).

```sh
emu research root fs-write --guest-id <UUID> --path <GUEST_PATH> --src <LOCAL_PATH> [--authorize <DIGEST>] [--json]
```

#### `emu research root fs-export`

Exports an in-guest directory or file to the host destination under explicit researcher authorization (FR-025).

```sh
emu research root fs-export --guest-id <UUID> --path <GUEST_PATH> --dest <HOST_PATH> [--authorize <DIGEST>] [--json]
```

#### `emu research root ps`

Enumerates active guest processes via in-guest shell/Frida worker on `Inferno` (FR-026).

```sh
emu research root ps --guest-id <UUID> [--json]
```

#### `emu research root mach-services`

Enumerates registered Mach system and user services on `Inferno` (FR-026).

```sh
emu research root mach-services --guest-id <UUID> [--json]
```

#### `emu research root security inspect`

Inspects the runtime `GuestSecurityProfile` applied to the guest (code signing, AMFI, sandbox modes) (FR-030).

```sh
emu research root security inspect --guest-id <UUID> [--json]
```

#### `emu research root security apply`

Applies a declared `GuestSecurityProfile` to the guest (FR-030).

```sh
emu research root security apply --guest-id <UUID> --profile <PROFILE_ID|PATH> [--authorize <DIGEST>] [--json]
```

#### `emu research root security revert`

Reverts the guest security policy back to a verified baseline configuration (NOT an assumed universal stock state) (FR-030).

```sh
emu research root security revert --guest-id <UUID> --baseline-id <BASELINE_ID> [--dry-run] [--authorize <DIGEST>] [--json]
```

---

### Family 4: Application Lifecycle Management (`app`)

_Supported exclusively on `Inferno`. Rejected with Exit Code 3 (`unsupported`) on `darwin-vm` (FR-023, SC-003)._

#### `emu research app import`

Validates Mach-O headers (ARM64 architecture) and registers an owned application artifact (FR-022, FR-031).

```sh
emu research app import --package <PATH_TO_IPA_OR_APP> [--json]
```

#### `emu research app install`

Installs application onto running `Inferno` guest via `ideviceinstaller` over the companion usbmuxd bridge (FR-016, D-03).

```sh
emu research app install (--id <GUEST_ID> | --name <GUEST_NAME>) --app-id <APP_ID> [--json]
```

#### `emu research app list`

Queries installed applications on the target guest using `ideviceinstaller -u <UDID> list --json` (D-03).

```sh
emu research app list (--id <GUEST_ID> | --name <GUEST_NAME>) [--json]
```

#### `emu research app launch`

Spawns target application and verifies sandboxed container creation under `/private/var/mobile/Containers/Data/Application/<UUID>` (D-03).

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

#### `emu research app container-write`

Writes a local file to an authorized destination inside the target application data container (FR-024).

```sh
emu research app container-write --guest-id <GUEST_ID> --bundle-id <BUNDLE_ID> --src <LOCAL_PATH> --dest <CONTAINER_PATH> [--authorize <DIGEST>] [--json]
```

#### `emu research app container-export`

Exports container directory to host destination under explicit researcher authorization (FR-024).

```sh
emu research app container-export (--id <GUEST_ID> | --name <GUEST_NAME>) --bundle-id <BUNDLE_ID> --destination <HOST_DIR> [--authorize-export] [--json]
```

---

### Family 5: Frida Dynamic Instrumentation (`frida`)

_Managed via private child executable `emu-frida-worker` linking official `frida-core` 17.18.0 C devkit over bounded stdio JSON (D-02)._

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

#### `emu research frida configure`

Configures the in-guest Frida agent with a validated options file containing the pinned server TLS certificate fingerprint, per-session authentication token, and guest bridge endpoints over an owned SSH bridge with pinned host key (FR-017, D-02).

```sh
emu research frida configure --guest-id <GUEST_ID> --options-file <PATH> [--json]
```

#### `emu research frida start`

Starts agent daemon inside guest, binding strictly to loopback bridge (D-02, FR-039).

```sh
emu research frida start (--id <GUEST_ID> | --name <GUEST_NAME>) [--port 27042] [--json]
```

#### `emu research frida attach`

Attaches dynamic instrumentation probes to a target PID, bundle ID, or daemon using user-supplied pre-bundled script (FR-018, FR-020). Asserts probe specificity against an uninstrumented control process (FR-021, SC-002).

```sh
emu research frida attach (--id <GUEST_ID> | --name <GUEST_NAME>) \
  (--pid <PID> | --bundle-id <BUNDLE_ID> | --daemon <NAME>) \
  --script <SCRIPT_PATH> \
  [--control-pid <CONTROL_PID>] \
  [--json]
```

#### `emu research frida spawn`

Spawns target bundle ID under suspended execution, injects user-supplied script with pre-bundled `frida-objc-bridge`, and optionally resumes execution (FR-018).

```sh
emu research frida spawn --guest-id <GUEST_ID> --bundle-id <BUNDLE_ID> --script <SCRIPT_PATH> [--pause] [--json]
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

Initiates the supervisor's persistent GDB RSP chardev socket connection to QEMU's gdbstub upon this first explicit debug entry (preventing premature hypervisor stop during initial boot in upstream Inferno) and halts virtual CPU execution cleanly. Transitions runstate to `paused` under an exclusive `KernelDebugLease` (D-05, FR-027, FR-028).

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

Disconnects debugger observer client. The supervisor maintains the underlying RSP connection, never forwards `D`/`c`/`k`, and queries QMP `query-status`. If paused, it preserves paused status truthfully without silent resumption (FR-029, SC-006).

```sh
emu research debug disconnect (--id <GUEST_ID> | --name <GUEST_NAME>) [--action <preserve-paused|resume>] [--json]
```

---

### Family 7: Image Preparation & Artifact Management (`image`)

#### `emu research image register`

Computes SHA-256 digest, extracts build identity from plist, and registers artifact (FR-031).
Allowed `--type` values: `kernelcache`, `ramdisk`, `devicetree`, `trustcache`, `root_disk`, `ipsw_restore_bundle`, `sptm_firmware`, `sep_firmware`, `nvram_template`.

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

_Manages local helper Linux VM on macOS Apple Silicon for `Inferno` restore and USB-over-IP workflows using custom `qemu-system-x86_64` under TCG (FR-037, D-08)._

#### `emu research companion start`

Launches companion VM with declared CPU and memory resource limits. Companion listener socket must be launched and ready before Inferno guest connects (FR-037, D-08).

```sh
emu research companion start --parent-guest-id <GUEST_ID> [--cpus 2] [--memory-mb 2048] [--json]
```

#### `emu research companion inspect`

Reports companion lifecycle status, resource consumption, active restore tasks, and authoritative live dependent guest sets (FR-038).

```sh
emu research companion inspect --parent-guest-id <GUEST_ID> [--json]
```

#### `emu research companion stop`

Terminates companion VM. Fails with Exit Code 5 (`conflict`) if live dependent guest sessions remain bound (FR-038, SC-011).

```sh
emu research companion stop --parent-guest-id <GUEST_ID> [--force] [--json]
```

#### `emu research companion status`

Returns overall companion environment health across all active instances.

```sh
emu research companion status [--json]
```

---

### Family 9: Profile & Baseline Management (`profile` / `baseline`)

#### `emu research profile apply`

Applies declared `ResearchExperimentProfile` to a guest instance. Idempotent: returns Exit Code 0 with `outcome: "already_satisfied"` if active configuration already matches (FR-047).

```sh
emu research profile apply (--id <GUEST_ID> | --name <GUEST_NAME>) --profile <PROFILE_FILE> [--json]
```

#### `emu research profile export`

Exports versioned research profile. Automatically strips all host environment secrets, SSH keys, and credentials (FR-041). Guest container data is included only if `--include-guest-data` is supplied.

```sh
emu research profile export (--id <GUEST_ID> | --name <GUEST_NAME>) --output <PATH> [--include-guest-data] [--json]
```

#### `emu research profile import`

Imports portable profile, validating local artifact hashes against manifest before registering (FR-040).

```sh
emu research profile import --file <PATH> [--json]
```

#### `emu research baseline create`

Captures verified clean reference state for guest instance rollback within declared SLA (FR-043).

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

### Family 10: Operation Status, Events & Cancellation (`operation`)

#### `emu research operation status`

Queries current execution progress, phase, and state of a long-running operation.

```sh
emu research operation status --id <OPERATION_ID> [--json]
```

#### `emu research operation cancel`

Requests immediate cooperative cancellation. Acknowledges within `<= 200ms` with `"cancellation_pending"` and confirms `"cancelled"` upon reaching verified safe transaction boundary (FR-046, SC-015).

```sh
emu research operation cancel --id <OPERATION_ID> [--json]
```

#### `emu research operation wait`

Blocks until operation completes, fails, or timeout elapses. If timeout expires, returns Exit Code 124 (`timed_out`), reporting actual continuing state in `data` without terminating background task (FR-046, SC-016).

```sh
emu research operation wait --id <OPERATION_ID> --timeout <SECS> [--json]
```

#### `emu research operation events`

Paginates buffered operational diagnostic log events from `operations/<id>.events.jsonl` using `StreamLogEnvelope` schema.

```sh
# Paged JSON envelope retrieval (strictly one OutputEnvelope on stdout)
emu research operation events --id <OPERATION_ID> [--cursor <N>] [--limit <N>] [--json]

# Continuous live streaming (emits human text on stderr; --json with --follow is rejected)
emu research operation events --id <OPERATION_ID> --follow
```

---

### Family 11: Experiment Record Audit & Inspection (`record`)

_Queries and inspects immutable historical trial audit logs created under FR-042 and SC-018._

#### `emu research record inspect`

Inspects an immutable `ExperimentRecord` file by record UUID (FR-042, SC-018).

```sh
emu research record inspect --id <RECORD_ID> [--json]
```

#### `emu research record list`

Lists stored historical `ExperimentRecord` audits, optionally filtered by guest instance ID or backend.

```sh
emu research record list [--guest-id <UUID>] [--backend <darwin-vm|Inferno>] [--json]
```

---

## 6. Supervisor IPC Interface Contract (`supervisor.sock`)

The private supervisor process (`emu __supervise --vm-id <ID>`) listens on an owner-restricted (`0700`) Unix Domain Socket at `/tmp/emu-<short_uuid>/supervisor.sock`.

### 6.1 Framing Specification

- **Wire Format**: Strict **newline-delimited UTF-8 JSON** (`\n`). Every request, response, and event notification is serialized as a single line terminated by `\n`.
- **Bounded Framing Limits**: Maximum frame size of 1 MiB (1,048,576 bytes); maximum queue depth of 256 pending requests.

### 6.2 Supervisor Request Schema

Requests follow a deterministic RPC model. Actions: `query_runstate`, `pause`, `resume`, `poweroff`, `acquire_debug_lease`, `release_debug_lease`, `query_privilege`.

```json
{
  "seq": 101,
  "action": "query_runstate",
  "arguments": {}
}
```

### 6.3 Supervisor Response Schema

Responses correlate strictly by sequence number (`seq`).

```json
{
  "seq": 101,
  "status": "ok",
  "result": {
    "runstate": "running",
    "debug_lease_active": false
  },
  "error": null
}
```

### 6.4 Supervisor Asynchronous Event Notifications

Events are pushed out-of-band as individual newline-delimited JSON records.

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

---

## 7. Interactive TUI Action Mapping & Navigation

When `emu` is launched on an interactive terminal without subcommands, it enters the full-screen terminal user interface powered by Ratatui and Crossterm (FR-048, SC-015).

### 7.1 Operational Screen Mapping

| Screen / View                    | Navigation Key | Mapped CLI Family      | Primary Interactive Operations                                                                                                                          |
| :------------------------------- | :------------: | :--------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Backends Screen**              |      `1`       | `backend`              | View host preflight diagnostics, hardware acceleration status, and backend capabilities.                                                                |
| **Guests Screen**                |      `2`       | `guest`                | List registered instances, view lifecycle badges, start/stop/restart guests, trigger two-step delete and wipe dialogs.                                  |
| **Root & Security Screen**       |      `3`       | `root`                 | Run root verification wizard, attach interactive root console PTY, view effective vs desired privileges, inspect/revert `GuestSecurityProfile`.         |
| **Applications Screen**          |      `4`       | `app`                  | Import IPA packages, trigger companion usbmuxd installation, launch/stop applications, browse and export sandbox containers.                            |
| **Frida Instrumentation Screen** |      `5`       | `frida`                | Prepare and configure agent, inject pre-bundled scripts, monitor live hook telemetry, verify target specificity, detach probes.                         |
| **Kernel Debugger Screen**       |      `6`       | `debug`                | Pause/resume CPU, inspect/edit 64-bit general-purpose registers, step instructions, set breakpoints, disconnect with safe paused preservation.          |
| **Images & Storage Screen**      |      `7`       | `image`                | Register firmware artifacts, trigger host-side image preparation, verify volume UUID and device nodes, run disposable cleanups.                         |
| **Companion VM Screen**          |      `8`       | `companion`            | Monitor helper Linux VM status, inspect active usbmuxd forwarders and dependent guest bindings, manage companion lifecycle.                             |
| **Profiles & Baselines Screen**  |      `9`       | `profile` / `baseline` | Edit/apply versioned profiles, export secret-stripped profiles, create baseline snapshots, initiate two-step baseline recovery.                         |
| **Operations & Audit Screen**    |      `0`       | `operation` / `record` | Monitor active asynchronous worker operations, view live event logs, issue cooperative cancellation, browse immutable `ExperimentRecord` audit history. |

### 7.2 Interactive Performance SLAs & Invariants (SC-015)

- **Input Responsiveness**: TUI view navigation responds within $\le 100\text{ms}$ across 50 consecutive navigation events.
- **Cancellation Responsiveness**: In-TUI cancellation requests acknowledge within $\le 200\text{ms}$ across 10 consecutive trials without freezing the UI thread.
- **Detachment Invariant**: Pressing `q` or `Esc` to exit the TUI detaches the UI process only; running guest virtual machines, supervisors, and background operations continue running in the background.
