//! Repository and journal integration tests:
//! - Concurrent single-use proposal consumption under race
//! - Write-once immutable records under race (atomic create-new)
//! - Propagation of malformed records on list queries (no silent filtering)
//! - Guarded operation lifecycle transitions and two-phase cancellation
//! - Cancellation update under held worker ownership lock (.op.lock)
//! - Deterministic, bounded, corruption-proof streaming event reader
//! - Reconciler recovery with zero retries per FR-047 and lost cancellation handling

use emu::cli::envelope::StreamLogEnvelope;
use emu::constants::research::ERR_RUNTIME_EXECUTION_ERROR;
use emu::models::research::{
    AuthorizationContext, BackendType, BootArtifactMap, CpuArchitecture, ExperimentRecord,
    InstanceLifecycleState, MutationType, OperationRecord, OperationStatus, PrivilegeState,
    ProposalRequest, ResearchGuestId, ResearchGuestInstance, RootVerificationState, Sha256Digest,
};
use emu::persistence::instances::{list_instances, save_instance};
use emu::persistence::lock::AdvisoryLock;
use emu::persistence::operations::{
    append_operation_event, list_operations, load_operation, read_operation_events,
    save_operation_unlocked, update_operation_with_lock,
};
use emu::persistence::paths::ResearchPaths;
use emu::persistence::records::{list_records, load_record, save_record};
use emu::services::research::{OperationReconciler, ResearchCoordinator};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

const VALID_CONFIG_REV: &str =
    "sha256:1111111122222222333333334444444455555555666666667777777788888888";

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

// ============================================================================
// 1. Concurrent Single-Use Proposal Consumption Under Race
// ============================================================================

#[tokio::test]
async fn test_concurrent_single_use_proposal_under_race() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let coordinator = Arc::new(ResearchCoordinator::new(paths.clone()));
    let inst_id = "a1b2c3d4-e5f6-47a8-b9c0-d1e2f3a4b5c6".to_string();

    let live_inst = create_test_guest_instance(&inst_id, BackendType::DarwinVm, VALID_CONFIG_REV);
    save_instance(&paths, &live_inst).await.expect("save live");

    let op = coordinator
        .dispatch_operation(
            "instance_delete",
            Some(inst_id.clone()),
            Some(BackendType::DarwinVm),
        )
        .await
        .expect("op");

    let req = ProposalRequest {
        operation_type: MutationType::InstanceDelete,
        target_instance_id: Some(inst_id.clone()),
        target_resource: None,
        backend: BackendType::DarwinVm,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/instance.json".to_string()],
        parameters: serde_json::json!({}),
        ttl_seconds: Some(900),
    };

    let proposal = coordinator.create_proposal(req).await.expect("create");

    let auth_ctx = Arc::new(AuthorizationContext {
        proposal_digest: proposal.proposal_digest.clone(),
        operation_type: MutationType::InstanceDelete,
        target_instance_id: Some(inst_id),
        target_resource: None,
        backend: BackendType::DarwinVm,
        config_revision: Sha256Digest::new(VALID_CONFIG_REV),
        affected_paths: vec!["/tmp/instance.json".to_string()],
        parameters: serde_json::json!({}),
    });

    let op_id = Arc::new(op.operation_id.clone());

    // Spawn 8 concurrent tasks racing to consume the exact same proposal
    let mut handles = Vec::new();
    for _ in 0..8 {
        let coord = coordinator.clone();
        let ctx = auth_ctx.clone();
        let oid = op_id.clone();
        handles.push(tokio::spawn(async move {
            coord.verify_and_consume_proposal(&ctx, &oid).await
        }));
    }

    let mut success_count = 0;
    let mut failure_count = 0;

    for handle in handles {
        let result = handle.await.expect("task join");
        if result.is_ok() {
            success_count += 1;
        } else {
            failure_count += 1;
        }
    }

    // CRITICAL: Exactly ONE contender can win consumption; all others MUST fail!
    assert_eq!(
        success_count, 1,
        "Exactly one contender must successfully consume the proposal"
    );
    assert_eq!(
        failure_count, 7,
        "All other contenders must fail to consume the already-consumed proposal"
    );

    // Verify proposal file is deleted from disk
    let prop_file = paths
        .proposal_path(proposal.proposal_digest.as_str())
        .expect("path");
    assert!(
        !prop_file.exists(),
        "Consumed proposal file must not exist on disk"
    );
}

// ============================================================================
// 2. Write-Once Immutable Records Under Race (Atomic Create-New)
// ============================================================================

#[tokio::test]
async fn test_write_once_immutable_records_under_race() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let record_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let record_1 = ExperimentRecord {
        record_id: record_id.clone(),
        profile_id: "prof_alpha".to_string(),
        backend: BackendType::DarwinVm,
        upstream_build_identity: "build_alpha".to_string(),
        artifact_checksums: BTreeMap::new(),
        session_parameters: BTreeMap::new(),
        observed_root_proof: None,
        instrumentation_summary: None,
        kernel_debug_telemetry: None,
        execution_status: OperationStatus::Completed,
        started_at: now.clone(),
        completed_at: now.clone(),
        errors: Vec::new(),
    };

    let mut record_2 = record_1.clone();
    record_2.upstream_build_identity = "build_beta_overwrite_attempt".to_string();

    // Spawn 2 racing contenders attempting to save the same record ID
    let paths_1 = paths.clone();
    let h1 = tokio::spawn(async move { save_record(&paths_1, &record_1).await });

    let paths_2 = paths.clone();
    let h2 = tokio::spawn(async move { save_record(&paths_2, &record_2).await });

    let r1 = h1.await.expect("join 1");
    let r2 = h2.await.expect("join 2");

    let successes = [r1.is_ok(), r2.is_ok()].iter().filter(|&&ok| ok).count();
    let failures = [r1.is_err(), r2.is_err()]
        .iter()
        .filter(|&&err| err)
        .count();

    assert_eq!(
        successes, 1,
        "Exactly one contender must succeed in creating the immutable record"
    );
    assert_eq!(
        failures, 1,
        "The competing contender must receive an immutable record violation error"
    );

    // Verify record on disk can be loaded and was never overwritten
    let loaded = load_record(&paths, &record_id)
        .await
        .expect("load")
        .expect("exists");
    assert_eq!(loaded.record_id, record_id);
}

// ============================================================================
// 3. Propagation of Malformed Records on List Queries (No Silent Filtering)
// ============================================================================

#[tokio::test]
async fn test_malformed_record_propagation_on_list() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    // 1. Corrupt operation file in operations directory
    let corrupt_op_path = paths.operations().join("op_corrupted.json");
    tokio::fs::write(&corrupt_op_path, b"{\"operation_id\": \"INVALID_TRUNCATED")
        .await
        .expect("write corrupt");

    let list_res = list_operations(&paths).await;
    assert!(
        list_res.is_err(),
        "list_operations must propagate error when encountering corrupted JSON"
    );

    // Clean up corrupt operation to isolate next check
    let _ = tokio::fs::remove_file(&corrupt_op_path).await;

    // 2. Corrupt record file in records directory
    let corrupt_rec_path = paths.records().join("rec_corrupted.json");
    tokio::fs::write(&corrupt_rec_path, b"NOT_A_VALID_JSON_RECORD")
        .await
        .expect("write corrupt record");

    let list_rec_res = list_records(&paths).await;
    assert!(
        list_rec_res.is_err(),
        "list_records must propagate error when encountering corrupted JSON"
    );

    let _ = tokio::fs::remove_file(&corrupt_rec_path).await;

    // 3. Corrupt instance file in instances directory
    let corrupt_inst_path = paths.instances().join("inst_corrupted.json");
    tokio::fs::write(&corrupt_inst_path, b"{\"invalid\": true}")
        .await
        .expect("write corrupt instance");

    let list_inst_res = list_instances(&paths).await;
    assert!(
        list_inst_res.is_err(),
        "list_instances must propagate error when encountering corrupted JSON"
    );
}

// ============================================================================
// 4. Guarded Operation Lifecycle Transitions and Two-Phase Cancellation
// ============================================================================

#[test]
fn test_operation_state_transitions_guard() {
    let mut op = OperationRecord::new(
        "op_test123",
        "disk_wipe",
        Some(uuid::Uuid::new_v4().to_string()),
        Some(BackendType::DarwinVm),
    );

    assert_eq!(op.status, OperationStatus::Created);

    // Valid forward transitions
    assert!(
        op.transition_to(OperationStatus::Preflight, "preflight")
            .is_ok()
    );
    assert!(op.transition_to(OperationStatus::Staged, "staged").is_ok());
    assert!(op.start("executing").is_ok());
    assert_eq!(op.status, OperationStatus::Executing);
    assert!(op.started_at.is_some());

    assert!(
        op.transition_to(OperationStatus::Verifying, "verifying")
            .is_ok()
    );
    assert!(op.complete("settled").is_ok());
    assert_eq!(op.status, OperationStatus::Completed);
    assert_eq!(op.progress_percent, Some(100));
    assert!(op.completed_at.is_some());
    assert!(op.is_terminal());

    // CRITICAL: Any transition from terminal state must be REJECTED
    let invalid_restart = op.transition_to(OperationStatus::Executing, "executing");
    assert!(
        invalid_restart.is_err(),
        "Cannot transition out of terminal state Completed"
    );

    // Test Two-Phase Cancellation
    let mut op_cancel = OperationRecord::new(
        "op_cancel456",
        "disk_wipe",
        None,
        Some(BackendType::Inferno),
    );

    assert!(op_cancel.start("executing").is_ok());

    // Request cancellation: moves to CancellationPending, NOT Cancelled
    assert!(op_cancel.request_cancellation().is_ok());
    assert_eq!(op_cancel.status, OperationStatus::CancellationPending);
    assert!(!op_cancel.is_terminal());

    // Worker acknowledges cancellation: transitions to Cancelled
    assert!(op_cancel.acknowledge_cancellation().is_ok());
    assert_eq!(op_cancel.status, OperationStatus::Cancelled);
    assert!(op_cancel.is_terminal());

    // Cannot acknowledge again when not pending
    assert!(op_cancel.acknowledge_cancellation().is_err());
}

// ============================================================================
// 5. Cancellation Under Active Worker Ownership Lock (.op.lock)
// ============================================================================

#[tokio::test]
async fn test_cancellation_under_active_worker_lock() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let op_id = "op_WorkerActiveCancel";
    let mut op = OperationRecord::new(op_id, "disk_wipe", None, Some(BackendType::Inferno));
    op.status = OperationStatus::Executing;
    op.phase = "running".to_string();
    save_operation_unlocked(&paths, &op)
        .await
        .expect("initial save");

    // Simulate worker process acquiring and holding .op.lock
    let worker_lock_path = paths.operation_lock_path(op_id).expect("lock path");
    let worker_lock = AdvisoryLock::try_acquire(&worker_lock_path)
        .await
        .expect("worker acquires .op.lock");

    // While worker lock is actively held, external caller requests cancellation
    // Under journal lock (.journal.lock), this must succeed without contention on .op.lock!
    let updated = update_operation_with_lock(&paths, op_id, |op_rec| {
        op_rec.request_cancellation()?;
        Ok(())
    })
    .await
    .expect("update_operation_with_lock succeeds while worker holds .op.lock");

    assert_eq!(updated.status, OperationStatus::CancellationPending);

    // Worker subsequently acknowledges cancellation
    let final_op = update_operation_with_lock(&paths, op_id, |op_rec| {
        op_rec.acknowledge_cancellation()?;
        Ok(())
    })
    .await
    .expect("worker acknowledges cancellation");

    assert_eq!(final_op.status, OperationStatus::Cancelled);
    drop(worker_lock);
}

// ============================================================================
// 6. Deterministic, Bounded, Corruption-Proof Paginated Event Stream Reading
// ============================================================================

#[tokio::test]
async fn test_paginated_event_reader_bounded_and_corruption_proof() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let op_id = "op_EventsTest";

    // 1. Append 6 valid event lines
    for i in 0..6 {
        let env = StreamLogEnvelope::new(
            "INFO",
            "step",
            "executing",
            format!("Diagnostic step {i}"),
            op_id,
            None,
        );
        append_operation_event(&paths, op_id, &env)
            .await
            .expect("append");
    }

    // Case A: limit = Some(0) returns 0 events
    let (zero_limit_events, zero_cursor) =
        read_operation_events(&paths, op_id, Some(0), Some(0), None)
            .await
            .expect("limit 0");
    assert_eq!(zero_limit_events.len(), 0);
    assert_eq!(zero_cursor, 0);

    // Case B: max_bytes smaller than first record returns 0 events
    let (small_byte_events, small_cursor) =
        read_operation_events(&paths, op_id, Some(0), Some(5), Some(10))
            .await
            .expect("small byte budget");
    assert_eq!(small_byte_events.len(), 0);
    assert_eq!(small_cursor, 0);

    // Case C: Read page 1: limit 3
    let (page_1, next_cursor_1) = read_operation_events(&paths, op_id, Some(0), Some(3), None)
        .await
        .expect("page 1");
    assert_eq!(page_1.len(), 3);
    assert_eq!(next_cursor_1, 3);
    assert_eq!(page_1[0].message, "Diagnostic step 0");
    assert_eq!(page_1[2].message, "Diagnostic step 2");

    // Case D: Read page 2: cursor 3, limit 3
    let (page_2, next_cursor_2) =
        read_operation_events(&paths, op_id, Some(next_cursor_1), Some(3), None)
            .await
            .expect("page 2");
    assert_eq!(page_2.len(), 3);
    assert_eq!(next_cursor_2, 6);
    assert_eq!(page_2[0].message, "Diagnostic step 3");
    assert_eq!(page_2[2].message, "Diagnostic step 5");

    // Case E: Blank line corruption rejection
    let events_path = paths.operation_events_path(op_id).expect("path");
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&events_path)
        .await
        .expect("open");
    file.write_all(b"   \n").await.expect("write blank line");
    file.sync_data().await.expect("sync");

    // Reading stream at line 6 MUST return Err for blank line and NOT advance cursor
    let blank_read = read_operation_events(&paths, op_id, Some(6), Some(5), None).await;
    assert!(
        blank_read.is_err(),
        "read_operation_events must propagate error upon encountering blank line"
    );

    // Case F: Non-JSON corruption rejection
    file.write_all(b"CORRUPTED_NON_JSON_LINE\n")
        .await
        .expect("write non json");
    file.sync_data().await.expect("sync");

    let corrupt_read = read_operation_events(&paths, op_id, Some(7), Some(5), None).await;
    assert!(
        corrupt_read.is_err(),
        "read_operation_events must propagate error upon encountering corrupted JSON"
    );
}

// ============================================================================
// 7. Reconciler Zero-Retry and Lost Worker Recorded as Failed (FR-047)
// ============================================================================

#[tokio::test]
async fn test_reconciler_zero_retries_and_lost_worker_failed() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = Arc::new(ResearchPaths::new(temp_dir.path().to_path_buf()).expect("paths"));
    paths.ensure().await.expect("ensure");

    let reconciler = OperationReconciler::new();

    // 1. Create an executing operation whose worker terminated unexpectedly
    let mut op_exec = OperationRecord::new(
        "op_CrashedWorker",
        "image_prepare",
        None,
        Some(BackendType::DarwinVm),
    );
    op_exec.status = OperationStatus::Executing;
    op_exec.phase = "running".to_string();
    save_operation_unlocked(&paths, &op_exec)
        .await
        .expect("save");

    // 2. Create an operation that was in CancellationPending when worker terminated
    let mut op_cancelling = OperationRecord::new(
        "op_CancellingWorker",
        "disk_wipe",
        None,
        Some(BackendType::Inferno),
    );
    op_cancelling.status = OperationStatus::CancellationPending;
    op_cancelling.phase = "stopping".to_string();
    save_operation_unlocked(&paths, &op_cancelling)
        .await
        .expect("save");

    // Run reconciliation
    let reconciled_count = reconciler
        .reconcile_operations(&paths)
        .await
        .expect("reconcile");
    assert_eq!(reconciled_count, 2);

    // Verify op_exec was transitioned to Failed with ERR_RUNTIME_EXECUTION_ERROR
    let reloaded_exec = load_operation(&paths, "op_CrashedWorker")
        .await
        .expect("load")
        .expect("exists");
    assert_eq!(reloaded_exec.status, OperationStatus::Failed);
    assert_eq!(reloaded_exec.phase, "teardown");
    assert!(reloaded_exec.completed_at.is_some());
    let err = reloaded_exec.error.expect("error record present");
    assert_eq!(err.code, ERR_RUNTIME_EXECUTION_ERROR);

    // CRITICAL: Lost worker with CancellationPending MUST be Failed, NEVER Cancelled!
    let reloaded_cancel = load_operation(&paths, "op_CancellingWorker")
        .await
        .expect("load")
        .expect("exists");
    assert_eq!(
        reloaded_cancel.status,
        OperationStatus::Failed,
        "Lost worker with CancellationPending must reconcile to Failed (no safe acknowledgment)"
    );
    assert_eq!(reloaded_cancel.phase, "teardown");
    assert!(reloaded_cancel.completed_at.is_some());
}
