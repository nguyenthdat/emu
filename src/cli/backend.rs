//! Backend capability discovery and preflight diagnostic commands.

use crate::cli::envelope::OutputEnvelope;
use crate::constants::research::EXIT_SUCCESS;
use crate::models::research::{
    BackendCapabilityProfile, BackendType, BinaryPrerequisite, CapabilityDetail,
    EntitlementPrerequisite, SupportStatus,
};
use crate::persistence::paths::ResearchPaths;
use anyhow::Result;

pub async fn preflight(_paths: &ResearchPaths, json: bool) -> Result<i32> {
    let is_apple_silicon = cfg!(all(target_os = "macos", target_arch = "aarch64"));
    let support = if is_apple_silicon {
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
            resolved_path: which::which("qemu-system-aarch64")
                .ok()
                .map(|p| p.display().to_string()),
            found: which::which("qemu-system-aarch64").is_ok(),
        }],
        required_entitlements: vec![EntitlementPrerequisite {
            name: "kern.hv_support".to_string(),
            granted: is_apple_silicon,
            details: Some("Evaluated during host preflight".to_string()),
        }],
        remediation_steps: vec!["Install custom darwin-vm QEMU binary".to_string()],
    };

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
                resolved_path: which::which("qemu-system-aarch64")
                    .ok()
                    .map(|p| p.display().to_string()),
                found: which::which("qemu-system-aarch64").is_ok(),
            },
            BinaryPrerequisite {
                binary_name: "ideviceinstaller".to_string(),
                resolved_path: which::which("ideviceinstaller")
                    .ok()
                    .map(|p| p.display().to_string()),
                found: which::which("ideviceinstaller").is_ok(),
            },
        ],
        required_entitlements: vec![],
        remediation_steps: vec![],
    };

    let profiles = vec![darwin_profile, inferno_profile];
    let data = serde_json::to_value(&profiles)?;
    let env = OutputEnvelope::success("op_preflight", Some(data));

    if json {
        env.print_stdout()?;
    } else {
        println!("Backend Preflight Diagnostics:");
        for p in &profiles {
            println!(
                "- {}: {:?} (Frameworks: {})",
                p.backend, p.support_status, p.app_frameworks_supported
            );
        }
    }
    Ok(EXIT_SUCCESS)
}

pub async fn list(_paths: &ResearchPaths, json: bool) -> Result<i32> {
    let list = serde_json::json!([
        {
            "backend": "darwin-vm",
            "description": "Minimal headless Darwin VM (root shell, CLI, kernel debug)",
            "app_frameworks_supported": false
        },
        {
            "backend": "Inferno",
            "description": "iOS research VM (SpringBoard, app lifecycle, Frida)",
            "app_frameworks_supported": true
        }
    ]);
    let env = OutputEnvelope::success("op_backend_list", Some(list));

    if json {
        env.print_stdout()?;
    } else {
        println!("Available Research Backends:");
        println!("  darwin-vm - Minimal headless Darwin VM (root shell, CLI, kernel debug)");
        println!("  Inferno   - iOS research VM (SpringBoard, app lifecycle, Frida)");
    }
    Ok(EXIT_SUCCESS)
}
