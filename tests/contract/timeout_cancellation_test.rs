//! Contract tests for bounded observer waiting, caller wait timeout (exit code 124),
//! and cooperative confirmed cancellation (exit code 130).

use emu::cli::envelope::OutputEnvelope;
use emu::constants::research::{ERR_TIMEOUT, EXIT_CANCELLED, EXIT_TIMEOUT};
use emu::models::error::ErrorRecord;
use emu::models::research::{OperationRecord, OperationStatus};

#[test]
fn test_caller_timeout_reports_continuing_task_without_stopping() {
    let mut op = OperationRecord::new(
        "op_longtask123",
        "baseline_restore",
        Some("guest456".to_string()),
        None,
    );
    op.start("executing").expect("start");

    // When a caller's wait deadline expires:
    // 1. Task remains in executing state in the background
    assert_eq!(op.status, OperationStatus::Executing);
    assert!(!op.is_terminal());

    // 2. Output envelope reflects timeout outcome with exit code 124
    let err = ErrorRecord::new(ERR_TIMEOUT, "Caller wait timeout expired", None);
    let env = OutputEnvelope::timed_out(
        "op_wait01",
        err,
        Some(serde_json::json!({
            "operation_id": op.operation_id,
            "actual_state": "continuing",
            "progress_percent": op.progress_percent,
        })),
    );

    assert_eq!(env.outcome, "timeout");
    assert_eq!(EXIT_TIMEOUT, 124);
}

#[test]
fn test_cooperative_cancellation_confirmed_at_safe_boundary() {
    let mut op = OperationRecord::new(
        "op_cancelme",
        "disk_wipe",
        Some("guest789".to_string()),
        None,
    );
    op.start("executing").expect("start");

    // Step 1: Cancellation requested
    op.request_cancellation().expect("cancel request");
    assert_eq!(op.status, OperationStatus::CancellationPending);
    assert!(!op.is_terminal());

    // Step 2: Worker acknowledges and cleanly halts at safe boundary
    op.acknowledge_cancellation().expect("acknowledge cancel");
    assert_eq!(op.status, OperationStatus::Cancelled);
    assert!(op.is_terminal());

    // Step 3: Cancellation envelope outcome and exit code 130
    let err = ErrorRecord::new(
        emu::constants::research::ERR_CANCELLED,
        "Operation cleanly cancelled at safe boundary",
        None,
    );
    let env = OutputEnvelope::cancelled(
        "op_diskwipe01",
        err,
        Some(serde_json::json!({
            "operation_id": op.operation_id,
            "status": "cancelled",
        })),
    );

    assert_eq!(env.outcome, "cancelled");
    assert_eq!(EXIT_CANCELLED, 130);
}
