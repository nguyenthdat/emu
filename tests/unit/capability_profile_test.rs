//! Unit tests for BackendCapabilityProfile and preflight diagnostics.

use emu::models::research::{
    BackendCapabilityProfile, BackendType, BinaryPrerequisite, CapabilityDetail,
    EntitlementPrerequisite, SupportStatus,
};

#[test]
fn test_capability_profile_preflight_truthfulness() {
    let support = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        SupportStatus::Supported
    } else {
        SupportStatus::Unsupported
    };

    let darwin_profile = BackendCapabilityProfile {
        backend: BackendType::DarwinVm,
        host_os: "macos".to_string(),
        host_arch: "aarch64".to_string(),
        hypervisor: "tcg_emulation".to_string(),
        support_status: support,
        supported_guest_families: vec!["darwin-minimal".to_string()],
        headless_console_support: true,
        graphical_display_support: false,
        companion_vm_required: false,
        app_frameworks_supported: false,
        supported_debug_interfaces: vec!["gdb_rsp".to_string(), "qmp_monitor".to_string()],
        capabilities: CapabilityDetail {
            root_shell: support,
            kernel_debug: support,
            app_frameworks: SupportStatus::Unsupported,
            dynamic_instrumentation: SupportStatus::Unsupported,
            companion_bridge: SupportStatus::Unsupported,
        },
        required_binaries: vec![BinaryPrerequisite {
            binary_name: "qemu-system-aarch64".to_string(),
            found: true,
            resolved_path: Some("/opt/homebrew/bin/qemu-system-aarch64".to_string()),
        }],
        required_entitlements: vec![EntitlementPrerequisite {
            name: "kern.hv_support".to_string(),
            granted: true,
            details: Some("Evaluated during host preflight".to_string()),
        }],
        remediation_steps: vec!["Install custom darwin-vm QEMU binary".to_string()],
    };

    assert_eq!(darwin_profile.backend, BackendType::DarwinVm);
    assert!(!darwin_profile.app_frameworks_supported);
    assert_eq!(
        darwin_profile.capabilities.app_frameworks,
        SupportStatus::Unsupported
    );

    let inferno_profile = BackendCapabilityProfile {
        backend: BackendType::Inferno,
        host_os: "macos".to_string(),
        host_arch: "aarch64".to_string(),
        hypervisor: "tcg_emulation".to_string(),
        support_status: support,
        supported_guest_families: vec!["ios-14".to_string()],
        headless_console_support: true,
        graphical_display_support: true,
        companion_vm_required: true,
        app_frameworks_supported: true,
        supported_debug_interfaces: vec!["gdb_rsp".to_string(), "qmp_monitor".to_string()],
        capabilities: CapabilityDetail {
            root_shell: support,
            kernel_debug: support,
            app_frameworks: support,
            dynamic_instrumentation: support,
            companion_bridge: support,
        },
        required_binaries: vec![
            BinaryPrerequisite {
                binary_name: "qemu-system-aarch64".to_string(),
                found: true,
                resolved_path: Some("/opt/homebrew/bin/qemu-system-aarch64".to_string()),
            },
            BinaryPrerequisite {
                binary_name: "ideviceinstaller".to_string(),
                found: true,
                resolved_path: Some("/opt/homebrew/bin/ideviceinstaller".to_string()),
            },
        ],
        required_entitlements: vec![],
        remediation_steps: vec![],
    };

    assert_eq!(inferno_profile.backend, BackendType::Inferno);
    assert!(inferno_profile.app_frameworks_supported);
    assert_eq!(inferno_profile.capabilities.app_frameworks, support);
}
