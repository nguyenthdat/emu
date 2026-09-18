# iOS Root VM and Darwin Security Research Backends

## 1. Overview and Architectural Scope

Emu provides dedicated mobile virtual machine and Darwin guest management for security research on macOS Apple Silicon (`aarch64`).
The platform features a complementary dual-backend virtualization architecture:

- **`darwin-vm`**: Specialized in minimal headless Darwin virtual machine guests, kernel bootstrap, direct root guest CLI binaries, and low-level kernel debugging via GDB RSP. Application layer frameworks (SpringBoard, UIKit, IPA lifecycle) are explicitly unsupported and refused with exit code 3 (`APP_FRAMEWORKS_UNAVAILABLE`).
- **`Inferno`**: Specialized in complete iOS research environments based on QEMU-SPTM, supporting userland application lifecycle (IPA install, launch, container export), SpringBoard/UIKit services, and dynamic binary instrumentation via Frida 17.18.0.

```
+-------------------------------------------------------------------------+
|                               Emu CLI                                    |
|              (Single Unified Binary with Headless Bypass)               |
+------------------------------------+------------------------------------+
                                     |
              +----------------------+----------------------+
              |                                             |
              v                                             v
     +-----------------+                           +-----------------+
     |    darwin-vm    |                           |     Inferno     |
     | Minimal Darwin  |                           | Full iOS (SPTM) |
     +--------+--------+                           +--------+--------+
              |                                             |
     +--------+--------+                           +--------+--------+
     |  Kernel Console |                           | App Lifecycle   |
     |  GDB RSP Debug  |                           | Frida 17.18.0   |
     |  Base Recovery  |                           | Companion Linux |
     +-----------------+                           +-----------------+
```

---

## 2. Core Invariants and Security Boundaries

1. **Zero Host Mutation**:
   All guest execution, in-guest filesystem writes, kernel debugging, and Frida dynamic instrumentation operate strictly inside virtual guest instances. The host operating system's System Integrity Protection (SIP), Sealed System Volume (SSV), and NVRAM remain completely untouched.

2. **Isolated Child Process Supervision**:
   Hypervisor instances are managed by private child supervisor processes (`emu __supervise --vm-id <ID>`) running in detached process groups (`NewProcessGroup`). Supervisors hold exclusive cross-process advisory locks (`fs4::FileExt`) on `.lock` files, preventing concurrent operations on the same guest.

3. **Restricted Runtime Directories**:
   All Unix Domain Sockets (QMP, supervisor, guest console, GDB RSP, Frida bridge) are created within owner-restricted temporary directories (`/tmp/emu-<short_uuid>/`) with POSIX `0700` permissions. Sockets are deleted cleanly upon process shutdown.

4. **Two-Step Safety Gates**:
   Destructive mutations (`guest delete`, `guest wipe`, `baseline restore`, `security-profile relax`) require explicit confirmation challenge parameters (`--confirm <CHALLENGE_STRING>`). Unattended requests without confirmation fail immediately with exit code 4 (`AUTH_REQUIRED`).

5. **Truthful Falsification and Negative Controls**:
   Root verification workflows require both a positive root probe (UID 0 execution) and an unprivileged falsification probe (UID 501 / mobile). Any unexpected privilege leak invalidates the proof and transitions instance status to `Unverified`.

---

## 3. CLI Command Family Reference

All research capabilities are routed through `emu research <FAMILY>`:

### 3.1 Backend Diagnostics

```bash
emu research backend list --json
emu research backend preflight --backend darwin-vm --json
emu research backend preflight --backend inferno --json
```

### 3.2 Guest Lifecycle

```bash
emu research guest create --name lab-darwin-01 --backend darwin-vm --base-image /path/to/disk.img
emu research guest list --json
emu research guest start --id <GUEST_UUID>
emu research guest stop --id <GUEST_UUID>
emu research guest restart --id <GUEST_UUID>
emu research guest inspect --id <GUEST_UUID> --json
emu research guest delete --id <GUEST_UUID> --confirm <CHALLENGE>
```

### 3.3 Root Verification and In-Guest Console

```bash
emu research root verify --id <GUEST_UUID> --json
emu research root status --id <GUEST_UUID> --json
emu research root console --id <GUEST_UUID> --command "uname -a"
```

### 3.4 Application Lifecycle (`Inferno` only)

```bash
emu research app install --id <GUEST_UUID> --file /path/to/payload.ipa
emu research app list --id <GUEST_UUID> --json
emu research app start --id <GUEST_UUID> --bundle-id com.example.target
emu research app stop --id <GUEST_UUID> --bundle-id com.example.target
emu research app container-export --id <GUEST_UUID> --bundle-id com.example.target --output /path/to/out.tar.gz --authorize-export
```

### 3.5 Dynamic Binary Instrumentation (Frida)

```bash
emu research frida attach --id <GUEST_UUID> --bundle-id com.example.target --script /path/to/hook.js
emu research frida status --id <GUEST_UUID> --json
emu research frida detach --id <GUEST_UUID> --session-id <SESSION_ID>
```

### 3.6 Kernel Debugging (GDB RSP)

```bash
emu research debug pause --id <GUEST_UUID>
emu research debug registers --id <GUEST_UUID> --json
emu research debug resume --id <GUEST_UUID>
emu research debug disconnect --id <GUEST_UUID>
```

### 3.7 Profiles and Baselines

```bash
emu research profile apply --id <GUEST_UUID> --profile-id secprof_relaxed --confirm <CHALLENGE>
emu research baseline create --id <GUEST_UUID> --json
emu research baseline restore --id <GUEST_UUID> --baseline-id base_xxx --confirm <CHALLENGE>
```

---

## 4. Exit Code Taxonomy

| Code  | Identifier             | Description                                                               |
| ----- | ---------------------- | ------------------------------------------------------------------------- |
| `0`   | `EXIT_SUCCESS`         | Command completed successfully or desired state already satisfied         |
| `1`   | `EXIT_RUNTIME_FAILURE` | Unhandled runtime or execution error                                      |
| `2`   | `EXIT_INVALID_INPUT`   | Syntax error, unknown parameter, or schema contract violation             |
| `3`   | `EXIT_UNSUPPORTED`     | Capability not supported on target backend (e.g. apps on `darwin-vm`)     |
| `4`   | `EXIT_AUTH_REFUSED`    | Elevated operation or destructive confirmation missing in unattended mode |
| `5`   | `EXIT_CONFLICT`        | Resource lock contended or conflicting active session                     |
| `124` | `EXIT_TIMEOUT`         | Bounded wait timeout elapsed while background operation continues         |
| `130` | `EXIT_CANCELLED`       | Operation cleanly cancelled at confirmed safe boundary                    |
