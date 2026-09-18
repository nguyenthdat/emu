//! Contract tests for two-step mutation proposal authorization, SHA-256 digest calculation,
//! expiration handling, cross-target/backend/revision rejection, live state reload, and single-use consumption.

use emu::models::research::{
    AuthorizationContext, BackendType, BootArtifactMap, CpuArchitecture, InstanceLifecycleState,
    MutationType, PrivilegeState, ProposalRequest, ResearchGuestId, ResearchGuestInstance,
    RootVerificationState, Sha256Digest,
};
use emu::persistence::instances::save_instance;
use emu::persistence::paths::ResearchPaths;
use emu::services::research::ResearchCoordinator;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

const VALID_CONFIG_REV: &str =
    "sha256:a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0";
const OTHER_CONFIG_REV: &str =
    "sha256:fedcba9876543210fedcba9876543210123456789abcdef0123456789abcdef0";

fn create_test_guest_instance(
    inst_id: &str,
    backend: BackendType,
    config_rev: &str,
) -> ResearchGuestInstance {
    let digest = Sha256Digest::new(config_rev);
    ResearchGuestInstance::new(
        ResearchGuestId::from_str(inst_id).expect("valid uuid"),
        "test-guest",
        backend,
        InstanceLifecycleState::Stopped,
        CpuArchitecture::Arm64,
        "14.0",
        "18A5351d",
        digest.clone(),
        digest.clone(),
        BootArtifactMap::new(digest.clone(), digest),
        PathBuf::from("/tmp/emu-test"),
        PrivilegeState::Unprivileged,
        RootVerificationState::Unverified,
    )
}

#[tokio::test]
async fn test_mutation_proposal_creation_and_consumption() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let coordinator = ResearchCoordinator::new(paths.clone());

    let inst_id = "e4b1c2d3-4567-49ab-cdef-0123456789ab".to_string();
    let affected = vec!["/tmp/test_instance.json".to_string()];
    let params = serde_json::json!({ "force": false });

    // 1. Persist live guest instance to satisfy live state reload during authorization
    let live_inst = create_test_guest_instance(&inst_id, BackendType::DarwinVm, VALID_CONFIG_REV);
    save_instance(&paths, &live_inst)
        .await
        .expect("save live instance");

    // 2. Dispatch an operation to receive journal tracking
    let op = coordinator
        .dispatch_operation(
            "instance_delete",
            Some(inst_id.clone()),
            Some(BackendType::DarwinVm),
        )
        .await
        .expect("Failed to dispatch operation");

    // 3. Create proposal
    let req = ProposalRequest {
        operation_type: MutationType::InstanceDelete,
        target_instance_id: Some(inst_id.clone()),
        target_resource: None,
        backend: BackendType::DarwinVm,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: affected.clone(),
        parameters: params.clone(),
        ttl_seconds: Some(900),
    };

    let proposal = coordinator
        .create_proposal(req)
        .await
        .expect("Failed to create proposal");

    assert!(proposal.proposal_digest.0.starts_with("sha256:"));
    assert_eq!(proposal.operation_type, MutationType::InstanceDelete);
    assert!(proposal.destructive);

    // 4. Verify and consume under lock with matching authorization context and required operation ID
    let auth_ctx = AuthorizationContext {
        proposal_digest: proposal.proposal_digest.clone(),
        operation_type: MutationType::InstanceDelete,
        target_instance_id: Some(inst_id.clone()),
        target_resource: None,
        backend: BackendType::DarwinVm,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: affected.clone(),
        parameters: params.clone(),
    };

    let guard = coordinator
        .verify_and_consume_proposal(&auth_ctx, &op.operation_id)
        .await
        .expect("Failed to verify and consume proposal");
    assert_eq!(guard.proposal().proposal_id, proposal.proposal_id);
    assert_eq!(
        guard.operation().consumed_proposal_digest,
        Some(proposal.proposal_digest.clone())
    );

    // Verify the operation record on disk journaled the consumed proposal digest
    let updated_op = emu::persistence::operations::load_operation(&paths, &op.operation_id)
        .await
        .expect("load_operation")
        .expect("op exists");
    assert_eq!(
        updated_op.consumed_proposal_digest,
        Some(proposal.proposal_digest.clone())
    );

    // 5. Re-consuming the same proposal must fail (single-use invariant: tombstone exists)
    let re_consume = coordinator
        .verify_and_consume_proposal(&auth_ctx, &op.operation_id)
        .await;
    assert!(
        re_consume.is_err(),
        "Second consumption of the same proposal must fail"
    );

    // 6. Recreating or re-saving the consumed proposal must be rejected by tombstone
    let save_again = emu::persistence::proposals::save_proposal(&paths, &proposal).await;
    assert!(
        save_again.is_err(),
        "Recreating a consumed proposal token must be rejected"
    );
}

#[tokio::test]
async fn test_mutation_proposal_cross_target_rejection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let coordinator = ResearchCoordinator::new(paths.clone());

    let inst_a = "11111111-1111-4111-8111-111111111111".to_string();
    let inst_b = "22222222-2222-4222-8222-222222222222".to_string();

    let live_b = create_test_guest_instance(&inst_b, BackendType::DarwinVm, VALID_CONFIG_REV);
    save_instance(&paths, &live_b).await.expect("save b");

    let op_b = coordinator
        .dispatch_operation(
            "instance_delete",
            Some(inst_b.clone()),
            Some(BackendType::DarwinVm),
        )
        .await
        .expect("op b");

    let req = ProposalRequest {
        operation_type: MutationType::InstanceDelete,
        target_instance_id: Some(inst_a),
        target_resource: None,
        backend: BackendType::DarwinVm,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/target_a.json".to_string()],
        parameters: serde_json::json!({}),
        ttl_seconds: Some(900),
    };

    let proposal = coordinator.create_proposal(req).await.expect("create");

    // Attempt to consume proposal against guest B
    let auth_ctx = AuthorizationContext {
        proposal_digest: proposal.proposal_digest.clone(),
        operation_type: MutationType::InstanceDelete,
        target_instance_id: Some(inst_b), // mismatch
        target_resource: None,
        backend: BackendType::DarwinVm,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/target_a.json".to_string()],
        parameters: serde_json::json!({}),
    };

    let res = coordinator
        .verify_and_consume_proposal(&auth_ctx, &op_b.operation_id)
        .await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("target instance mismatch") || err_msg.contains("TargetInstanceMismatch"),
        "Unexpected error message: {err_msg}"
    );
}

#[tokio::test]
async fn test_mutation_proposal_changed_config_rejection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let coordinator = ResearchCoordinator::new(paths.clone());
    let inst_id = "33333333-3333-4333-8333-333333333333".to_string();

    let live_inst = create_test_guest_instance(&inst_id, BackendType::Inferno, VALID_CONFIG_REV);
    save_instance(&paths, &live_inst).await.expect("save live");

    let op = coordinator
        .dispatch_operation(
            "disk_wipe",
            Some(inst_id.clone()),
            Some(BackendType::Inferno),
        )
        .await
        .expect("op");

    let req = ProposalRequest {
        operation_type: MutationType::DiskWipe,
        target_instance_id: Some(inst_id.clone()),
        target_resource: None,
        backend: BackendType::Inferno,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/disk.raw".to_string()],
        parameters: serde_json::json!({}),
        ttl_seconds: Some(900),
    };

    let proposal = coordinator.create_proposal(req).await.expect("create");

    // Live instance config is mutated on disk to OTHER_CONFIG_REV
    let mut updated_live = live_inst;
    updated_live.config_revision = Sha256Digest::new(OTHER_CONFIG_REV);
    emu::persistence::instances::update_instance_with_lock(&paths, &inst_id, |inst| {
        inst.config_revision = Sha256Digest::new(OTHER_CONFIG_REV);
        Ok(())
    })
    .await
    .expect("update live");

    let auth_ctx = AuthorizationContext {
        proposal_digest: proposal.proposal_digest.clone(),
        operation_type: MutationType::DiskWipe,
        target_instance_id: Some(inst_id),
        target_resource: None,
        backend: BackendType::Inferno,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/disk.raw".to_string()],
        parameters: serde_json::json!({}),
    };

    // Live state reload must detect that the guest config changed and reject authorization
    let res = coordinator
        .verify_and_consume_proposal(&auth_ctx, &op.operation_id)
        .await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("Stale proposal authorization rejected")
            || err_msg.contains("does not match proposal revision"),
        "Unexpected error message: {err_msg}"
    );
}

#[tokio::test]
async fn test_mutation_proposal_cross_backend_rejection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let coordinator = ResearchCoordinator::new(paths.clone());
    let inst_id = "44444444-4444-4444-8444-444444444444".to_string();

    let live_inst = create_test_guest_instance(&inst_id, BackendType::DarwinVm, VALID_CONFIG_REV);
    save_instance(&paths, &live_inst).await.expect("save live");

    let op = coordinator
        .dispatch_operation(
            "disk_wipe",
            Some(inst_id.clone()),
            Some(BackendType::DarwinVm),
        )
        .await
        .expect("op");

    let req = ProposalRequest {
        operation_type: MutationType::DiskWipe,
        target_instance_id: Some(inst_id.clone()),
        target_resource: None,
        backend: BackendType::DarwinVm,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/disk.raw".to_string()],
        parameters: serde_json::json!({}),
        ttl_seconds: Some(900),
    };

    let proposal = coordinator.create_proposal(req).await.expect("create");

    // Attempt to consume proposal under different backend
    let auth_ctx = AuthorizationContext {
        proposal_digest: proposal.proposal_digest.clone(),
        operation_type: MutationType::DiskWipe,
        target_instance_id: Some(inst_id),
        target_resource: None,
        backend: BackendType::Inferno, // mismatched backend
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/disk.raw".to_string()],
        parameters: serde_json::json!({}),
    };

    let res = coordinator
        .verify_and_consume_proposal(&auth_ctx, &op.operation_id)
        .await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("backend mismatch")
            || err_msg.contains("BackendMismatch")
            || err_msg.contains("Operation backend mismatch"),
        "Unexpected error message: {err_msg}"
    );
}

#[tokio::test]
async fn test_mutation_proposal_type_mismatch_rejection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let coordinator = ResearchCoordinator::new(paths.clone());
    let inst_id = "55555555-5555-4555-8555-555555555555".to_string();

    let live_inst = create_test_guest_instance(&inst_id, BackendType::Inferno, VALID_CONFIG_REV);
    save_instance(&paths, &live_inst).await.expect("save live");

    let op = coordinator
        .dispatch_operation(
            "disk_wipe",
            Some(inst_id.clone()),
            Some(BackendType::Inferno),
        )
        .await
        .expect("op");

    let req = ProposalRequest {
        operation_type: MutationType::DiskWipe,
        target_instance_id: Some(inst_id.clone()),
        target_resource: None,
        backend: BackendType::Inferno,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/test_disk.raw".to_string()],
        parameters: serde_json::json!({}),
        ttl_seconds: Some(900),
    };

    let proposal = coordinator.create_proposal(req).await.expect("create");

    // Attempt to consume with wrong expected mutation type
    let auth_ctx = AuthorizationContext {
        proposal_digest: proposal.proposal_digest.clone(),
        operation_type: MutationType::BaselineRestore, // type mismatch
        target_instance_id: Some(inst_id),
        target_resource: None,
        backend: BackendType::Inferno,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/test_disk.raw".to_string()],
        parameters: serde_json::json!({}),
    };

    let res = coordinator
        .verify_and_consume_proposal(&auth_ctx, &op.operation_id)
        .await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("operation type mismatch")
            || err_msg.contains("OperationTypeMismatch")
            || err_msg.contains("Operation type mismatch"),
        "Unexpected error message: {err_msg}"
    );
}

#[tokio::test]
async fn test_mutation_proposal_all_zero_config_rejected() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let coordinator = ResearchCoordinator::new(paths.clone());

    let all_zero_rev = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    let req = ProposalRequest {
        operation_type: MutationType::InstanceDelete,
        target_instance_id: Some("66666666-6666-4666-8666-666666666666".to_string()),
        target_resource: None,
        backend: BackendType::DarwinVm,
        config_revision: Sha256Digest::new(all_zero_rev),
        affected_paths: vec!["/tmp/test.json".to_string()],
        parameters: serde_json::json!({}),
        ttl_seconds: Some(900),
    };

    let res = coordinator.create_proposal(req).await;
    assert!(res.is_err(), "All-zero config revision must be rejected");
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("All-zero SHA-256") || err_msg.contains("AllZeroConfigRevision"),
        "Unexpected error message: {err_msg}"
    );
}
