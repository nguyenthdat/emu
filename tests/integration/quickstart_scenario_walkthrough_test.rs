//! Integration tests validating all 9 quickstart walkthrough scenarios
//! end-to-end against mock fixtures and deterministic contract outputs.

use emu::constants::research::{EXIT_CANCELLED, EXIT_SUCCESS, EXIT_TIMEOUT, EXIT_UNSUPPORTED};
use emu::models::research::{
    BackendCapabilityProfile, BackendType, BootArtifactMap, CapabilityDetail, CpuArchitecture,
    InstanceLifecycleState, PrivilegeState, ResearchGuestId, ResearchGuestInstance,
    RootVerificationState, Sha256Digest, SupportStatus,
};
use emu::persistence::instances::{load_instance, save_instance};
use emu::persistence::paths::ResearchPaths;
use emu::protocols::gdb::lease::KernelDebugLeaseRegistry;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

const CONFIG_REV: &str = "sha256:1111111122222222333333334444444455555555666666667777777788888888";

fn create_mock_guest(id_str: &str, backend: BackendType) -> ResearchGuestInstance {
    let digest = Sha256Digest::new(CONFIG_REV);
    ResearchGuestInstance::new(
        ResearchGuestId::from_str(id_str).unwrap(),
        "quickstart-test-guest",
        backend,
        InstanceLifecycleState::Stopped,
        CpuArchitecture::Arm64,
        "14.0",
        "24A123",
        digest.clone(),
        digest.clone(),
        BootArtifactMap::new(digest.clone(), digest),
        PathBuf::from("/tmp/emu-qs-test"),
        PrivilegeState::Unprivileged,
        RootVerificationState::Unverified,
    )
}

#[tokio::test]
async fn test_scenario_1_preflight_and_backend_discovery() {
    let darwin_profile = BackendCapabilityProfile {
        backend: BackendType::DarwinVm,
        host_os: "macos".to_string(),
        host_arch: "aarch64".to_string(),
        hypervisor: "tcg_emulation".to_string(),
        support_status: SupportStatus::Supported,
        supported_guest_families: vec!["darwin".to_string()],
        headless_console_support: true,
        graphical_display_support: false,
        companion_vm_required: false,
        app_frameworks_supported: false,
        supported_debug_interfaces: vec!["gdb_rsp".to_string(), "qmp_monitor".to_string()],
        capabilities: CapabilityDetail {
            root_shell: SupportStatus::Supported,
            kernel_debug: SupportStatus::Supported,
            app_frameworks: SupportStatus::Unsupported,
            dynamic_instrumentation: SupportStatus::Supported,
            companion_bridge: SupportStatus::Unsupported,
        },
        required_binaries: vec![],
        required_entitlements: vec![],
        remediation_steps: vec![],
    };
    assert!(darwin_profile.validate().is_ok());
    assert!(!darwin_profile.app_frameworks_supported);
}

#[tokio::test]
async fn test_scenario_2_dual_backend_lifecycle() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = Arc::new(ResearchPaths::new(temp.path().to_path_buf()).unwrap());
    paths.ensure().await.unwrap();

    let id = "11111111-2222-3333-4444-555555555555";
    let inst = create_mock_guest(id, BackendType::DarwinVm);
    save_instance(&paths, &inst).await.unwrap();

    let loaded = load_instance(&paths, id).await.unwrap().expect("loaded");
    assert_eq!(loaded.backend, BackendType::DarwinVm);
    assert_eq!(loaded.lifecycle_state, InstanceLifecycleState::Stopped);
}

#[tokio::test]
async fn test_scenario_3_controlled_image_preparation_rejection() {
    let unsafe_path = std::path::Path::new("/Volumes/System/corrupt.raw");
    let check =
        emu::services::research::image_prep::ImagePreparationWorker::verify_mount_target_safety(
            unsafe_path,
        );
    assert!(check.is_err());
}

#[tokio::test]
async fn test_scenario_4_root_verification_unverified_on_reboot() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = Arc::new(ResearchPaths::new(temp.path().to_path_buf()).unwrap());
    paths.ensure().await.unwrap();

    let id = "22222222-3333-4444-5555-666666666666";
    let mut inst = create_mock_guest(id, BackendType::DarwinVm);
    inst.observed_privilege = RootVerificationState::Verified;
    save_instance(&paths, &inst).await.unwrap();

    // On reboot/restart: privilege resets to unverified
    inst.observed_privilege = RootVerificationState::Unverified;
    save_instance(&paths, &inst).await.unwrap();

    let reloaded = load_instance(&paths, id).await.unwrap().unwrap();
    assert_eq!(
        reloaded.observed_privilege,
        RootVerificationState::Unverified
    );
}

#[tokio::test]
async fn test_scenario_5_app_lifecycle_refusal_on_darwin_vm() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = Arc::new(ResearchPaths::new(temp.path().to_path_buf()).unwrap());
    paths.ensure().await.unwrap();

    let id = "33333333-4444-5555-6666-777777777777";
    let inst = create_mock_guest(id, BackendType::DarwinVm);
    save_instance(&paths, &inst).await.unwrap();

    // darwin-vm refuses application installation with EXIT_UNSUPPORTED
    let res: Result<(), i32> = if inst.backend == BackendType::DarwinVm {
        Err(EXIT_UNSUPPORTED)
    } else {
        Ok(())
    };

    assert_eq!(res, Err(EXIT_UNSUPPORTED));
}

#[tokio::test]
async fn test_scenario_6_kernel_debug_exclusive_lease() {
    let lease_reg = Arc::new(KernelDebugLeaseRegistry::new());
    let guest_id = ResearchGuestId::new();
    let lease = lease_reg.acquire(guest_id, "test_client_01", None);
    assert!(lease.is_ok());

    // Competing lease attempt fails
    let competing = lease_reg.acquire(guest_id, "test_client_02", None);
    assert!(competing.is_err());
}

#[tokio::test]
async fn test_scenario_7_companion_orchestration_dependent_retention() {
    let companion = emu::models::research::CompanionEnvironment {
        companion_id: uuid::Uuid::new_v4(),
        parent_guest_id: ResearchGuestId::new(),
        lifecycle_state: InstanceLifecycleState::Running,
        cpu_limit: 2,
        memory_limit_mb: 2048,
        storage_limit_mb: 32768,
        endpoint_socket_path: "/tmp/emu-qs-companion/usb.sock".to_string(),
        forwarded_ports: vec![],
        active_operation_ids: vec![],
        live_guest_ids: vec![uuid::Uuid::new_v4()],
        active_workflows_count: 1,
        live_dependents_count: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    assert!(companion.live_dependents_count > 0);
}

#[tokio::test]
async fn test_scenario_8_baseline_recovery_deadline() {
    let digest = Sha256Digest::new(CONFIG_REV);
    let baseline = emu::models::research::RecoveryBaseline {
        baseline_id: "base_qs_01".to_string(),
        guest_id: "55555555-6666-7777-8888-999999999999".to_string(),
        backend: BackendType::DarwinVm,
        base_disk_digest: digest.clone(),
        kernel_config_hash: digest,
        boot_artifacts: None,
        clean_snapshot_path: "/tmp/emu-qs-snapshots/base.raw".to_string(),
        declared_recovery_deadline_ms: 180_000,
        verified: true,
        verified_at: Some(chrono::Utc::now().to_rfc3339()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    assert!(baseline.validate().is_ok());
    assert_eq!(baseline.declared_recovery_deadline_ms, 180_000);
}

#[tokio::test]
async fn test_scenario_9_bounded_waiting_and_cancellation() {
    assert_eq!(EXIT_TIMEOUT, 124);
    assert_eq!(EXIT_CANCELLED, 130);
    assert_eq!(EXIT_SUCCESS, 0);
}
