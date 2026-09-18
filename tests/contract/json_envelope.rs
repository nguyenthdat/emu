//! Contract tests for JSON OutputEnvelope and StreamLogEnvelope formatting and schema validation.

use emu::cli::envelope::{EnvelopeOutcome, EnvelopeStatus, OutputEnvelope, StreamLogEnvelope};
use emu::models::error::ErrorRecord;
use emu::models::research::*;

#[test]
fn test_output_envelope_all_statuses_and_outcomes() {
    let cases = vec![
        (EnvelopeStatus::Success, EnvelopeOutcome::Completed, None, 0),
        (
            EnvelopeStatus::AlreadySatisfied,
            EnvelopeOutcome::AlreadySatisfied,
            None,
            0,
        ),
        (
            EnvelopeStatus::Accepted,
            EnvelopeOutcome::ProposalCreated,
            None,
            0,
        ),
        (
            EnvelopeStatus::Failed,
            EnvelopeOutcome::ExecutionFailed,
            Some(ErrorRecord::new(
                emu::constants::research::ERR_RUNTIME_EXECUTION_ERROR,
                "Execution failed",
                None,
            )),
            1,
        ),
        (
            EnvelopeStatus::Rejected,
            EnvelopeOutcome::InvalidInput,
            Some(ErrorRecord::new(
                emu::constants::research::ERR_INVALID_INPUT,
                "Invalid input",
                None,
            )),
            2,
        ),
        (
            EnvelopeStatus::Rejected,
            EnvelopeOutcome::Unsupported,
            Some(ErrorRecord::new(
                emu::constants::research::ERR_UNSUPPORTED_HOST,
                "Unsupported host",
                None,
            )),
            3,
        ),
        (
            EnvelopeStatus::Rejected,
            EnvelopeOutcome::AuthRefused,
            Some(ErrorRecord::new(
                emu::constants::research::ERR_AUTH_REQUIRED,
                "Auth required",
                None,
            )),
            4,
        ),
        (
            EnvelopeStatus::Rejected,
            EnvelopeOutcome::Conflict,
            Some(ErrorRecord::new(
                emu::constants::research::ERR_CONCURRENCY_CONFLICT,
                "Conflict",
                None,
            )),
            5,
        ),
        (
            EnvelopeStatus::TimedOut,
            EnvelopeOutcome::Timeout,
            Some(ErrorRecord::new(
                emu::constants::research::ERR_TIMEOUT,
                "Timeout",
                None,
            )),
            124,
        ),
        (
            EnvelopeStatus::TimedOut,
            EnvelopeOutcome::CancellationPending,
            Some(ErrorRecord::new(
                emu::constants::research::ERR_CANCELLATION_PENDING,
                "Cancellation pending",
                None,
            )),
            124,
        ),
        (
            EnvelopeStatus::Cancelled,
            EnvelopeOutcome::Cancelled,
            Some(ErrorRecord::new(
                emu::constants::research::ERR_CANCELLED,
                "Cancelled",
                None,
            )),
            130,
        ),
    ];

    for (status, outcome, error, expected_exit) in cases {
        let env = OutputEnvelope::new(
            status,
            outcome.clone(),
            "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
            Some(serde_json::json!({"test": true})),
            error,
        );
        assert!(
            env.validate().is_ok(),
            "Envelope must validate: {status:?} {outcome:?}"
        );
        let serialized = serde_json::to_string(&env).expect("Serialization failed");
        let val: serde_json::Value =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(val["status"].as_str().unwrap(), status.as_str());
        assert_eq!(val["outcome"].as_str().unwrap(), outcome.as_str());
        assert_eq!(
            val["operation_id"].as_str().unwrap(),
            "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0"
        );
        assert_eq!(env.exit_code(), expected_exit);
    }
}

#[test]
fn test_output_envelope_error_record_exit_code_propagation() {
    let err = ErrorRecord::new(
        emu::constants::research::ERR_AUTH_REQUIRED,
        "Administrative authorization required",
        Some(serde_json::json!({ "details": "sudo required" })),
    );
    let env = OutputEnvelope::rejected(
        EnvelopeOutcome::AuthRefused,
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        err,
        None,
    );

    assert_eq!(env.exit_code(), 4);
    let serialized = serde_json::to_string(&env).unwrap();
    assert!(serialized.contains("\"code\":\"AUTH_REQUIRED\""));
    assert!(serialized.contains("\"outcome\":\"auth_refused\""));
}

#[test]
fn test_stream_log_envelope_phases_and_formatting() {
    let phases = vec![
        "preflight",
        "staging",
        "booting",
        "verifying",
        "settled",
        "recovering",
        "stopping",
        "cleanup",
        "teardown",
        "paused",
        "executing",
        "unknown",
    ];

    for phase in phases {
        let log = StreamLogEnvelope::new(
            "INFO",
            "phase_transition",
            phase,
            format!("Transitioned to phase {phase}"),
            "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
            Some(serde_json::json!({"phase": phase})),
        );
        assert!(log.validate().is_ok());
        let serialized = serde_json::to_string(&log).unwrap();
        let val: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(val["phase"].as_str().unwrap(), phase);
        assert_eq!(val["level"].as_str().unwrap(), "INFO");
        assert!(val["timestamp"].as_str().unwrap().contains('T'));
        assert!(
            val.get("$schema").is_none(),
            "StreamLogEnvelope must NOT emit $schema"
        );
    }
}

#[test]
fn test_output_envelope_validation_invariants() {
    // Rejected/Failed envelope with null error must fail validation
    let invalid_rejected = OutputEnvelope::new(
        EnvelopeStatus::Rejected,
        EnvelopeOutcome::InvalidInput,
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        None,
        None,
    );
    assert!(invalid_rejected.validate().is_err());

    // Success envelope with non-null error must fail validation
    let invalid_success = OutputEnvelope::new(
        EnvelopeStatus::Success,
        EnvelopeOutcome::Completed,
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        None,
        Some(ErrorRecord::new(
            emu::constants::research::ERR_INVALID_INPUT,
            "Err",
            None,
        )),
    );
    assert!(invalid_success.validate().is_err());

    // Invalid operation_id pattern must fail validation
    let invalid_op_id = OutputEnvelope::success("invalid_id", None);
    assert!(invalid_op_id.validate().is_err());

    // Non-object data (e.g. array) must fail validation
    let invalid_data = OutputEnvelope::success(
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        Some(serde_json::json!([1, 2, 3])),
    );
    assert!(invalid_data.validate().is_err());
}

#[test]
fn test_stream_log_envelope_validation_invariants() {
    // Invalid level
    let bad_level = StreamLogEnvelope::new(
        "INVALID",
        "event",
        "booting",
        "msg",
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        None,
    );
    assert!(bad_level.validate().is_err());

    // Invalid phase
    let bad_phase = StreamLogEnvelope::new(
        "INFO",
        "event",
        "invalid_phase",
        "msg",
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        None,
    );
    assert!(bad_phase.validate().is_err());

    // Invalid operation_id
    let bad_op = StreamLogEnvelope::new("INFO", "event", "booting", "msg", "bad_op_id", None);
    assert!(bad_op.validate().is_err());
}

#[test]
fn test_sha256_digest_validation_boundaries() {
    // Valid 64-hex lowercase with prefix
    let valid = Sha256Digest::new(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    assert!(valid.validate().is_ok());

    // Missing prefix
    let no_prefix =
        Sha256Digest::new("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    assert!(no_prefix.validate().is_err());

    // Uppercase hex
    let uppercase = Sha256Digest::new(
        "sha256:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    assert!(uppercase.validate().is_err());

    // Too short (63 hex)
    let too_short =
        Sha256Digest::new("sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde");
    assert!(too_short.validate().is_err());

    // Too long (65 hex)
    let too_long = Sha256Digest::new(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0",
    );
    assert!(too_long.validate().is_err());

    // Non-hex character
    let non_hex = Sha256Digest::new(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
    );
    assert!(non_hex.validate().is_err());
}

#[test]
fn test_root_proof_evidence_validation_boundaries() {
    let now = chrono::Utc::now().to_rfc3339();
    let valid_digest = Sha256Digest::new(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );

    // 1. Unverified evidence without probes must pass base validation
    let unverified = RootProofEvidence {
        evidence_id: uuid::Uuid::new_v4().to_string(),
        guest_id: uuid::Uuid::new_v4().to_string(),
        boot_session_id: uuid::Uuid::new_v4().to_string(),
        backend: BackendType::DarwinVm,
        guest_build_identity: "darwin_kc_24A123".to_string(),
        image_artifact_digest: valid_digest.clone(),
        config_revision_hash: valid_digest.clone(),
        verified_uid: None,
        positive_probe_outcome: None,
        negative_control_outcome: None,
        observed_kernel_version: None,
        observed_boot_args: None,
        benign_binary_digest: None,
        verification_state: RootVerificationState::Unverified,
        verified_at: now.clone(),
        diagnostics: vec!["initial unverified probe state".to_string()],
    };
    assert!(unverified.validate().is_ok());

    // 2. Verified evidence missing positive probe outcome must fail
    let mut bad_verified = unverified.clone();
    bad_verified.verification_state = RootVerificationState::Verified;
    bad_verified.verified_uid = Some(0);
    assert!(bad_verified.validate().is_err());

    // 3. Verified evidence with positive probe but failing negative control (euid=0) must fail
    let positive_probe = ProbeOutcome {
        path: "/private/var/root/.emu_probe".to_string(),
        status: "success".to_string(),
        target_user: "root".to_string(),
        executed_euid: 0,
        expected_denial: false,
        helper_executed: true,
        syscall_errno: None,
        syscall_error_name: None,
        content_verified: Some(true),
        output: "emu_root_probe_v1\n".to_string(),
    };
    let bad_negative_probe = ProbeOutcome {
        path: "/private/var/root/.emu_probe".to_string(),
        status: "denied".to_string(),
        target_user: "mobile".to_string(),
        executed_euid: 0, // ERROR: must be dropped to non-zero uid (>= 1)
        expected_denial: true,
        helper_executed: true,
        syscall_errno: Some(1),
        syscall_error_name: Some("EPERM".to_string()),
        content_verified: None,
        output: "Permission denied".to_string(),
    };
    let mut bad_neg_verified = unverified.clone();
    bad_neg_verified.verification_state = RootVerificationState::Verified;
    bad_neg_verified.verified_uid = Some(0);
    bad_neg_verified.positive_probe_outcome = Some(positive_probe.clone());
    bad_neg_verified.negative_control_outcome = Some(bad_negative_probe);
    bad_neg_verified.observed_kernel_version = Some("Darwin 24.0.0".to_string());
    bad_neg_verified.observed_boot_args = Some("debug=0x144".to_string());
    bad_neg_verified.benign_binary_digest = Some(valid_digest.clone());
    assert!(bad_neg_verified.validate().is_err());

    // 4. Verified evidence with complete, valid positive probe and negative control must pass
    let valid_negative_probe = ProbeOutcome {
        path: "/private/var/root/.emu_probe".to_string(),
        status: "denied".to_string(),
        target_user: "mobile".to_string(),
        executed_euid: 501, // Non-zero drop
        expected_denial: true,
        helper_executed: true,
        syscall_errno: Some(1), // EPERM
        syscall_error_name: Some("EPERM".to_string()),
        content_verified: None,
        output: "Permission denied".to_string(),
    };
    let mut valid_verified = unverified.clone();
    valid_verified.verification_state = RootVerificationState::Verified;
    valid_verified.verified_uid = Some(0);
    valid_verified.positive_probe_outcome = Some(positive_probe);
    valid_verified.negative_control_outcome = Some(valid_negative_probe);
    valid_verified.observed_kernel_version = Some("Darwin 24.0.0".to_string());
    valid_verified.observed_boot_args = Some("debug=0x144".to_string());
    valid_verified.benign_binary_digest = Some(valid_digest.clone());
    assert!(valid_verified.validate().is_ok());
}

#[test]
fn test_backend_capability_profile_validation_boundaries() {
    let valid_profile = BackendCapabilityProfile {
        backend: BackendType::DarwinVm,
        host_os: "macos".to_string(),
        host_arch: "aarch64".to_string(),
        hypervisor: "hypervisor_framework".to_string(),
        support_status: SupportStatus::Supported,
        supported_guest_families: vec!["iOS".to_string()],
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
        required_binaries: vec![BinaryPrerequisite {
            binary_name: "qemu-system-aarch64".to_string(),
            found: true,
            resolved_path: Some("/opt/homebrew/bin/qemu-system-aarch64".to_string()),
        }],
        required_entitlements: vec![EntitlementPrerequisite {
            name: "com.apple.security.hypervisor".to_string(),
            granted: true,
            details: None,
        }],
        remediation_steps: vec![],
    };
    assert!(valid_profile.validate().is_ok());

    // Invalid host OS
    let mut bad_os = valid_profile.clone();
    bad_os.host_os = "linux".to_string();
    assert!(bad_os.validate().is_err());

    // Invalid hypervisor
    let mut bad_hyp = valid_profile.clone();
    bad_hyp.hypervisor = "kvm".to_string();
    assert!(bad_hyp.validate().is_err());

    // Invalid debug interface
    let mut bad_iface = valid_profile.clone();
    bad_iface.supported_debug_interfaces = vec!["invalid_debug".to_string()];
    assert!(bad_iface.validate().is_err());
}

#[test]
fn test_security_profile_baseline_id_serialization() {
    let now = chrono::Utc::now().to_rfc3339();
    let profile = GuestSecurityProfile {
        profile_id: "secprof_test".to_string(),
        name: "Test Profile".to_string(),
        code_signing_mode: CodeSigningMode::Enforced,
        amfi_status: AmfiStatus::Enforced,
        sandbox_mode: SandboxMode::Enforced,
        root_filesystem_mount: MountMode::ReadOnly,
        kernel_privilege_level: KernelPrivilegeLevel::Stock,
        guest_relaxations: vec![],
        baseline_id: None,
        created_at: now.clone(),
        updated_at: now,
    };
    assert!(profile.validate().is_ok());

    // Serialized output must contain "baseline_id":null, NOT omit it!
    let serialized = serde_json::to_string(&profile).unwrap();
    assert!(
        serialized.contains("\"baseline_id\":null"),
        "baseline_id must be serialized as null when None per schema required property"
    );
}

#[test]
fn test_draft07_schema_conformance_all_models() {
    let raw_schema = include_str!("../../specs/002-ios-root-vm/contracts/research.schema.json");
    let schema_json: serde_json::Value =
        serde_json::from_str(raw_schema).expect("Valid research.schema.json");
    let definitions = schema_json
        .get("definitions")
        .expect("schema has definitions")
        .clone();

    let make_validator = |def_name: &str| {
        let target = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "$ref": format!("#/definitions/{def_name}"),
            "definitions": definitions.clone(),
        });
        jsonschema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .should_validate_formats(true)
            .build(&target)
            .unwrap_or_else(|e| panic!("Failed to compile validator for {def_name}: {e}"))
    };

    let now = chrono::Utc::now().to_rfc3339();
    let valid_digest = Sha256Digest::new(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );

    // 1. OutputEnvelope (root schema) with format validation enabled
    let root_validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .should_validate_formats(true)
        .build(&schema_json)
        .expect("root validator");
    let success_env = OutputEnvelope::success(
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        Some(serde_json::json!({"installed": true})),
    );
    let val = serde_json::to_value(&success_env).unwrap();
    assert!(
        root_validator.is_valid(&val),
        "OutputEnvelope success must conform to schema"
    );
    let fail_env = OutputEnvelope::execution_failed(
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        ErrorRecord::new(
            emu::constants::research::ERR_RUNTIME_EXECUTION_ERROR,
            "Hypervisor exited unexpectedly",
            Some(serde_json::json!({"signal": 9})),
        ),
        None,
    );
    let val = serde_json::to_value(&fail_env).unwrap();
    assert!(
        root_validator.is_valid(&val),
        "OutputEnvelope failed must conform to schema"
    );

    // 2. StreamLogEnvelope
    let log_validator = make_validator("StreamLogEnvelope");
    let log_env = StreamLogEnvelope::new(
        "INFO",
        "vmm_boot",
        "booting",
        "Guest vCPU started",
        "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
        Some(serde_json::json!({"vcpu": 0})),
    );
    let val = serde_json::to_value(&log_env).unwrap();
    assert!(
        log_validator.is_valid(&val),
        "StreamLogEnvelope must conform to schema"
    );

    // 3. ErrorRecord
    let err_validator = make_validator("ErrorRecord");
    let err_record = ErrorRecord::new(
        emu::constants::research::ERR_AUTH_REQUIRED,
        "Interactive credentials required",
        Some(serde_json::json!({"prompt": "sudo"})),
    );
    let val = serde_json::to_value(&err_record).unwrap();
    assert!(
        err_validator.is_valid(&val),
        "ErrorRecord must conform to schema"
    );

    // 4. ResearchGuestInstance
    let guest_validator = make_validator("ResearchGuestInstance");
    let guest_id = ResearchGuestId::new();
    let guest = ResearchGuestInstance::new(
        guest_id,
        "darwin-test-guest",
        BackendType::DarwinVm,
        InstanceLifecycleState::Stopped,
        CpuArchitecture::Arm64,
        "Darwin 24.0.0",
        "22A123",
        valid_digest.clone(),
        valid_digest.clone(),
        BootArtifactMap::new(valid_digest.clone(), valid_digest.clone()),
        std::path::PathBuf::from("/tmp/guest-runtime"),
        PrivilegeState::Root,
        RootVerificationState::Unverified,
    );
    let val = serde_json::to_value(&guest).unwrap();
    assert!(
        guest_validator.is_valid(&val),
        "ResearchGuestInstance must conform to schema"
    );

    // 5. BackendCapabilityProfile
    let backend_validator = make_validator("BackendCapabilityProfile");
    let profile = BackendCapabilityProfile {
        backend: BackendType::DarwinVm,
        host_os: "macos".to_string(),
        host_arch: "aarch64".to_string(),
        hypervisor: "hypervisor_framework".to_string(),
        support_status: SupportStatus::Supported,
        supported_guest_families: vec!["iOS".to_string()],
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
        required_binaries: vec![BinaryPrerequisite {
            binary_name: "qemu-system-aarch64".to_string(),
            found: true,
            resolved_path: Some("/opt/homebrew/bin/qemu-system-aarch64".to_string()),
        }],
        required_entitlements: vec![EntitlementPrerequisite {
            name: "com.apple.security.hypervisor".to_string(),
            granted: true,
            details: None,
        }],
        remediation_steps: vec![],
    };
    let val = serde_json::to_value(&profile).unwrap();
    assert!(
        backend_validator.is_valid(&val),
        "BackendCapabilityProfile must conform to schema"
    );

    // 6. GuestSecurityProfile
    let secprof_validator = make_validator("GuestSecurityProfile");
    let sec_profile = GuestSecurityProfile {
        profile_id: "secprof_default".to_string(),
        name: "Default Profile".to_string(),
        code_signing_mode: CodeSigningMode::Enforced,
        amfi_status: AmfiStatus::Enforced,
        sandbox_mode: SandboxMode::Enforced,
        root_filesystem_mount: MountMode::ReadOnly,
        kernel_privilege_level: KernelPrivilegeLevel::Stock,
        guest_relaxations: vec![],
        baseline_id: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let val = serde_json::to_value(&sec_profile).unwrap();
    assert!(
        secprof_validator.is_valid(&val),
        "GuestSecurityProfile must conform to schema"
    );

    // 7. CompanionEnvironment
    let companion_validator = make_validator("CompanionEnvironment");
    let companion = CompanionEnvironment {
        companion_id: uuid::Uuid::new_v4(),
        parent_guest_id: guest_id,
        lifecycle_state: InstanceLifecycleState::Stopped,
        cpu_limit: 2,
        memory_limit_mb: 2048,
        storage_limit_mb: 10240,
        endpoint_socket_path: "/tmp/companion.sock".to_string(),
        forwarded_ports: vec![2222],
        active_operation_ids: vec![],
        live_guest_ids: vec![],
        active_workflows_count: 0,
        live_dependents_count: 0,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let val = serde_json::to_value(&companion).unwrap();
    assert!(
        companion_validator.is_valid(&val),
        "CompanionEnvironment must conform to schema"
    );

    // 8. InstrumentationSession
    let session_validator = make_validator("InstrumentationSession");
    let session = InstrumentationSession {
        session_id: "session_frida_01".to_string(),
        guest_id,
        boot_session_id: BootSessionId::new(),
        target_process_id: 1234,
        target_bundle_id: Some("com.apple.springboard".to_string()),
        agent_package_version: "17.18.0".to_string(),
        agent_integrity_hash: valid_digest.clone(),
        tls_server_cert_pinned: true,
        session_token_configured: true,
        injected_script_hashes: vec![valid_digest.clone()],
        hook_status: HookStatus {
            native_hooks_count: 10,
            objc_hooks_count: 20,
            events_intercepted: 100,
        },
        attachment_state: FridaAttachmentState::Attached,
        attached_at: Some(now.clone()),
        detached_at: None,
        captured_hook_events_count: 100,
        control_process_id: None,
        control_process_hooked: false,
        diagnostics: vec!["frida agent injected successfully".to_string()],
    };
    let val = serde_json::to_value(&session).unwrap();
    assert!(
        session_validator.is_valid(&val),
        "InstrumentationSession must conform to schema"
    );

    // 9. RecoveryBaseline
    let baseline_validator = make_validator("RecoveryBaseline");
    let baseline = RecoveryBaseline {
        baseline_id: "baseline_snap_01".to_string(),
        guest_id: guest_id.to_string(),
        backend: BackendType::DarwinVm,
        base_disk_digest: valid_digest.clone(),
        kernel_config_hash: valid_digest.clone(),
        boot_artifacts: Some(BootArtifactMap::new(
            valid_digest.clone(),
            valid_digest.clone(),
        )),
        clean_snapshot_path: "/tmp/clean.qcow2".to_string(),
        declared_recovery_deadline_ms: 5000,
        verified: true,
        verified_at: Some(now.clone()),
        created_at: now.clone(),
    };
    let val = serde_json::to_value(&baseline).unwrap();
    assert!(
        baseline_validator.is_valid(&val),
        "RecoveryBaseline must conform to schema"
    );

    // 10. ResearchImageArtifact
    let artifact_validator = make_validator("ResearchImageArtifact");
    let artifact = ResearchImageArtifact {
        artifact_id: "art_kc_01".to_string(),
        artifact_type: ImageArtifactType::Kernelcache,
        file_path: "/tmp/kernelcache".to_string(),
        sha256_digest: valid_digest.clone(),
        origin_metadata: ImageOriginMetadata {
            source_type: "user_supplied".to_string(),
            build_identity: "22A123".to_string(),
            source_url_or_ref: None,
        },
        target_backend: BackendType::DarwinVm,
        build_version_identity: "22A123".to_string(),
        verified_device_node: None,
        verified_volume_uuid: None,
        trust_status: ArtifactTrustStatus::VerifiedTrusted,
        created_at: now.clone(),
    };
    let val = serde_json::to_value(&artifact).unwrap();
    assert!(
        artifact_validator.is_valid(&val),
        "ResearchImageArtifact must conform to schema"
    );

    // 11. RootProofEvidence (verified)
    let proof_validator = make_validator("RootProofEvidence");
    let verified_proof = RootProofEvidence {
        evidence_id: uuid::Uuid::new_v4().to_string(),
        guest_id: guest_id.to_string(),
        boot_session_id: uuid::Uuid::new_v4().to_string(),
        backend: BackendType::DarwinVm,
        guest_build_identity: "22A123".to_string(),
        image_artifact_digest: valid_digest.clone(),
        config_revision_hash: valid_digest.clone(),
        verified_uid: Some(0),
        positive_probe_outcome: Some(ProbeOutcome {
            path: "/private/var/root/.emu_probe".to_string(),
            status: "success".to_string(),
            target_user: "root".to_string(),
            executed_euid: 0,
            expected_denial: false,
            helper_executed: true,
            syscall_errno: None,
            syscall_error_name: None,
            content_verified: Some(true),
            output: "emu_root_probe_v1\n".to_string(),
        }),
        negative_control_outcome: Some(ProbeOutcome {
            path: "/private/var/root/.emu_probe".to_string(),
            status: "denied".to_string(),
            target_user: "mobile".to_string(),
            executed_euid: 501,
            expected_denial: true,
            helper_executed: true,
            syscall_errno: Some(1),
            syscall_error_name: Some("EPERM".to_string()),
            content_verified: None,
            output: "Permission denied".to_string(),
        }),
        observed_kernel_version: Some("Darwin 24.0.0".to_string()),
        observed_boot_args: Some("debug=0x144".to_string()),
        benign_binary_digest: Some(valid_digest.clone()),
        verification_state: RootVerificationState::Verified,
        verified_at: now.clone(),
        diagnostics: vec!["empirical probe passed".to_string()],
    };
    let val = serde_json::to_value(&verified_proof).unwrap();
    assert!(
        proof_validator.is_valid(&val),
        "RootProofEvidence (verified) must conform to schema"
    );

    // 12. ApplicationArtifact
    let app_validator = make_validator("ApplicationArtifact");
    let app_art = ApplicationArtifact {
        app_id: ApplicationId::new("app_springboard_01"),
        bundle_identifier: "com.apple.springboard".to_string(),
        bundle_name: "SpringBoard".to_string(),
        package_path: std::path::PathBuf::from("/Applications/SpringBoard.app"),
        sha256_digest: valid_digest.clone(),
        binary_architecture: CpuArchitecture::Arm64,
        code_signature_identity: "Apple System".to_string(),
        sandbox_container_path: Some(
            "/private/var/mobile/Containers/Data/Application/SB".to_string(),
        ),
        entitlement_manifest: serde_json::json!({"get-task-allow": false}),
        deployment_status: AppDeploymentStatus::Installed,
        target_guest_id: Some(uuid::Uuid::new_v4()),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let val = serde_json::to_value(&app_art).unwrap();
    assert!(
        app_validator.is_valid(&val),
        "ApplicationArtifact must conform to schema"
    );

    // 13. BootArtifactMap
    let boot_validator = make_validator("BootArtifactMap");
    let boot_map = BootArtifactMap::new(valid_digest.clone(), valid_digest.clone());
    let val = serde_json::to_value(&boot_map).unwrap();
    assert!(
        boot_validator.is_valid(&val),
        "BootArtifactMap must conform to schema"
    );

    // 14. ResearchExperimentProfile
    let exp_prof_validator = make_validator("ResearchExperimentProfile");
    let exp_prof = ResearchExperimentProfile {
        profile_id: "profile_exp_01".to_string(),
        name: "Security Research Baseline".to_string(),
        target_backend: BackendType::DarwinVm,
        base_image_digest: valid_digest.clone(),
        boot_artifacts: BootArtifactMap::new(valid_digest.clone(), valid_digest.clone()),
        kernel_boot_args: "debug=0x144 keepsyms=1".to_string(),
        applied_patches: vec!["amfi_bypass_v1".to_string()],
        security_profile: sec_profile,
        app_artifacts: vec![valid_digest.clone()],
        instrumentation_scripts: vec![valid_digest.clone()],
        guest_fixtures: vec!["payload.bin".to_string()],
        exported_at: now.clone(),
        version: "1.0.0".to_string(),
    };
    let val = serde_json::to_value(&exp_prof).unwrap();
    assert!(
        exp_prof_validator.is_valid(&val),
        "ResearchExperimentProfile must conform to schema"
    );

    // 15. ExperimentRecord
    let record_validator = make_validator("ExperimentRecord");
    let exp_record = ExperimentRecord {
        record_id: uuid::Uuid::new_v4().to_string(),
        profile_id: "profile_exp_01".to_string(),
        backend: BackendType::DarwinVm,
        upstream_build_identity: "Darwin 24.0.0".to_string(),
        artifact_checksums: std::collections::BTreeMap::from([(
            "kc".to_string(),
            valid_digest.clone(),
        )]),
        session_parameters: std::collections::BTreeMap::from([(
            "mode".to_string(),
            serde_json::json!("research"),
        )]),
        observed_root_proof: Some(verified_proof.clone()),
        instrumentation_summary: Some(HookStatus {
            native_hooks_count: 5,
            objc_hooks_count: 10,
            events_intercepted: 50,
        }),
        kernel_debug_telemetry: Some(DebugTelemetryRecord {
            breakpoints_hit_count: 2,
            step_operations_count: 4,
            memory_reads_count: 10,
            memory_writes_count: 0,
            register_reads_count: 20,
            disconnect_paused_observed: false,
        }),
        execution_status: OperationStatus::Completed,
        started_at: now.clone(),
        completed_at: now.clone(),
        errors: vec![],
    };
    let val = serde_json::to_value(&exp_record).unwrap();
    assert!(
        record_validator.is_valid(&val),
        "ExperimentRecord must conform to schema"
    );

    // 16. GuestProcessRecord
    let proc_validator = make_validator("GuestProcessRecord");
    let proc_record = GuestProcessRecord {
        pid: 1,
        ppid: 0,
        name: "launchd".to_string(),
        user: "root".to_string(),
        uid: 0,
        arch: CpuArchitecture::Arm64,
    };
    let val = serde_json::to_value(&proc_record).unwrap();
    assert!(
        proc_validator.is_valid(&val),
        "GuestProcessRecord must conform to schema"
    );

    // 17. GuestMachServiceRecord
    let mach_validator = make_validator("GuestMachServiceRecord");
    let mach_record = GuestMachServiceRecord {
        service_name: "com.apple.system.logger".to_string(),
        active: true,
        pid: Some(42),
    };
    let val = serde_json::to_value(&mach_record).unwrap();
    assert!(
        mach_validator.is_valid(&val),
        "GuestMachServiceRecord must conform to schema"
    );

    // 18. Schema constraint regression check: missing required field and invalid pattern must fail Draft07 validation
    let mut invalid_guest_json = serde_json::to_value(&guest).unwrap();
    invalid_guest_json.as_object_mut().unwrap().remove("id");
    assert!(
        !guest_validator.is_valid(&invalid_guest_json),
        "Guest validator must reject missing required field 'id'"
    );

    let mut invalid_digest_json = serde_json::to_value(&guest).unwrap();
    invalid_digest_json["config_revision"] = serde_json::json!("not-a-valid-sha256");
    assert!(
        !guest_validator.is_valid(&invalid_digest_json),
        "Guest validator must reject invalid pattern for config_revision"
    );
}
