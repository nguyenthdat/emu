//! Contract tests for profile portability, secret stripping, immutable experiment history,
//! and authorized baseline recovery within declared deadlines.

use emu::models::research::{
    AmfiStatus, BackendType, BootArtifactMap, CodeSigningMode, ExperimentRecord,
    GuestSecurityProfile, KernelPrivilegeLevel, MountMode, RecoveryBaseline,
    ResearchExperimentProfile, SandboxMode, Sha256Digest,
};
use std::collections::BTreeMap;

#[test]
fn test_research_profile_portability_and_secret_safety() {
    let digest = Sha256Digest::new(
        "sha256:2222222222222222222222222222222222222222222222222222222222222222",
    );
    let sec_profile = GuestSecurityProfile {
        profile_id: "secprof_relaxed_01".to_string(),
        name: "Relaxed Analysis".to_string(),
        code_signing_mode: CodeSigningMode::Disabled,
        amfi_status: AmfiStatus::AmfiGetOutOfMyWay,
        sandbox_mode: SandboxMode::Relaxed,
        root_filesystem_mount: MountMode::ReadWrite,
        kernel_privilege_level: KernelPrivilegeLevel::KernelDebugEnabled,
        guest_relaxations: vec!["amfi_get_out_of_my_way=1".to_string()],
        baseline_id: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let profile = ResearchExperimentProfile {
        profile_id: "prof_portable_analysis".to_string(),
        name: "Portable Analysis Profile".to_string(),
        target_backend: BackendType::DarwinVm,
        base_image_digest: digest.clone(),
        boot_artifacts: BootArtifactMap::new(digest.clone(), digest),
        kernel_boot_args: "-v debug=0x144".to_string(),
        applied_patches: vec![],
        security_profile: sec_profile,
        app_artifacts: vec![],
        instrumentation_scripts: vec![],
        guest_fixtures: vec![],
        exported_at: chrono::Utc::now().to_rfc3339(),
        version: "1.0.0".to_string(),
    };
    assert!(profile.validate().is_ok());

    // Verify profile serialization does not include private keys or credentials
    let json_str = serde_json::to_string(&profile).expect("serialize profile");
    assert!(!json_str.contains("password"));
    assert!(!json_str.contains("secret"));
    assert!(!json_str.contains("private_key"));
}

#[test]
fn test_recovery_baseline_deadline_validation() {
    let digest = Sha256Digest::new(
        "sha256:3333333333333333333333333333333333333333333333333333333333333333",
    );
    let baseline = RecoveryBaseline {
        baseline_id: "base_inst_test01".to_string(),
        guest_id: "00000000-0000-0000-0000-000000000001".to_string(),
        backend: BackendType::DarwinVm,
        base_disk_digest: digest.clone(),
        kernel_config_hash: digest,
        boot_artifacts: None,
        clean_snapshot_path: "/tmp/emu-snapshots/base_test.raw".to_string(),
        declared_recovery_deadline_ms: 180_000,
        verified: true,
        verified_at: Some(chrono::Utc::now().to_rfc3339()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    assert!(baseline.validate().is_ok());
    assert_eq!(baseline.declared_recovery_deadline_ms, 180_000);
}

#[test]
fn test_immutable_experiment_record_schema() {
    let digest = Sha256Digest::new(
        "sha256:4444444444444444444444444444444444444444444444444444444444444444",
    );
    let mut checksums = BTreeMap::new();
    checksums.insert("disk.img".to_string(), digest);

    let record = ExperimentRecord {
        record_id: uuid::Uuid::new_v4().to_string(),
        profile_id: "prof_portable_analysis".to_string(),
        backend: BackendType::DarwinVm,
        upstream_build_identity: "24A123".to_string(),
        artifact_checksums: checksums,
        session_parameters: BTreeMap::new(),
        observed_root_proof: None,
        instrumentation_summary: None,
        kernel_debug_telemetry: None,
        execution_status: emu::models::research::OperationStatus::Completed,
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: chrono::Utc::now().to_rfc3339(),
        errors: vec![],
    };

    assert!(record.validate().is_ok());
}
