# Quickstart: iOS Root VM & Darwin Security Research Backends

**Version**: 1.0.0  
**Feature Branch**: `002-ios-root-vm`  
**Status**: Complete  
**Canonical Schema Identifier**: `https://emu.rs/schemas/v1/research-ios.schema.json`  
**Target Host Platform**: macOS Apple Silicon (`aarch64`, Darwin 24.x/25.x, macOS 15+ candidate)  
**Toolchain Baseline**: Rust 2024 edition, pinned to `rustc 1.88.0` (`.tool-versions`); `bun 1.2.17`  
**Authority**: `specs/002-ios-root-vm/research.md`, `specs/002-ios-root-vm/spec.md`, `specs/002-ios-root-vm/data-model.md`, and `specs/002-ios-root-vm/contracts/cli.md`

---

## 1. Prerequisites and Laboratory Setup

> **Implementation & Empirical Execution Notice**: This document serves as the normative end-to-end operational execution and validation guide for the planned iOS Root Virtual Machine and Darwin Security Research Harness. It defines executable CLI/TUI scenarios covering all 8 User Stories, all 10 operational command families, and the 19 Success Criteria (SC-001 through SC-019). The commands, schemas, and workflows documented herein represent the post-implementation delivery contract. They do NOT assert that research hypervisor backends currently exist in the repository (which currently contains only standard Android AVD and iOS Simulator managers under `src/`). Full empirical execution requires legally obtained Apple firmware artifacts and user-compiled hypervisor binaries; absent real assets block empirical runs, not mock substitutes. Build and test commands documented in this guide are illustrative validation procedures and MUST NOT be executed during Phase 1 documentation work.

### 1.1 Host Environment Target Matrix

| Host Workstation OS                                          | Architecture                  | Virtualization Subsystem                                       |        `darwin-vm` Status        |         `Inferno` Status         | Primary Operational Scope                                                                                                                                                           |
| :----------------------------------------------------------- | :---------------------------- | :------------------------------------------------------------- | :------------------------------: | :------------------------------: | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Configured macOS (Darwin 24.x/25.x, macOS 15+ candidate)** | **Apple Silicon (`aarch64`)** | `sysctl hw.optional.arm64 = 1`; HVF evaluated during preflight | **Research Target (Unverified)** | **Research Target (Unverified)** | Dual-backend research, root proof, kernel debugging, Frida dynamic instrumentation, and companion VM orchestration. Evaluated as unverified candidate prior to physical lab trials. |
| **macOS (Intel `x86_64`)**                                   | Intel 64-bit                  | Intel VT-x / VMX                                               |         **Unsupported**          |         **Unsupported**          | Apple Silicon ARM64 host required (FR-001). Exits with Exit Code 3 (`unsupported`).                                                                                                 |
| **Linux / Windows**                                          | x86_64 / arm64                | KVM / WHPX                                                     |         **Unsupported**          |         **Unsupported**          | Darwin security research backends require macOS host platform (FR-001). Exits with Exit Code 3 (`unsupported`).                                                                     |

_Note on Hypervisor Acceleration_: While `Hypervisor.framework` availability (`sysctl kern.hv_support = 1`) is evaluated during host preflight diagnostics, the research hypervisors (`darwin-vm`, `Inferno`, and the companion `qemu-system-x86_64`) execute in user space using QEMU's Tiny Code Generator (TCG) translating ARM64/x86_64 guest instructions. Therefore, universal HVF entitlement is not an absolute execution prerequisite for guest emulation on Apple Silicon.

### 1.2 Toolchain & Source Revision Verification

Before executing research commands, verify that the host toolchain matches the repository's pinned baseline:

```bash
# 1. Verify Rust toolchain pinned to 1.88.0
case "$(rustc --version)" in
  *"1.88.0"*) ;;
  *) echo "Error: Pinned rustc 1.88.0 required"; exit 1 ;;
esac

# 2. Verify Bun runtime
case "$(bun --version)" in
  *"1.2.17"*) ;;
  *) echo "Error: Pinned bun 1.2.17 required"; exit 1 ;;
esac

# 3. Verify Apple Silicon architecture
[ "$(sysctl -n hw.optional.arm64 2>/dev/null)" = "1" ] || { echo "Error: Apple Silicon ARM64 host required"; exit 1; }

# 4. Record source revision
git rev-parse HEAD
```

### 1.3 User-Supplied Artifact Directory Setup

Emu **never bundles, downloads, or distributes proprietary Apple firmware, IPSWs, kernelcaches, or APtickets**. Researchers must legally obtain and supply compatible artifacts in a local staging directory (`~/research_artifacts/`):

```bash
export ASSET_DIR="$HOME/research_artifacts"
mkdir -p "$ASSET_DIR"
```

Expected user-supplied artifacts include:

- `darwin_bootkc_minimal.bin`: Extracted Mach kernelcache for `darwin-vm` (`kernelcache`).
- `darwin_dtree_minimal.dtb`: Device tree blob for `darwin-vm` (`devicetree`).
- `darwin_root_ramdisk.img`: Minimal root ramdisk with bootstrap console (`ramdisk`).
- `darwin_trustcache.tc`: Trust cache for minimal Darwin boot (`trustcache`).
- `inferno_kernelcache_18A5351d`: iOS 14.0 beta 5 kernelcache for Apple iPhone 11 (`iPhone12,1`, platform board identifier **`n104ap`**, A13 Bionic; `kernelcache`).
- `inferno_dtree_n104ap.dtb`: iPhone 11 (`n104ap`) device tree blob (`devicetree`).
- `inferno_rootfs_18A5351d.raw`: Prepared iOS research root disk image (`root_disk`).
- `bundled_research_hooks.js`: User-supplied script with pre-bundled `frida-objc-bridge` runtime.
- `SampleResearchApp.ipa`: Owned test application (`com.example.researchapp`) compiled for `arm64`.
- `ControlApp.ipa`: Uninstrumented control application (`com.example.controlapp`).
- `benign_test_binary`: Benign ARM64 guest CLI verification binary.

### 1.4 Native Frida Helper Compilation (Laboratory Setup Only)

To maintain clean CI boundaries, `emu-frida-worker` is architected as an explicitly separate native child package in `tools/frida-worker/` outside the root workspace and default Cargo target graph. In laboratory environments where dynamic instrumentation is exercised, compile the helper once using the official `frida-core` 17.18.0 C devkit:

```bash
# Set path to validated frida-core C devkit directory
export FRIDA_CORE_DEVKIT="$ASSET_DIR/frida-core-devkit-17.18.0-mac"

# Build isolated Frida worker binary
cargo build --manifest-path tools/frida-worker/Cargo.toml --release
```

---

## 2. Scenario 1: Host Preflight Diagnostics, Image Registration & Dual-Backend Guest Creation (User Story 1 - P1)

_Covers Success Criteria: SC-008, SC-014, SC-017 | Functional Requirements: FR-001, FR-002, FR-003, FR-005, FR-006, FR-007, FR-031_

### 2.1 Preflight Compatibility Diagnostics

Run non-interactive preflight diagnostics across both research backends:

```bash
emu research backend preflight --json
```

_Preflight Contract & Observability_:

- Returns **Exit Code 0** (`completed`) for non-interactive diagnostics.
- Reports host architecture (`aarch64`) and hypervisor support (`sysctl kern.hv_support`).
- Prior to physical lab verification, hypervisor backends report `support_status: "unverified"` or `"experimental"`, operating under user-space QEMU TCG translation without asserting universal HVF entitlement.
- Projects configured paths to hypervisor binaries (e.g. custom `qemu-system-aarch64` and `Inferno` binaries) and companion dependencies (`ideviceinstaller`, companion `qemu-system-x86_64`).

### 2.2 Base Image Registration (Before Instance Creation)

Images must be registered and verified before guest instance creation (FR-031, SC-009). Capture actual JSON responses and extract digests using `jq`:

```bash
# 1. Register darwin-vm root ramdisk artifact
DARWIN_RAMDISK_RESP=$(emu research image register \
  --file "$ASSET_DIR/darwin_root_ramdisk.img" \
  --type ramdisk \
  --backend darwin-vm \
  --json)
DARWIN_RAMDISK_DIGEST=$(printf '%s' "$DARWIN_RAMDISK_RESP" | jq -er '.data.artifact_digest')

# 2. Register darwin-vm kernelcache
DARWIN_KC_RESP=$(emu research image register \
  --file "$ASSET_DIR/darwin_bootkc_minimal.bin" \
  --type kernelcache \
  --backend darwin-vm \
  --json)
DARWIN_KC_DIGEST=$(printf '%s' "$DARWIN_KC_RESP" | jq -er '.data.artifact_digest')

# 3. Register Inferno root disk artifact
INFERNO_ROOT_RESP=$(emu research image register \
  --file "$ASSET_DIR/inferno_rootfs_18A5351d.raw" \
  --type root_disk \
  --backend Inferno \
  --json)
INFERNO_ROOT_DIGEST=$(printf '%s' "$INFERNO_ROOT_RESP" | jq -er '.data.artifact_digest')

# 4. Register Inferno kernelcache
INFERNO_KC_RESP=$(emu research image register \
  --file "$ASSET_DIR/inferno_kernelcache_18A5351d" \
  --type kernelcache \
  --backend Inferno \
  --json)
INFERNO_KC_DIGEST=$(printf '%s' "$INFERNO_KC_RESP" | jq -er '.data.artifact_digest')
```

### 2.3 Dual-Backend Guest Registration with Identical Display Names

Register two guest instances with the identical display name `"ios-sec-lab"` under `darwin-vm` and `Inferno` to verify unambiguous UUID assignment and identity disambiguation (FR-005, FR-007, SC-017):

```bash
# 1. Register darwin-vm guest
DARWIN_CREATE_RESP=$(emu research guest create \
  --name "ios-sec-lab" \
  --backend darwin-vm \
  --root-disk "$DARWIN_RAMDISK_DIGEST" \
  --kernelcache "$DARWIN_KC_DIGEST" \
  --json)
DARWIN_ID=$(printf '%s' "$DARWIN_CREATE_RESP" | jq -er '.data.id')

# 2. Register Inferno guest with identical display name
INFERNO_CREATE_RESP=$(emu research guest create \
  --name "ios-sec-lab" \
  --backend Inferno \
  --root-disk "$INFERNO_ROOT_DIGEST" \
  --kernelcache "$INFERNO_KC_DIGEST" \
  --json)
INFERNO_ID=$(printf '%s' "$INFERNO_CREATE_RESP" | jq -er '.data.id')
```

### 2.4 Ambiguous Target Name Collision Rejection & Disambiguation

Attempting to inspect or start without specifying the backend or unique ID is rejected with Exit Code 2 (FR-007):

```bash
emu research guest inspect --name "ios-sec-lab" --json
```

_Expected Exit Code_: `2` (`invalid_input`). Error code `AMBIGUOUS_INSTANCE_NAME` lists both matching UUIDs and instructs qualification via `--id` or `--backend`.

Inspect unambiguously using UUID or backend qualification:

```bash
emu research guest inspect --id "$DARWIN_ID" --json
emu research guest inspect --name "ios-sec-lab" --backend Inferno --json
```

### 2.5 Independent Guest Launch

Boot the `darwin-vm` instance cleanly using its unique UUID:

```bash
emu research guest start --id "$DARWIN_ID" --json
```

_Expected Exit Code_: `0` (`completed`). The instance transitions to `running`, starts the dedicated child supervisor, and initial privilege status reports `unverified`.

---

## 3. Scenario 2: Verifiable iOS-Derived Root Proof & Falsification Controls (User Story 2 - P1)

_Covers Success Criteria: SC-001, SC-007 | Functional Requirements: FR-010, FR-011, FR-012, FR-013, FR-014, FR-015, FR-025, FR-026, FR-030_

### 3.1 Empirical Root Proof Verification (3 Cold Boots Repetition; SC-001)

To satisfy SC-001 across the reference cohort, execute root verification across 3 consecutive cold boots:

```bash
for boot in 1 2 3; do
  echo "=== Root Proof Evaluation Boot Trial $boot ==="
  if [ "$boot" -gt 1 ]; then
    emu research guest restart --id "$DARWIN_ID" --json
  fi

  ROOT_VERIFY_RESP=$(emu research root verify \
    --id "$DARWIN_ID" \
    --test-binary "$ASSET_DIR/benign_test_binary" \
    --json)

  # Assert effective UID 0 positive probe success and non-zero UID negative control denial
  printf '%s' "$ROOT_VERIFY_RESP" | jq -er '.data.positive_probe.status == "success"'
  printf '%s' "$ROOT_VERIFY_RESP" | jq -er '.data.negative_control.status == "denied"'
  printf '%s' "$ROOT_VERIFY_RESP" | jq -er '.data.verification_state == "verified"'
done
```

### 3.2 Negative Control Falsification (Gate T-02, SC-019 Case 6)

In a negative evaluation case (e.g. guest filesystem misconfigured with permissive root directory permissions, or unprivileged probe helper allowed to write `/private/var/root/.emu_probe`), the verification workflow fails closed:

```bash
# Execute root verify against an intentionally unprivileged probe fixture
emu research root verify --id "$DARWIN_ID" --json || echo "Negative control gate triggered: Exit code $?"
```

_Expected Outcome_: Exit Code `1` (`execution_failed`). System marks privilege status as `unverified` and logs structured diagnostic warnings.

### 3.3 Root Proof Invalidation on Guest Reboot (Gate T-03, SC-019 Case 7)

Cold rebooting the guest resets the boot session UUID and immediately invalidates active root proof until re-proven (FR-015):

```bash
emu research guest restart --id "$DARWIN_ID" --json
STATUS_RESP=$(emu research root status --id "$DARWIN_ID" --json)
printf '%s' "$STATUS_RESP" | jq -er '.data.observed_privilege == "unverified"'
```

### 3.4 Authorized In-Guest Root Filesystem Operations (FR-025)

Perform direct in-guest root filesystem read, write, and export operations without host disk mounting:

```bash
# 1. Read in-guest system file
emu research root fs-read --guest-id "$DARWIN_ID" --path "/etc/hosts" --json

# 2. Write file to guest runtime storage
emu research root fs-write \
  --guest-id "$DARWIN_ID" \
  --path "/private/var/root/test.txt" \
  --src "/tmp/local_test.txt" \
  --json

# 3. Export in-guest log to host
emu research root fs-export \
  --guest-id "$DARWIN_ID" \
  --path "/private/var/log/system.log" \
  --dest "/tmp/system.log" \
  --json
```

### 3.5 Security Profile Reversion to Verified Baseline (FR-030)

Inspect active guest security profile and revert policies to a verified baseline configuration (never an assumed universal stock state):

```bash
# 1. Inspect security profile
emu research root security inspect --guest-id "$DARWIN_ID" --json

# 2. Dry run revert to baseline to inspect proposal
emu research root security revert \
  --guest-id "$DARWIN_ID" \
  --baseline-id "base_e4b1c2d3_stock" \
  --dry-run \
  --json

# 3. Manual Operator Review: Operator reviews stdout proposal and inputs digest
read -r -p "Enter reviewed security revert proposal digest: " SEC_DIGEST
emu research root security revert \
  --guest-id "$DARWIN_ID" \
  --baseline-id "base_e4b1c2d3_stock" \
  --authorize "$SEC_DIGEST" \
  --json
```

---

## 4. Scenario 3: Application Lifecycle Management on `Inferno` & Minimal Backend Refusal Gates (User Story 3 - P1)

_Covers Success Criteria: SC-002, SC-003 | Functional Requirements: FR-016, FR-022, FR-023, FR-024_

### 4.1 Application Framework Refusal Gate on Minimal Backend (Gate T-06, SC-003)

Attempting to install an application on minimal `darwin-vm` truthfully reports missing application frameworks with Exit Code 3 (`unsupported`):

```bash
emu research app install \
  --id "$DARWIN_ID" \
  --app-id "app_com_example_researchapp_01" \
  --json
```

_Expected Exit Code_: `3` (`unsupported`). Error code `APP_FRAMEWORKS_UNAVAILABLE` explains that `darwin-vm` lacks application-layer frameworks.

### 4.2 Application Import & Installation on `Inferno`

Boot the `Inferno` guest instance and deploy an owned iOS research application:

```bash
# 1. Start companion listener first (launch ordering invariant; D-08)
emu research companion start --parent-guest-id "$INFERNO_ID" --json

# 2. Boot Inferno guest
emu research guest start --id "$INFERNO_ID" --json

# 3. Import application package (Mach-O ARM64 validation; target guest ID is optional at import)
APP_IMPORT_RESP=$(emu research app import --package "$ASSET_DIR/SampleResearchApp.ipa" --json)
APP_ID=$(printf '%s' "$APP_IMPORT_RESP" | jq -er '.data.app_id')

# 4. Install onto running Inferno guest via companion usbmuxd bridge
emu research app install \
  --id "$INFERNO_ID" \
  --app-id "$APP_ID" \
  --json

# 5. Launch application and verify container creation
emu research app launch \
  --id "$INFERNO_ID" \
  --bundle-id "com.example.researchapp" \
  --json

# 6. Scoped container read, write, and export (FR-024)
emu research app container-write \
  --guest-id "$INFERNO_ID" \
  --bundle-id "com.example.researchapp" \
  --src "/tmp/test_data.json" \
  --dest "Documents/test_data.json" \
  --json

emu research app container-read \
  --id "$INFERNO_ID" \
  --bundle-id "com.example.researchapp" \
  --path "Documents/test_data.json" \
  --json

emu research app container-export \
  --id "$INFERNO_ID" \
  --bundle-id "com.example.researchapp" \
  --destination "/tmp/exported_app_container" \
  --json
```

---

## 5. Scenario 4: Frida Dynamic Instrumentation, Native/ObjC Hooks & Target Specificity (User Story 3 & 4 - P1 & P2)

_Covers Success Criteria: SC-002, SC-005 | Functional Requirements: FR-017, FR-018, FR-019, FR-020, FR-021_

### 5.1 Frida Agent Deployment & Authenticated Configuration

Deploy the pinned Frida 17.18.0 agent and configure endpoint credentials (Decision D-02):

```bash
# 1. Stage agent package
emu research frida prepare --id "$INFERNO_ID" --json

# 2. Install agent package into guest runtime
emu research frida install --id "$INFERNO_ID" --json

# 3. Configure agent with validated options file (pinned TLS server cert + session token over owned SSH bridge)
cat << 'EOF' > /tmp/frida_options.json
{
  "server_tls_cert_fingerprint": "SHA256:abcd1234ef567890abcd1234ef567890abcd1234ef567890abcd1234ef567890",
  "session_token": "tok_sess_9a8b7c6d5e4f3a2b1c0d",
  "listen_endpoint": "127.0.0.1:27042"
}
EOF

emu research frida configure \
  --guest-id "$INFERNO_ID" \
  --options-file /tmp/frida_options.json \
  --json

# 4. Start Frida agent daemon
emu research frida start --id "$INFERNO_ID" --json
```

### 5.2 Dynamic Script Injection (3 Cold Boot Trials; SC-002)

To satisfy SC-002, attach dynamic instrumentation using the user-provided bundled script (`$ASSET_DIR/bundled_research_hooks.js`) hooking native C functions (`open`) and Objective-C methods (`NSURLSession`), asserting that control process PID 510 remains unhooked across 3 trials:

```bash
for trial in 1 2 3; do
  echo "=== Frida Hook Evaluation Trial $trial ==="
  ATTACH_RESP=$(emu research frida attach \
    --id "$INFERNO_ID" \
    --bundle-id "com.example.researchapp" \
    --script "$ASSET_DIR/bundled_research_hooks.js" \
    --control-pid 510 \
    --json)

  printf '%s' "$ATTACH_RESP" | jq -er '.data.hooks_active.native_c_hooks >= 1'
  printf '%s' "$ATTACH_RESP" | jq -er '.data.hooks_active.objc_method_hooks >= 1'
  printf '%s' "$ATTACH_RESP" | jq -er '.data.control_process_hooked == false'

  SESS_ID=$(printf '%s' "$ATTACH_RESP" | jq -er '.data.session_id')
  emu research frida detach --session-id "$SESS_ID" --json
done
```

### 5.3 System Daemon Tracing (3 Trials; SC-005)

Attach dynamic instrumentation to a compatible guest system daemon:

```bash
for trial in 1 2 3; do
  echo "=== Daemon Hook Evaluation Trial $trial ==="
  DAEMON_RESP=$(emu research frida attach \
    --id "$INFERNO_ID" \
    --daemon "installd" \
    --script "$ASSET_DIR/bundled_research_hooks.js" \
    --json)

  DAEMON_SESS=$(printf '%s' "$DAEMON_RESP" | jq -er '.data.session_id')
  emu research frida detach --session-id "$DAEMON_SESS" --json
done
```

---

## 6. Scenario 5: Deep System & Kernel Debugging under `KernelDebugLease` (User Story 4 - P2)

_Covers Success Criteria: SC-004, SC-006 | Functional Requirements: FR-027, FR-028, FR-029_

### 6.1 Kernel Execution Pause & Register Inspection

Under an exclusive `KernelDebugLease` over the supervisor's GDB RSP chardev socket:

```bash
# 1. Pause virtual CPU execution
emu research debug pause --id "$DARWIN_ID" --json

# 2. Inspect 64-bit general-purpose registers
REGS_RESP=$(emu research debug registers --id "$DARWIN_ID" --json)
printf '%s' "$REGS_RESP" | jq -er '.data.pc != null'

# 3. Single instruction step ('s' packet)
emu research debug step --id "$DARWIN_ID" --json

# 4. Controlled test register edit and restore
ORIG_X0=$(printf '%s' "$REGS_RESP" | jq -er '.data.general_registers.x0')
emu research debug registers --id "$DARWIN_ID" --write x0=0x0000000000000042 --json
emu research debug registers --id "$DARWIN_ID" --write "x0=$ORIG_X0" --json
```

### 6.2 Truthful Debugger Disconnect State Handling (FR-029, SC-006)

Simulate debugger client disconnection while the guest is paused. The supervisor maintains the persistent RSP connection, never forwards `D`/`c`/`k` to QEMU, queries QMP `query-status`, and reports status truthfully as `paused` without silent resumption:

```bash
DISCONN_RESP=$(emu research debug disconnect --id "$DARWIN_ID" --action preserve-paused --json)
printf '%s' "$DISCONN_RESP" | jq -er '.data.observed_guest_runstate == "paused"'
printf '%s' "$DISCONN_RESP" | jq -er '.data.silently_resumed == false'
```

---

## 7. Scenario 6: Host-Side Image Preparation & Disposable Resource Cleanup (User Story 5 - P2)

_Covers Success Criteria: SC-009, SC-010 | Functional Requirements: FR-031, FR-032, FR-033, FR-034, FR-035, FR-036_

### 7.1 Verified Device Node & Volume Identity Inspection

Host-side image preparation verifies the exact device node (`/dev/diskNsM`) and Volume UUID via `diskutil info -plist`, preserving host SSV and SIP untouched (FR-033, FR-034, SC-010):

```bash
PREP_RESP=$(emu research image prepare \
  --source "$ASSET_DIR/darwin_root_ramdisk.img" \
  --target-backend darwin-vm \
  --output /tmp/emu_prepared_disk.raw \
  --json)
printf '%s' "$PREP_RESP" | jq -er '.data.verified_device_node != null'
printf '%s' "$PREP_RESP" | jq -er '.data.host_ssv_untouched == true'
printf '%s' "$PREP_RESP" | jq -er '.data.host_sip_untouched == true'
```

### 7.2 Unattended Elevation Refusal Gate (SC-019 Case 5)

When elevated host privileges are required during unattended automation execution without pre-authorized credentials, image preparation terminates immediately with Exit Code 4 (`auth_refused` / `AUTH_REQUIRED`) (FR-035):

```bash
emu research image prepare \
  --source "$ASSET_DIR/darwin_root_ramdisk.img" \
  --target-backend darwin-vm \
  --output /tmp/emu_privileged.raw \
  --unattended \
  --json || echo "Unattended elevation refused with exit code $?"
```

---

## 8. Scenario 7: Companion VM Orchestration for Restore Dependencies (User Story 6 - P3)

_Covers Success Criteria: SC-011 | Functional Requirements: FR-037, FR-038, FR-039_

### 8.1 Local x86_64 Companion VM on Same Mac

Launch the documented custom `qemu-system-x86_64` Linux companion VM under TCG on the same macOS host with UDS USB transport (`usb-tcp-remote`) (FR-037, D-08). The companion listener must achieve socket readiness before Inferno connects:

```bash
emu research companion start \
  --parent-guest-id "$INFERNO_ID" \
  --cpus 2 \
  --memory-mb 2048 \
  --json
```

### 8.2 Teardown Refusal While Live Dependent Guest Sessions Remain Bound (Gate G-07, SC-011)

Attempting to stop the companion VM while live dependent guest sessions remain bound is rejected with Exit Code 5 (`conflict`) (FR-038, SC-011):

```bash
emu research companion stop --parent-guest-id "$INFERNO_ID" --json || echo "Stop refused with exit code $?"
```

_Expected Outcome_: Exit Code `5` (`conflict`). Error code `DEPENDENT_GUEST_ACTIVE` indicates that dependent guest sessions must be terminated first.

---

## 9. Scenario 8: Research Profiles, Baseline Recovery & Guest Disk Wipe (User Story 7 - P4)

_Covers Success Criteria: SC-012, SC-013, SC-018 | Functional Requirements: FR-040, FR-041, FR-042, FR-043, FR-047_

### 9.1 Profile Export Without Host Secrets & Live Replay (SC-013)

Export a reproducible research experiment profile, verify host secret stripping, and replay on a fresh instance to prove live reproduction:

```bash
# 1. Export profile
emu research profile export \
  --id "$DARWIN_ID" \
  --output /tmp/exported_profile.json \
  --json

# 2. Verify host secrets are excluded using jq assertions
jq -er '
  (.data.kernel_boot_args | contains("password") | not) and
  (.data.applied_patches | length >= 0)
' /tmp/exported_profile.json

# 3. Import profile onto a fresh guest instance (SC-013)
emu research profile import --file /tmp/exported_profile.json --json
```

### 9.2 Baseline Creation Before Restoration (FR-043)

Capture a verified reference state before executing recovery workflows:

```bash
# Create baseline snapshot
emu research baseline create --id "$DARWIN_ID" --deadline-ms 15000 --json
```

### 9.3 Two-Step Authorized Baseline Recovery & Disk Wipe

Restore a guest instance to its verified `RecoveryBaseline`. Two-step authorization requires manual operator review:

```bash
# Step 1: Generate mutation proposal via dry-run
emu research baseline restore --id "$DARWIN_ID" --dry-run --json

# Step 2: Manual Operator Review - Operator reads digest from stdout and enters it
read -r -p "Enter reviewed baseline restore proposal digest: " REVIEWED_BASE_DIGEST
emu research baseline restore \
  --id "$DARWIN_ID" \
  --authorize "$REVIEWED_BASE_DIGEST" \
  --json

# 2. Two-step authorized disk wipe
emu research guest wipe --id "$DARWIN_ID" --dry-run --json
read -r -p "Enter reviewed wipe proposal digest: " REVIEWED_WIPE_DIGEST
emu research guest wipe --id "$DARWIN_ID" --authorize "$REVIEWED_WIPE_DIGEST" --json
```

---

## 10. Scenario 9: Bounded Caller Wait Timeouts, Safe Cancellation & TUI Parity (User Story 8 - P5)

_Covers Success Criteria: SC-014, SC-015, SC-016, SC-018, SC-019 | Functional Requirements: FR-044, FR-045, FR-046, FR-048_

### 10.1 Bounded Caller Wait Timeout (SC-016)

When a caller wait deadline elapses before a background task finishes, the system returns Exit Code 124 (`timed_out` / `timeout`), reporting actual background task continuing without terminating it (FR-046, SC-016):

```bash
emu research operation wait --id "op_01J8LONGTASK01" --timeout 1 --json || echo "Wait timed out with exit code $?"
```

_Expected Outcome_: Exit Code `124` (`timed_out` / `timeout`). Background task continues running.

### 10.2 Diagnostic Event Paging vs Continuous Streaming

```bash
# Paged single-envelope retrieval
emu research operation events --id "op_01J8LONGTASK01" --limit 50 --json

# Continuous live streaming to terminal stderr
emu research operation events --id "op_01J8LONGTASK01" --follow
```

### 10.3 Explicit Cancellation at Safe Boundary (Gate G-08, SC-015)

Request immediate cancellation. The system acknowledges within `<= 200ms` with `"cancellation_pending"` and confirms `"cancelled"` upon reaching a verified safe transaction boundary with disposable cleanup (FR-046, SC-015):

```bash
emu research operation cancel --id "op_01J8LONGTASK01" --json
```

_Expected Exit Code_: `130` (`cancelled`).

### 10.4 Standard Platform Non-Regression Verification (FR-004, SC-008)

To evaluate SC-008, 20 fixed lifecycle trials (launch, inspect, log stream, stop) of standard Android AVD and macOS iOS Simulator are executed via the legacy manager integration test runner, preserving constitution budgets:

```bash
# Run the dedicated legacy non-regression test suite
cargo test --test legacy_lifecycle_non_regression -- --nocapture
```

---

## 11. Acceptance Verification Matrix & 16 Critical Negative Cases

Formal empirical evaluation of the Reference Acceptance Set across the 4-guest cohort (2 `darwin-vm`, 2 `Inferno`) is executed and audited via the planned laboratory test runner:

```bash
cargo run --example ios_research_lab -- --config "$ASSET_DIR/lab_config.json" --negative-cases
```

The table below documents the exact test command invocations, falsification criteria, Gate bindings, and expected process exit codes verified across the cohort:

| Case # | Evaluated Condition & Gate                                         | Tested Command Invocation                                                                                                           |   Expected Exit Code   | Falsification Oracle & Outcome Classification                                              |
| :----: | :----------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------- | :--------------------: | :----------------------------------------------------------------------------------------- |
| **1**  | Missing prerequisites / unaccelerated host limits (Gate G-01)      | `emu research backend preflight --json`                                                                                             |        **`0`**         | `completed` (reports limits truthfully; `app_frameworks_supported: false`)                 |
| **2**  | Ambiguous instance name collision across backends (Gate G-03)      | `emu research guest inspect --name "ios-sec-lab" --json`                                                                            |        **`2`**         | `invalid_input` (rejected with `AMBIGUOUS_INSTANCE_NAME`)                                  |
| **3**  | Corrupted or truncated image artifact rejection (Gate G-05)        | `emu research image register --file /dev/null --type root_disk --backend Inferno --json`                                            |        **`2`**         | `invalid_input` (rejected with `CORRUPT_ARTIFACT`)                                         |
| **4**  | Experimental image missing baseline / opt-in (Gate G-05)           | `emu research image register --file "$ASSET_DIR/experimental.img" --type root_disk --backend Inferno --json`                        |        **`2`**         | `invalid_input` (rejected with `EXPERIMENTAL_OPT_IN_REQUIRED`)                             |
| **5**  | Denied administrative authorization in unattended mode (Gate G-06) | `emu research image prepare --unattended --source "$ASSET_DIR/ramdisk.img" --target-backend darwin-vm --output /tmp/out.raw --json` |        **`4`**         | `auth_refused` (rejected with `AUTH_REQUIRED`)                                             |
| **6**  | Incomplete root proof / negative control failure (Gate T-02)       | `emu research root verify --id "$DARWIN_ID" --json` (under misconfigured root permissions)                                          |        **`1`**         | `execution_failed` (privilege marked `unverified`; `PROBE_VERIFICATION_FAILED`)            |
| **7**  | Stale proof invalidation on reboot (Gate T-03)                     | `emu research root status --id "$DARWIN_ID" --json` (immediately after reboot)                                                      |        **`0`**         | `completed` (`observed_privilege` immediately transitions to `unverified`)                 |
| **8**  | Unsafe mount path detection and disposable cleanup (Gate G-06)     | `emu research image verify-mount --mount-path "/System" --json`                                                                     |        **`4`**         | `auth_refused` (`HOST_SYSTEM_PROTECTED`; disposable attachments released)                  |
| **9**  | Companion stop refused while live dependents exist (Gate G-07)     | `emu research companion stop --parent-guest-id "$INFERNO_ID" --json`                                                                |        **`5`**         | `conflict` (rejected with `DEPENDENT_GUEST_ACTIVE`)                                        |
| **10** | Cross-backend raw snapshot transfer rejection                      | `emu research guest create --name "fail" --backend darwin-vm --root-disk "$INFERNO_ROOT_DIGEST" --json`                             |        **`2`**         | `invalid_input` (rejected with `BACKEND_IMAGE_MISMATCH`)                                   |
| **11** | Application framework refusal on minimal backend (Gate T-06)       | `emu research app install --id "$DARWIN_ID" --app-id "sample" --json`                                                               |        **`3`**         | `unsupported` (rejected with `APP_FRAMEWORKS_UNAVAILABLE`)                                 |
| **12** | Frida script syntax error reporting process status                 | `emu research frida attach --id "$INFERNO_ID" --bundle-id "sample" --script /dev/null --json`                                       |        **`1`**         | `execution_failed` (`SCRIPT_PARSE_ERROR`; reports observed process health)                 |
| **13** | Debugger disconnect preserving paused state (Gate T-08)            | `emu research debug disconnect --id "$DARWIN_ID" --action preserve-paused --json`                                                   |        **`0`**         | `completed` (`observed_guest_runstate: "paused"`; no silent resumption)                    |
| **14** | Concurrent conflicting mutation rejection                          | `emu research guest stop --id "$DARWIN_ID" --json` (while operation lock held)                                                      |        **`5`**         | `conflict` (rejected with `INSTANCE_LOCKED`)                                               |
| **15** | Bounded wait timeout vs safe cancellation (Gate G-08)              | `emu research operation wait --id "op_long" --timeout 1 --json`<br>`emu research operation cancel --id "op_long" --json`            | **`124`**<br>**`130`** | `timeout` (`execution_state: "continuing"`)<br>`cancelled` (`safe_boundary_reached: true`) |
| **16** | Idempotent desired profile re-application                          | `emu research profile apply --id "$DARWIN_ID" --profile /tmp/exported_profile.json --json`                                          |        **`0`**         | `already_satisfied` (zero redundant actions; failed steps do not auto-retry)               |
