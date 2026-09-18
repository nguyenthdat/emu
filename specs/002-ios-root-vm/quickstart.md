# Quickstart: iOS Root VM & Darwin Security Research Backends

**Version**: 1.0.0  
**Feature Branch**: `002-ios-root-vm`  
**Status**: Complete  
**Authority**: `specs/002-ios-root-vm/research.md`, `specs/002-ios-root-vm/spec.md`, `specs/002-ios-root-vm/data-model.md`, and `specs/002-ios-root-vm/contracts/cli.md`

---

## 1. Prerequisites and Laboratory Setup

This guide provides fully documented, non-destructive CLI scenarios covering all 8 user stories, all 10 operational families, and the 19 success criteria defined in the specification.

### 1.1 Host Environment Support Matrix

| Host Workstation OS                   | Architecture                  | Hypervisor Extensions                                 | `darwin-vm` Status | `Inferno` Status | Primary Research Scope                                                                                                       |
| :------------------------------------ | :---------------------------- | :---------------------------------------------------- | :----------------: | :--------------: | :--------------------------------------------------------------------------------------------------------------------------- |
| **macOS 15+ (Sequoia) / Darwin 25.x** | **Apple Silicon (`aarch64`)** | `Hypervisor.framework` (`sysctl kern.hv_support = 1`) |   **Supported**    |  **Supported**   | Complete dual-backend research, root proof, kernel debugging, Frida dynamic instrumentation, and companion VM orchestration. |
| **macOS 14 (Sonoma)**                 | **Apple Silicon (`aarch64`)** | `Hypervisor.framework` (`sysctl kern.hv_support = 1`) |   **Supported**    |  **Supported**   | Standard dual-backend research execution.                                                                                    |
| **macOS (Intel `x86_64`)**            | Intel 64-bit                  | Hypervisor / VMX                                      |  **Unsupported**   | **Unsupported**  | Apple Silicon ARM64 hypervisor primitives required (FR-001).                                                                 |
| **Linux / Windows**                   | x86_64 / arm64                | KVM / WHPX                                            |  **Unsupported**   | **Unsupported**  | Darwin security research backends require macOS host platform (FR-001).                                                      |

### 1.2 User-Supplied Artifact Directory Setup

Ensure your local artifacts directory (`~/research_artifacts/`) is populated with compatible components matching your target guest architecture:

```bash
mkdir -p ~/research_artifacts
# Candidate artifacts include:
# - darwin_bootkc_minimal.bin          (Extracted Mach kernelcache for darwin-vm)
# - darwin_dtree_minimal.dtb           (Device tree blob)
# - darwin_root_ramdisk.img            (Minimal root ramdisk with bootstrap console)
# - inferno_kernelcache_18A5351d       (iOS 14.0 beta 5 kernelcache for d421ap)
# - inferno_dtree_d421ap.dtb           (iPhone 11 device tree)
# - inferno_rootfs_18A5351d.raw        (Prepared iOS research root disk image)
# - frida-core-devkit-17.18.0-mac      (Frida 17.18.0 C devkit for emu-frida-worker)
# - frida-server-17.18.0-ios-arm64     (Frida 17.18.0 guest agent package)
# - SampleResearchApp.ipa              (Owned test application com.example.researchapp)
# - ControlApp.ipa                     (Uninstrumented control com.example.controlapp)
# - benign_test_binary                 (Benign guest CLI verification binary)
```

---

## 2. Scenario 1: Host Preflight Diagnostics & Dual-Backend Guest Creation (User Story 1 - P1)

### 2.1 Preflight Compatibility Check

Run non-interactive preflight diagnostics across both research backends:

```bash
emu research backend preflight --json
```

**Stdout (Exit Code 0)**:

```json
{
  "$schema": "https://emu.rs/schemas/v1/research-ios.schema.json#/definitions/OutputEnvelope",
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8PREFLIGHT001",
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
        "host_os": "macos",
        "host_arch": "aarch64",
        "hypervisor": "hypervisor_framework",
        "support_status": "supported",
        "supported_guest_families": ["darwin-minimal"],
        "headless_console_support": true,
        "graphical_display_support": false,
        "companion_vm_required": false,
        "app_frameworks_supported": false,
        "supported_debug_interfaces": ["gdb_rsp", "qmp_monitor"],
        "required_binaries": [
          {
            "binary_name": "qemu-system-aarch64",
            "found": true,
            "resolved_path": "/opt/homebrew/bin/qemu-system-aarch64"
          }
        ],
        "required_entitlements": [
          {
            "name": "hypervisor_entitlement",
            "granted": true,
            "details": "com.apple.security.hypervisor present"
          }
        ],
        "remediation_steps": []
      },
      {
        "backend": "Inferno",
        "host_os": "macos",
        "host_arch": "aarch64",
        "hypervisor": "hypervisor_framework",
        "support_status": "supported",
        "supported_guest_families": ["ios-14", "ios-15"],
        "headless_console_support": true,
        "graphical_display_support": true,
        "companion_vm_required": true,
        "app_frameworks_supported": true,
        "supported_debug_interfaces": ["gdb_rsp", "qmp_monitor"],
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
        "required_entitlements": [
          {
            "name": "hypervisor_entitlement",
            "granted": true,
            "details": "com.apple.security.hypervisor present"
          }
        ],
        "remediation_steps": []
      }
    ]
  },
  "error": null
}
```

### 2.2 Dual-Backend Guest Registration with Identical Display Name

Register two guest instances with identical display name `"ios-sec-lab"` under `darwin-vm` and `Inferno` to verify unambiguous UUID assignment and identity disambiguation (FR-005, FR-007, SC-017):

```bash
# 1. Register darwin-vm guest
emu research guest create \
  --name "ios-sec-lab" \
  --backend darwin-vm \
  --image sha256:d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456 \
  --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8GUESTCREATE01",
  "data": {
    "id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "display_name": "ios-sec-lab",
    "backend": "darwin-vm",
    "lifecycle_state": "stopped",
    "guest_arch": "arm64",
    "guest_os_version": "Darwin 20.0.0 minimal",
    "build_identity": "20A2411",
    "observed_privilege": "unverified",
    "created_at": "2026-09-17T14:30:00.000Z"
  },
  "error": null
}
```

```bash
# 2. Register Inferno guest with the identical display name
emu research guest create \
  --name "ios-sec-lab" \
  --backend Inferno \
  --image sha256:c1d2e3f405162738495a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e \
  --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8GUESTCREATE02",
  "data": {
    "id": "f5c2d3e4-5678-9abc-def0-123456789abc",
    "display_name": "ios-sec-lab",
    "backend": "Inferno",
    "lifecycle_state": "stopped",
    "guest_arch": "arm64",
    "guest_os_version": "iOS 14.0 beta 5",
    "build_identity": "18A5351d",
    "observed_privilege": "unverified",
    "created_at": "2026-09-17T14:30:05.000Z"
  },
  "error": null
}
```

### 2.3 Ambiguous Target Name Collision Rejection & Disambiguation

Attempting to inspect or start without specifying the backend or UUID is rejected (FR-007):

```bash
emu research guest inspect --name "ios-sec-lab" --json
```

**Stdout (Exit Code 2 `invalid_input`)**:

```json
{
  "status": "rejected",
  "outcome": "invalid_input",
  "operation_id": "op_01J8INSPECTERR01",
  "data": null,
  "error": {
    "code": "AMBIGUOUS_INSTANCE_NAME",
    "message": "Multiple guest instances found matching display name 'ios-sec-lab'",
    "details": {
      "matching_instances": [
        {
          "id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
          "backend": "darwin-vm"
        },
        { "id": "f5c2d3e4-5678-9abc-def0-123456789abc", "backend": "Inferno" }
      ],
      "remediation": "Qualify query with --backend <darwin-vm|Inferno> or target directly by --id <UUID>"
    }
  }
}
```

### 2.4 Independent Guest Launch

Boot the `darwin-vm` instance cleanly:

```bash
emu research guest start --id e4b1c2d3-4567-89ab-cdef-0123456789ab --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8GUESTSTART01",
  "data": {
    "id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "display_name": "ios-sec-lab",
    "backend": "darwin-vm",
    "lifecycle_state": "running",
    "boot_session_id": "9a8b7c6d-5e4f-3a2b-1c0d-ef9876543210",
    "qmp_socket_path": "/tmp/emu-e4b1c2d3/qmp.sock",
    "console_socket_path": "/tmp/emu-e4b1c2d3/console.sock",
    "observed_privilege": "unverified"
  },
  "error": null
}
```

---

## 3. Scenario 2: Verifiable iOS Root Proof & Falsification Controls (User Story 2 - P1)

### 3.1 Empirical Root Proof Verification

Execute the privilege verification workflow against the running `darwin-vm` guest:

```bash
emu research root verify \
  --id e4b1c2d3-4567-89ab-cdef-0123456789ab \
  --test-binary ~/research_artifacts/benign_test_binary \
  --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8ROOTVERIFY01",
  "data": {
    "evidence_id": "7b8c9d0e-1f2a-3b4c-5d6e-7f8a9b0c1d2e",
    "guest_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "boot_session_id": "9a8b7c6d-5e4f-3a2b-1c0d-ef9876543210",
    "backend": "darwin-vm",
    "guest_build_identity": "20A2411",
    "image_artifact_digest": "sha256:d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456",
    "config_revision_hash": "sha256:3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b",
    "verified_uid": 0,
    "positive_probe_outcome": {
      "path": "/private/var/root/.emu_probe",
      "status": "success",
      "target_user": "root",
      "uid": 0,
      "expected_denial": false,
      "output": "emu_root_verified"
    },
    "negative_control_outcome": {
      "path": "/private/var/root/.emu_probe",
      "status": "denied",
      "target_user": "mobile",
      "uid": 501,
      "expected_denial": true,
      "output": "Permission denied"
    },
    "observed_kernel_version": "Darwin Kernel Version 20.0.0: root:xnu-7195.0.0~1/RELEASE_ARM64_T8030",
    "observed_boot_args": "debug=0x144 amfi=0xff -v",
    "benign_binary_digest": "sha256:5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b",
    "verification_state": "verified",
    "verified_at": "2026-09-17T14:35:10.000Z",
    "diagnostics": [
      "Root bootstrap console attached via Virtio chardev",
      "Positive probe succeeded: file write confirmed in /private/var/root",
      "Negative control succeeded: write as mobile (UID 501) returned EACCES"
    ]
  },
  "error": null
}
```

### 3.2 Negative Control Falsification (Negative Test)

Simulate a negative control failure (e.g. guest filesystem misconfigured with permissive root directory permissions):

```bash
emu research root verify --id e4b1c2d3-4567-89ab-cdef-0123456789ab --simulate-permissive-guest --json
```

**Stdout (Exit Code 1 `failed`)**:

```json
{
  "status": "failed",
  "outcome": "execution_failed",
  "operation_id": "op_01J8ROOTFAIL01",
  "data": {
    "verification_state": "unverified",
    "failure_reason": "Negative control check failed: unprivileged user 'mobile' (UID 501) was unexpectedly permitted to write /private/var/root/.emu_probe",
    "negative_control_outcome": {
      "path": "/private/var/root/.emu_probe",
      "status": "success",
      "target_user": "mobile",
      "uid": 501,
      "expected_denial": true,
      "output": "fail"
    }
  },
  "error": {
    "code": "PROBE_VERIFICATION_FAILED",
    "message": "Falsification gate triggered: unprivileged user write succeeded",
    "details": {
      "remediation": "Guest root filesystem permissions are permissive; verification refused"
    }
  }
}
```

### 3.3 Root Proof Invalidation on Guest Reboot

Cold rebooting the guest invalidates active root proof until re-proven (FR-015):

```bash
emu research guest restart --id e4b1c2d3-4567-89ab-cdef-0123456789ab --json
emu research root status --id e4b1c2d3-4567-89ab-cdef-0123456789ab --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8ROOTSTAT01",
  "data": {
    "guest_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "desired_privilege": "root",
    "observed_privilege": "unverified",
    "invalidation_reason": "Guest rebooted; fresh boot session ID requires empirical re-verification"
  },
  "error": null
}
```

---

## 4. Scenario 3: Application Lifecycle & Minimal Backend Refusal Gates (User Story 3 - P1)

### 4.1 Application Framework Refusal Gate on Minimal Backend

Attempting to install an application on minimal `darwin-vm` truthfully reports missing application frameworks (SC-003, FR-023):

```bash
emu research app install \
  --id e4b1c2d3-4567-89ab-cdef-0123456789ab \
  --package ~/research_artifacts/SampleResearchApp.ipa \
  --json
```

**Stdout (Exit Code 3 `unsupported`)**:

```json
{
  "status": "rejected",
  "outcome": "unsupported",
  "operation_id": "op_01J8APPUNSUP01",
  "data": null,
  "error": {
    "code": "APP_FRAMEWORKS_UNAVAILABLE",
    "message": "Backend 'darwin-vm' does not support iOS application-layer frameworks",
    "details": {
      "backend": "darwin-vm",
      "app_frameworks_supported": false,
      "supported_capabilities": [
        "headless_console",
        "kernel_debugging",
        "root_cli_testing"
      ],
      "remediation": "Deploy iOS applications to the 'Inferno' research backend"
    }
  }
}
```

### 4.2 Application Import & Installation on `Inferno`

Boot the `Inferno` guest instance and deploy an owned iOS research application:

```bash
# Start Inferno guest
emu research guest start --id f5c2d3e4-5678-9abc-def0-123456789abc --json

# Import application package
emu research app import --package ~/research_artifacts/SampleResearchApp.ipa --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8APPIMPORT01",
  "data": {
    "app_id": "app_com_example_researchapp_01",
    "bundle_identifier": "com.example.researchapp",
    "bundle_name": "ResearchApp",
    "binary_architecture": "arm64",
    "code_signature_identity": "Apple Development: Lab Signer",
    "sha256_digest": "sha256:1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b",
    "deployment_status": "imported"
  },
  "error": null
}
```

```bash
# Install onto running Inferno guest via InstallationProxy / ideviceinstaller
emu research app install \
  --id f5c2d3e4-5678-9abc-def0-123456789abc \
  --app-id app_com_example_researchapp_01 \
  --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8APPINSTALL01",
  "data": {
    "app_id": "app_com_example_researchapp_01",
    "bundle_identifier": "com.example.researchapp",
    "deployment_status": "installed",
    "sandbox_container_path": "/private/var/mobile/Containers/Data/Application/3D5E7A2B-1C4F-4E89-B672-9876543210AB"
  },
  "error": null
}
```

---

## 5. Scenario 4: Frida Dynamic Instrumentation, Native/ObjC Hooks & Target Specificity (User Story 3 - P1)

### 5.1 Frida Agent Deployment & Startup

Deploy the pinned Frida 17.18.0 agent to the `Inferno` guest:

```bash
emu research frida prepare --id f5c2d3e4-5678-9abc-def0-123456789abc --json
emu research frida start --id f5c2d3e4-5678-9abc-def0-123456789abc --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8FRIDASTART01",
  "data": {
    "guest_id": "f5c2d3e4-5678-9abc-def0-123456789abc",
    "agent_package_version": "17.18.0",
    "bridge_endpoint": "127.0.0.1:27042",
    "status": "ready"
  },
  "error": null
}
```

### 5.2 Dynamic Script Injection Hooking Native C & Objective-C

Spawn the target application, inject an instrumentation script hooking native `open` and Objective-C `NSURLSession`, and monitor an uninstrumented control process:

```bash
cat << 'EOF' > /tmp/research_hooks.js
Interceptor.attach(Module.getExportByName(null, 'open'), {
  onEnter: function (args) {
    send({ type: 'native', symbol: 'open', arg0: Memory.readUtf8String(args[0]) });
  }
});
if (ObjC.available) {
  var method = ObjC.classes.NSURLSession["- dataTaskWithRequest:"];
  Interceptor.attach(method.implementation, {
    onEnter: function (args) {
      send({ type: 'objc', class: 'NSURLSession', method: '- dataTaskWithRequest:' });
    }
  });
}
EOF

emu research frida attach \
  --id f5c2d3e4-5678-9abc-def0-123456789abc \
  --bundle-id com.example.researchapp \
  --script /tmp/research_hooks.js \
  --control-pid 510 \
  --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8FRIDAATTACH01",
  "data": {
    "session_id": "sess_01J8ATTACH001",
    "guest_id": "f5c2d3e4-5678-9abc-def0-123456789abc",
    "target_process_id": 412,
    "target_bundle_id": "com.example.researchapp",
    "attachment_state": "attached",
    "hook_status": {
      "native_hooks_count": 1,
      "objc_hooks_count": 1,
      "events_intercepted": 2
    },
    "control_process_id": 510,
    "control_process_hooked": false,
    "observed_events": [
      {
        "type": "native",
        "symbol": "open",
        "arg0": "/private/var/mobile/Containers/Data/Application/3D5E7A2B-1C4F-4E89-B672-9876543210AB/Documents/state.db"
      },
      {
        "type": "objc",
        "class": "NSURLSession",
        "method": "- dataTaskWithRequest:"
      }
    ]
  },
  "error": null
}
```

### 5.3 Clean Detachment

Detach probes verifying the target application continues running (SC-002):

```bash
emu research frida detach --session-id sess_01J8ATTACH001 --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8FRIDADETACH01",
  "data": {
    "session_id": "sess_01J8ATTACH001",
    "attachment_state": "detached_clean",
    "target_process_id": 412,
    "target_process_running": true
  },
  "error": null
}
```

---

## 6. Scenario 5: Deep Kernel Debugging & Disconnect Truthfulness (User Story 4 - P2)

### 6.1 Kernel Execution Pause & State Inspection

Pause the running `darwin-vm` guest and inspect ARM64 registers (SC-004, FR-027):

```bash
emu research debug pause --id e4b1c2d3-4567-89ab-cdef-0123456789ab --json
emu research debug registers --id e4b1c2d3-4567-89ab-cdef-0123456789ab --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8DEBUGREGS01",
  "data": {
    "guest_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "runstate": "paused",
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

### 6.2 Breakpoint, Single-Step & Test State Modification

Set a software breakpoint, execute a single instruction step, and edit register `x0`:

```bash
# 1. Single instruction step
emu research debug step --id e4b1c2d3-4567-89ab-cdef-0123456789ab --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8DEBUGSTEP01",
  "data": {
    "guest_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "step_result": "trap_hit",
    "pc": "0xfffffe0007812004",
    "instruction": "mov x0, #1"
  },
  "error": null
}
```

```bash
# 2. Modify test register x0
emu research debug registers --id e4b1c2d3-4567-89ab-cdef-0123456789ab --write x0=0x0000000000000042 --json
```

### 6.3 Truthful Debugger Disconnect Handling

Simulate unexpected debugger disconnection while the guest is paused (FR-029, SC-006):

```bash
emu research debug disconnect --id e4b1c2d3-4567-89ab-cdef-0123456789ab --action preserve-paused --json
```

**Stdout (Exit Code 0)**:

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

_The guest remains truthfully paused in virtual hardware; silent resumption is prohibited._

---

## 7. Scenario 6: Image Preparation & Disposable Cleanup (User Story 5 - P2)

### 7.1 Verified Device Node Mounting

Host-side image preparation verifies the exact device node (`/dev/diskNsM`) and Volume UUID via `diskutil info -plist` without hardcoded path assumptions (FR-033, SC-010):

```bash
emu research image prepare \
  --source ~/research_artifacts/darwin_root_ramdisk.img \
  --target-backend darwin-vm \
  --output /tmp/emu_prepared_disk.raw \
  --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8IMGPREP01",
  "data": {
    "target_backend": "darwin-vm",
    "verified_device_node": "/dev/disk4s1",
    "verified_volume_uuid": "B3A4C5D6-7E8F-9A0B-1C2D-3E4F5A6B7C8D",
    "output_artifact_digest": "sha256:7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a",
    "host_ssv_untouched": true,
    "host_sip_untouched": true
  },
  "error": null
}
```

### 7.2 Unattended Elevation Refusal Gate

If image preparation requires administrative elevation during unattended execution without pre-authorized credentials, it refuses immediately (FR-035):

```bash
emu research image prepare \
  --source ~/research_artifacts/darwin_root_ramdisk.img \
  --target-backend darwin-vm \
  --output /tmp/emu_privileged.raw \
  --unattended \
  --simulate-missing-sudo \
  --json
```

**Stdout (Exit Code 2 `invalid_input`)**:

```json
{
  "status": "rejected",
  "outcome": "invalid_input",
  "operation_id": "op_01J8AUTHFAIL01",
  "data": null,
  "error": {
    "code": "AUTH_REQUIRED",
    "message": "Administrative elevation required for disk image mounting but unattended automation mode was requested without pre-authorized credentials",
    "details": {
      "target_file": "/tmp/emu_privileged.raw",
      "remediation": "Provide pre-authorized credentials or run interactively"
    }
  }
}
```

---

## 8. Scenario 7: Companion VM Orchestration for Restore Dependencies (User Story 6 - P3)

### 8.1 Companion VM Startup & Dependency Binding

Launch the local helper Linux companion VM on Apple Silicon Mac for `Inferno` restore utilities (FR-037):

```bash
emu research companion start \
  --parent-guest-id f5c2d3e4-5678-9abc-def0-123456789abc \
  --cpus 2 \
  --memory-mb 2048 \
  --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8COMPSTART01",
  "data": {
    "companion_id": "comp_01J8COMPANION01",
    "parent_guest_id": "f5c2d3e4-5678-9abc-def0-123456789abc",
    "lifecycle_state": "running",
    "endpoint_socket_path": "/tmp/emu-f5c2d3e4/companion.sock",
    "forwarded_ports": [27042],
    "active_workflows_count": 1,
    "live_dependents_count": 1
  },
  "error": null
}
```

### 8.2 Teardown Refusal While Live Dependent Guest Sessions Remain

Attempting to stop the companion VM while the live `Inferno` guest session depends on companion forwarders is rejected (FR-038, SC-011):

```bash
emu research companion stop --parent-guest-id f5c2d3e4-5678-9abc-def0-123456789abc --json
```

**Stdout (Exit Code 5 `conflict`)**:

```json
{
  "status": "rejected",
  "outcome": "conflict",
  "operation_id": "op_01J8COMPSTOPERR01",
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

---

## 9. Scenario 8: Research Profiles, Baseline Recovery & Safe Cancellation (User Story 7 & 8 - P4 & P5)

### 9.1 Profile Export Without Host Credentials

Export a reproducible research profile; verify that host environment secrets, SSH keys, and credentials are automatically stripped (FR-041, SC-013):

```bash
emu research profile export \
  --id e4b1c2d3-4567-89ab-cdef-0123456789ab \
  --output /tmp/exported_profile.json \
  --json
```

**Stdout (Exit Code 0)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8PROFEXP01",
  "data": {
    "profile_id": "prof_01J8EXPORTED01",
    "target_backend": "darwin-vm",
    "base_image_digest": "sha256:d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456",
    "host_secrets_stripped": true,
    "exported_path": "/tmp/exported_profile.json"
  },
  "error": null
}
```

### 9.2 Two-Step Authorized Baseline Recovery

Restore a modified or corrupted guest to its verified `RecoveryBaseline` (FR-043, SC-012):

```bash
# 1. Dry run to inspect proposal and digest
emu research baseline restore --id e4b1c2d3-4567-89ab-cdef-0123456789ab --dry-run --json
```

**Stdout (Exit Code 0, proposal created)**:

```json
{
  "status": "accepted",
  "outcome": "proposal_created",
  "operation_id": "op_01J8BASEPROP01",
  "data": {
    "proposal_id": "prop_01J8BASE001",
    "proposal_digest": "sha256:b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef01",
    "operation_type": "baseline_restore",
    "target_instance_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "backend": "darwin-vm",
    "affected_paths": [
      "/Users/researcher/Library/Application Support/emu/research/disks/darwin_vm_root_e4b1c2d3.raw"
    ],
    "destructive": true,
    "expires_at": "2026-09-17T14:50:00.000Z"
  },
  "error": null
}
```

```bash
# 2. Execute with authorization digest
emu research baseline restore \
  --id e4b1c2d3-4567-89ab-cdef-0123456789ab \
  --authorize sha256:b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef01 \
  --json
```

**Stdout (Exit Code 0, baseline restored)**:

```json
{
  "status": "success",
  "outcome": "completed",
  "operation_id": "op_01J8BASERESTORE01",
  "data": {
    "guest_id": "e4b1c2d3-4567-89ab-cdef-0123456789ab",
    "baseline_id": "base_e4b1c2d3_stock",
    "restoration_time_ms": 3420,
    "declared_sla_ms": 15000,
    "sla_satisfied": true
  },
  "error": null
}
```

---

## 10. Scenario 9: Safe Cancellation Boundaries & Bounded Caller Wait Timeouts (User Story 8 - P5)

### 10.1 Bounded Caller Wait Timeout

When a caller-specified wait deadline expires before a background operation finishes, the system accurately reports continuing state without terminating the guest task (FR-046, SC-016):

```bash
emu research operation wait --id op_01J8LONGRESTORE01 --timeout 1 --json
```

**Stdout (Exit Code 124 `timed_out`)**:

```json
{
  "status": "timed_out",
  "outcome": "timeout",
  "operation_id": "op_01J8LONGRESTORE01",
  "data": {
    "operation_id": "op_01J8LONGRESTORE01",
    "execution_state": "continuing",
    "current_phase": "executing",
    "progress_percent": 45,
    "elapsed_seconds": 1.05
  },
  "error": {
    "code": "TIMEOUT",
    "message": "Caller wait deadline elapsed; background task continues running",
    "details": {
      "remediation": "Re-query with 'emu research operation status --id op_01J8LONGRESTORE01' or increase --timeout"
    }
  }
}
```

### 10.2 Explicit Cancellation at Safe Boundary

Request immediate cancellation. The system acknowledges within <= 200ms with `"cancellation_pending"` and confirms cessation only after reaching a verified safe transaction boundary with disposable cleanup (FR-046, SC-015):

```bash
emu research operation cancel --id op_01J8LONGRESTORE01 --json
```

**Stdout (Exit Code 130 `cancelled`)**:

```json
{
  "status": "cancelled",
  "outcome": "cancelled",
  "operation_id": "op_01J8LONGRESTORE01",
  "data": {
    "operation_id": "op_01J8LONGRESTORE01",
    "safe_boundary_reached": true,
    "cleaned_resources": [
      "/tmp/emu_scratch_restore.tmp",
      "/tmp/emu-f5c2d3e4/companion.sock"
    ],
    "residual_state": "clean_stopped"
  },
  "error": null
}
```

---

## 11. Acceptance Verification Matrix

| Scenario Number & Name                                     | Validated Success Criteria | Relevant Functional Requirements       | Primary Observable Oracles                                 | Outcome Exit Code |
| :--------------------------------------------------------- | :------------------------- | :------------------------------------- | :--------------------------------------------------------- | :---------------: |
| **Scenario 1**: Host Preflight & Dual-Backend Registration | SC-008, SC-014, SC-017     | FR-001, FR-002, FR-005, FR-007         | Preflight JSON; UUIDv4 assignment; name collision rejected |   **0** / **2**   |
| **Scenario 2**: iOS Root Proof & Falsification             | SC-001, SC-007             | FR-010, FR-012, FR-013, FR-014, FR-015 | UID 0 pass, UID 501 deny; reboot invalidates               |   **0** / **1**   |
| **Scenario 3**: App Lifecycle & Framework Refusal          | SC-002, SC-003             | FR-016, FR-022, FR-023, FR-024         | Refused on darwin-vm; installed on Inferno                 |   **0** / **3**   |
| **Scenario 4**: Frida Hooks & Specificity                  | SC-002, SC-005             | FR-017, FR-018, FR-020, FR-021         | Native + ObjC hooks; control untouched                     |       **0**       |
| **Scenario 5**: Kernel Debugging & Disconnect              | SC-004, SC-006             | FR-027, FR-028, FR-029                 | Pause/registers/step; truthful paused on disconnect        |       **0**       |
| **Scenario 6**: Image Prep & Mount Identity                | SC-009, SC-010             | FR-031, FR-033, FR-034, FR-035, FR-036 | Verified disk node/UUID; unattended elevation fail         |   **0** / **2**   |
| **Scenario 7**: Companion VM Orchestration                 | SC-011                     | FR-037, FR-038, FR-039                 | Local Linux helper; stop refused with dependents           |   **0** / **5**   |
| **Scenario 8**: Profiles & Baseline Recovery               | SC-012, SC-013             | FR-040, FR-041, FR-042, FR-043         | Host secrets stripped; two-step auth restore               |   **0** / **4**   |
| **Scenario 9**: Bounded Timeout & Safe Cancel              | SC-015, SC-016             | FR-044, FR-045, FR-046, FR-048         | Timeout code 124; cancel code 130 at safe boundary         | **124** / **130** |
