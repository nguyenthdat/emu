# Quickstart: Android Research Backends & Security Tooling

**Version**: 1.0.0  
**Feature Branch**: `specs/001-add-android-research-backends`  
**Status**: Draft  
**Authority**: `specs/001-add-android-research-backends/research.md` and `specs/001-add-android-research-backends/spec.md`

---

## 1. Prerequisites and Laboratory Setup

This guide provides fully documented, non-destructive CLI scenarios covering all 7 user stories, the 8 operational families, and all key validation gates defined in the specification.

### 1.1 Host Environment Support Matrix

| Host Workstation OS                     | Virtualization Prerequisites                                   | Android Emulator Status | Cuttlefish Status | Primary Use Case                                         |
| :-------------------------------------- | :------------------------------------------------------------- | :---------------------: | :---------------: | :------------------------------------------------------- |
| **Linux (Ubuntu 24.04 x86_64 / arm64)** | `/dev/kvm` r/w, `kvm` & `cvdnetwork` groups, `cuttlefish-base` |      **Supported**      |   **Supported**   | Complete empirical validation and Cuttlefish research.   |
| **macOS 12+ (Apple Silicon arm64)**     | `Hypervisor.Framework` (`sysctl kern.hv_support = 1`)          |      **Supported**      |  **Unsupported**  | Development, AVD iteration, and Emulator arm64 research. |
| **Windows 10/11 64-bit**                | Windows Hypervisor Platform (WHPX enabled)                     |      **Supported**      |  **Unsupported**  | Standard Emulator AVD validation.                        |

### 1.2 User-Supplied Artifact Directory Setup

Ensure your local artifacts directory (`~/research_artifacts/`) is populated with compatible binaries matching your target guest architecture:

```bash
mkdir -p ~/research_artifacts
# Fixtures include:
# - bzImage_ksunext_3.3.0_x86_64         (KernelSU Next v3.3.0 virtual_device kernel)
# - ksunext_manager_v3.3.0.apk           (KernelSU Next manager app)
# - frida-server-17.18.0-android-x86_64   (Pinned Frida 17.18.0 daemon)
# - vector_v2.2.apk                      (Vector Xposed manager)
# - neozygisk_v2.4.zip                   (NeoZygisk standalone injector)
# - sample_target_app.apk                (com.example.sampleapp)
# - sample_control_app.apk               (com.example.controlapp)
```

---

## 2. Scenario 1: Backend Discovery, Dual Device Creation & Identity Disambiguation (User Story 1 - P1)

### 2.1 Preflight Compatibility Check

Run non-interactive preflight diagnostics across all virtualization backends:

```bash
emu research backend preflight --json
```

**Stdout Output (Exit Code 0)**:

```json
{
  "$schema": "https://emu.rs/schemas/v1/research.schema.json#/definitions/OutputEnvelope",
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8PREFLIGHT001",
  "data": {
    "profiles": [
      {
        "backend": "emulator",
        "host_os": "linux",
        "host_arch": "x86_64",
        "hypervisor": "kvm",
        "support_status": "supported",
        "required_binaries": [
          {
            "binary_name": "emulator",
            "found": true,
            "version": "37.1.8",
            "resolved_path": "/opt/android-sdk/emulator/emulator"
          },
          {
            "binary_name": "adb",
            "found": true,
            "version": "35.0.2",
            "resolved_path": "/usr/bin/adb"
          }
        ],
        "required_entitlements": [
          {
            "name": "kvm_access",
            "checked": true,
            "granted": true,
            "remediation_hint": "Add user to kvm group"
          }
        ],
        "supported_guest_architectures": ["x86_64"],
        "supports_kernel_injection": true,
        "remediation_steps": []
      },
      {
        "backend": "cuttlefish",
        "host_os": "linux",
        "host_arch": "x86_64",
        "hypervisor": "kvm",
        "support_status": "supported",
        "required_binaries": [
          {
            "binary_name": "cvd",
            "found": true,
            "version": "1.3.0",
            "resolved_path": "/usr/bin/cvd"
          },
          {
            "binary_name": "launch_cvd",
            "found": true,
            "version": "1.3.0",
            "resolved_path": "/usr/bin/launch_cvd"
          }
        ],
        "required_entitlements": [
          {
            "name": "cvdnetwork_group",
            "checked": true,
            "granted": true,
            "remediation_hint": "sudo usermod -aG cvdnetwork $USER"
          }
        ],
        "supported_guest_architectures": ["x86_64"],
        "supports_kernel_injection": true,
        "remediation_steps": []
      }
    ]
  },
  "error": null
}
```

---

### 2.2 Dual Device Creation with Identical Display Names

Create an Android Emulator device and a Cuttlefish device, both assigned the human-friendly display name `"research-pixel"`:

```bash
# 1. Create Android Emulator instance
emu research device create \
  --backend emulator \
  --name "research-pixel" \
  --image-path "/opt/android-sdk/system-images/android-34/google_apis/x86_64/" \
  --arch x86_64 \
  --api-level 34 \
  --json

# 2. Register Cuttlefish instance in isolated runtime root
emu research device register \
  --backend cuttlefish \
  --name "research-pixel" \
  --instance-dir "/var/emu/cuttlefish_instances/cf_instance_01" \
  --json
```

**Verify Inventory Disambiguation (FR-002, SC-001)**:

```bash
emu research device list --json
```

**Stdout Output (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8DEVLIST001",
  "data": {
    "devices": [
      {
        "id": "emulator:avd-pixel-01",
        "backend": "emulator",
        "display_name": "research-pixel",
        "native_name": "pixel_api34_sec",
        "lifecycle_state": "stopped",
        "arch": "x86_64"
      },
      {
        "id": "cuttlefish:cf-pixel-01",
        "backend": "cuttlefish",
        "display_name": "research-pixel",
        "native_name": "cvd_instance_1",
        "lifecycle_state": "stopped",
        "arch": "x86_64"
      }
    ]
  },
  "error": null
}
```

---

### 2.3 Safe Rejection of Ambiguous Display Names (FR-033, SC-015)

Attempting to control a device using the ambiguous display name without backend qualification safely refuses:

```bash
emu research device start --device "research-pixel" --json
```

**Stdout Output (Exit Code 2: `invalid_input`)**:

```json
{
  "status": "rejected",
  "outcome": "invalid_input",
  "operation_id": "op_01J8AMBIG001",
  "data": null,
  "error": {
    "code": "INVALID_INPUT",
    "message": "Target display name 'research-pixel' is ambiguous across multiple backends",
    "details": {
      "candidate_identifiers": [
        "emulator:avd-pixel-01",
        "cuttlefish:cf-pixel-01"
      ],
      "remediation": "Re-run specifying the exact qualified ID via --device <ID>"
    }
  }
}
```

Targeting the qualified ID boots only the designated backend instance:

```bash
emu research device start --device cuttlefish:cf-pixel-01 --wait-ready --timeout 180 --json
```

_(Cuttlefish boots to `running` while `emulator:avd-pixel-01` remains `stopped`)_.

---

### 2.4 Standard Android Device Non-Regression Verification (FR-007, SC-008)

Verify that standard Android virtual devices operating without research tools operate with zero regressions:

```bash
# Standard device command (unaffected by research backends)
emu list --json
```

Baseline devices operate identically, with no research overlays or background task pollution.

---

## 3. Scenario 2: Unattended Automation & Two-Step Destructive Authorization (User Story 2 - P1)

### 3.1 Refusal on Missing Destructive Authorization (FR-006, FR-035)

Attempting to wipe device storage without authorization immediately refuses execution:

```bash
emu research device wipe --device cuttlefish:cf-pixel-01 --json
```

**Stdout Output (Exit Code 4: `auth_refused`)**:

```json
{
  "status": "rejected",
  "outcome": "auth_refused",
  "operation_id": "op_01J8WIPE001",
  "data": null,
  "error": {
    "code": "AUTH_REQUIRED",
    "message": "Destructive operation 'device.wipe' requires explicit authorization",
    "details": {
      "target_device_id": "cuttlefish:cf-pixel-01",
      "affected_resources": [
        "storage:/data/userdata.img",
        "qcow2:overlays_discarded"
      ],
      "hint": "Run with --dry-run to generate a MutationProposal, then provide --authorize sha256:<digest>"
    }
  }
}
```

---

### 3.2 Generating Mutation Proposal via `--dry-run`

```bash
emu research device wipe --device cuttlefish:cf-pixel-01 --dry-run --json
```

**Stdout Output (Exit Code 0: `accepted`)**:

```json
{
  "status": "accepted",
  "outcome": "proposal_created",
  "operation_id": "op_01J8WIPE002",
  "data": {
    "proposal": {
      "proposal_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "action": "device.wipe",
      "target_device_id": "cuttlefish:cf-pixel-01",
      "affected_resources": [
        "storage:/data/userdata.img",
        "qcow2:overlays_discarded"
      ],
      "risk_tier": "destructive_irreversible",
      "requires_reboot": true,
      "invalidates_overlays": true,
      "lossless": false,
      "created_at": "2026-09-17T15:00:00.000Z",
      "expires_at": "2026-09-17T15:15:00.000Z"
    }
  },
  "error": null
}
```

---

### 3.3 Authorized Execution with Stream Separation

Pass the generated `proposal_digest` to execute the wipe:

```bash
emu research device wipe \
  --device cuttlefish:cf-pixel-01 \
  --authorize sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 \
  --json
```

**Stderr Diagnostic Stream (JSONL Envelopes)**:

```json
{"timestamp":"2026-09-17T15:01:00.100Z","level":"INFO","event":"lock_acquired","phase":"preflight","message":"Acquired exclusive device lock for cuttlefish:cf-pixel-01","operation_id":"op_01J8WIPE003"}
{"timestamp":"2026-09-17T15:01:01.450Z","level":"INFO","event":"storage_wiped","phase":"teardown","message":"Executed powerwash_cvd on instance runtime root","operation_id":"op_01J8WIPE003"}
```

**Stdout Result (Exit Code 0: `success`)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8WIPE003",
  "data": {
    "device_id": "cuttlefish:cf-pixel-01",
    "wiped_resources": [
      "storage:/data/userdata.img",
      "qcow2:overlays_discarded"
    ],
    "lifecycle_state": "stopped"
  },
  "error": null
}
```

---

## 4. Scenario 3: KernelSU Next Privilege Escalation and Verification (User Story 3 - P2)

### 4.1 Artifact Validation & Kernel Deployment

Validate user-supplied custom kernel artifact:

```bash
emu research artifact validate \
  --path ~/research_artifacts/bzImage_ksunext_3.3.0_x86_64 \
  --type kernel_image \
  --expected-digest sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08 \
  --json
```

Stage KernelSU Next and apply:

```bash
# 1. Request dry-run
emu research tool enable \
  --device cuttlefish:cf-pixel-01 \
  --tool root:kernelsu_next \
  --artifact ~/research_artifacts/bzImage_ksunext_3.3.0_x86_64 \
  --dry-run --json

# 2. Authorize kernel replacement and restart
emu research tool enable \
  --device cuttlefish:cf-pixel-01 \
  --tool root:kernelsu_next \
  --artifact ~/research_artifacts/bzImage_ksunext_3.3.0_x86_64 \
  --authorize sha256:<PROPOSAL_DIGEST> \
  --json
```

---

### 4.2 Elevated Guest-Scoped Identity Verification (FR-012, Gate T-02)

Verify root privilege execution specifically confirming KernelSU Next identity:

```bash
emu research experiment run-test \
  --device cuttlefish:cf-pixel-01 \
  --test-type root_identity \
  --json
```

**Stdout Output (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8TESTROOT001",
  "data": {
    "device_id": "cuttlefish:cf-pixel-01",
    "test_type": "root_identity",
    "passed": true,
    "probe_details": {
      "command": "/data/adb/ksud debug info",
      "supercall_driver_node": "[ksu_driver]",
      "uapi_version": 2,
      "flavor": "kernelsu_next",
      "version": "3.3.0",
      "host_isolation_verified": true
    }
  },
  "error": null
}
```

Tool state transitions to `ready`:

```bash
emu research tool inspect --device cuttlefish:cf-pixel-01 --tool root:kernelsu_next --json
```

---

## 5. Scenario 4: Frida Dynamic Instrumentation Workflow (User Story 4 - P3)

### 5.1 Frida Deployment & Awaiting Verification State

Deploy Frida 17.18.0 server:

```bash
emu research tool enable \
  --device cuttlefish:cf-pixel-01 \
  --tool frida \
  --artifact ~/research_artifacts/frida-server-17.18.0-android-x86_64 \
  --json
```

Inspect observed state:

```bash
emu research tool inspect --device cuttlefish:cf-pixel-01 --tool frida --json
```

**Observed State is `available_awaiting_verification`** (NOT `ready` until telemetry is observed!).

---

### 5.2 Active Function Tracing & Transition to Ready (FR-014, Gate T-03)

Execute a non-destructive verification trace hooking `getpid` on the sample application:

```bash
emu research experiment trace \
  --device cuttlefish:cf-pixel-01 \
  --target-app com.example.sampleapp \
  --script ~/research_artifacts/probe_getpid.js \
  --duration 5 \
  --json
```

**Stdout Output (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8FRIDATRACE001",
  "data": {
    "target_app": "com.example.sampleapp",
    "events_captured": 4,
    "telemetry": [
      {
        "symbol": "libc.so!getpid",
        "invoked_by": "com.example.sampleapp",
        "pid": 1842
      }
    ],
    "tool_state_transition": {
      "tool_id": "frida",
      "previous_state": "available_awaiting_verification",
      "new_state": "ready"
    }
  },
  "error": null
}
```

---

### 5.3 Observation Interruption Invariant (SC-015)

Stopping an active trace session (via Ctrl+C or `--duration` expiry) **halts host monitoring only**, preserving guest application execution with zero process crashes.

---

## 6. Scenario 5: Vector v2.2 + NeoZygisk v2.4 Framework & Scoped Module Mediation (User Story 5 - P4)

### 6.1 Framework Activation & Pending-Reboot Tracking

Enable Vector framework:

```bash
emu research tool enable \
  --device cuttlefish:cf-pixel-01 \
  --tool framework:vector \
  --json
```

Observed state reports: `pending_reboot`.

Restart device:

```bash
emu research device restart --device cuttlefish:cf-pixel-01 --timeout 120 --json
```

Post-restart probe verifies `/data/adb/lspd/.cli_sock` via `vector-cli --json status`. Observed state transitions to `ready`.

---

### 6.2 Module Installation & Application Scoping (FR-017, Gate T-04)

Deploy sample module and scope strictly to `com.example.sampleapp`:

```bash
emu research tool scope \
  --device cuttlefish:cf-pixel-01 \
  --module com.example.samplemodule \
  --target-app com.example.sampleapp \
  --control-app com.example.controlapp \
  --json
```

Verify scoped mediation:

```bash
emu research experiment run-test \
  --device cuttlefish:cf-pixel-01 \
  --test-type vector_scope \
  --target-app com.example.sampleapp \
  --json
```

**Stdout Output (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8VECTSCOPE001",
  "data": {
    "target_app": "com.example.sampleapp",
    "target_behavior_modified": true,
    "control_app": "com.example.controlapp",
    "control_behavior_modified": false,
    "isolation_verified": true
  },
  "error": null
}
```

---

## 7. Scenario 6: Full Toolchain Coexistence, Provenance & Baseline Recovery (User Story 6 - P5)

### 7.1 Simultaneous Triad Operation (FR-018, Gate T-05)

Run composite evaluation exercising KernelSU Next, Frida, and Vector simultaneously on a single virtual guest:

```bash
emu research experiment run-test \
  --device cuttlefish:cf-pixel-01 \
  --test-type composite_triad \
  --target-app com.example.sampleapp \
  --json
```

**Stdout Output (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8TRIAD001",
  "data": {
    "device_id": "cuttlefish:cf-pixel-01",
    "kernelsu_next_status": "ready",
    "frida_status": "ready",
    "vector_status": "ready",
    "mutual_interference": false,
    "zygote_crashes": 0,
    "kernel_panics": 0
  },
  "error": null
}
```

---

### 7.2 Immutable Provenance Export & Reproducible Apply

Capture and export the full environment audit record:

```bash
# 1. Capture snapshot
emu research provenance capture \
  --device cuttlefish:cf-pixel-01 \
  --label "baseline-triad-eval" \
  --json

# 2. Export record
emu research provenance export \
  --id prov_01J8CAPTURE001 \
  --out ~/research_artifacts/triad_provenance.json \
  --json
```

Apply this profile to reconstruct an identical environment on a fresh instance:

```bash
emu research profile apply \
  --device cuttlefish:cf-fresh-01 \
  --profile ~/research_artifacts/triad_provenance.json \
  --wait --timeout 300 \
  --json
```

---

### 7.3 Experimental Failure & Safe Baseline Recovery (FR-021, SC-005)

Simulate an experimental kernel bootloop:

```bash
# Restore working baseline within 3 minutes
emu research baseline restore \
  --device cuttlefish:cf-pixel-01 \
  --authorize sha256:<BASELINE_PROPOSAL_DIGEST> \
  --timeout 180 \
  --json
```

Device restores to verified baseline disk and kernel state within 3 minutes.

---

## 8. Scenario 7: Extended Capability Catalog, Conflict Rejection & Idempotence (User Story 7 - P6)

### 8.1 Rejection of Conflicting Capabilities (FR-028, SC-013)

Attempting to apply an extended research profile containing mutually incompatible capabilities (e.g. KowSU + SUSFS) fails preflight:

```bash
emu research profile apply \
  --device cuttlefish:cf-pixel-01 \
  --profile ~/research_artifacts/profiles/kowsu_susfs_conflict.json \
  --json
```

**Stdout Output (Exit Code 5: `conflict`)**:

```json
{
  "status": "rejected",
  "outcome": "conflict",
  "operation_id": "op_01J8CONFLICT001",
  "data": null,
  "error": {
    "code": "CAPABILITY_CONFLICT",
    "message": "Mutually conflicting capabilities detected in research profile",
    "details": {
      "conflicts": [
        {
          "capability_a": "cap_kowsu",
          "capability_b": "cap_susfs",
          "reason": "KowSU kernel symbols abort SUSFS compilation"
        }
      ]
    }
  }
}
```

---

### 8.2 Physical Hardware Non-Applicability Truthfulness (FR-030, SC-012)

Querying or enabling Baseband Guard partition protection on a virtual device reports `not_applicable`:

```bash
emu research tool enable \
  --device cuttlefish:cf-pixel-01 \
  --tool cap_baseband_guard \
  --json
```

**Stdout Output (Exit Code 3: `unsupported`)**:

```json
{
  "status": "rejected",
  "outcome": "unsupported",
  "operation_id": "op_01J8BBG001",
  "data": {
    "tool_id": "cap_baseband_guard",
    "observed_state": "not_applicable",
    "reason": "Virtual research devices lack physical cellular modems and radio flash partitions",
    "host_partitions_accessed": 0
  },
  "error": {
    "code": "UNSUPPORTED_HARDWARE",
    "message": "Physical hardware feature is not applicable on virtual research devices"
  }
}
```

---

### 8.3 Repeat Declarative Apply Idempotence (FR-037, SC-016)

Re-applying an already-satisfied research profile against a device whose observed state matches returns immediately:

```bash
emu research profile apply \
  --device cuttlefish:cf-pixel-01 \
  --profile ~/research_artifacts/triad_provenance.json \
  --json
```

**Stdout Output (Exit Code 0: `already_satisfied`)**:

```json
{
  "status": "already_satisfied",
  "outcome": "already_satisfied",
  "operation_id": "op_01J8REPEAT001",
  "data": {
    "device_id": "cuttlefish:cf-pixel-01",
    "mutations_performed": 0,
    "kernel_flashes": 0,
    "reboots_triggered": 0,
    "message": "Device already matches all declared desired states"
  },
  "error": null
}
```

---

### 8.4 Operation Cancellation & Timeout Handling (FR-036, SC-015)

Cancel an in-progress operation:

```bash
emu research operation cancel \
  --operation-id op_01J8LONGOP001 \
  --wait-safe-boundary \
  --timeout 15 \
  --json
```

**Stdout Output (Exit Code 130: `cancelled`)**:

```json
{
  "status": "cancelled",
  "outcome": "cancelled",
  "operation_id": "op_01J8LONGOP001",
  "data": {
    "checkpoint": "kernel_staged_pre_reboot",
    "execution_state": "stopped",
    "partial_modifications": ["staged_file:/tmp/bzImage_staged"]
  },
  "error": null
}
```

If the deadline elapses before the safe boundary is reached, exit code is `124` (`cancellation_pending`), accurately reporting `execution_state: "continuing"` without falsely claiming cessation.
