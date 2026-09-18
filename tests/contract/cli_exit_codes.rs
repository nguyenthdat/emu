//! Contract tests for standardized CLI exit codes and canonical administrative-refusal normalization.

use emu::constants::research::*;
use emu::models::error::ErrorRecord;

#[test]
fn test_exit_code_constants() {
    assert_eq!(EXIT_SUCCESS, 0);
    assert_eq!(EXIT_RUNTIME_FAILURE, 1);
    assert_eq!(EXIT_INVALID_INPUT, 2);
    assert_eq!(EXIT_UNSUPPORTED, 3);
    assert_eq!(EXIT_AUTH_REFUSED, 4);
    assert_eq!(EXIT_CONFLICT, 5);
    assert_eq!(EXIT_TIMEOUT, 124);
    assert_eq!(EXIT_CANCELLED, 130);
}

#[test]
fn test_standardized_error_code_exit_mappings() {
    let cases = vec![
        (ERR_AUTH_REQUIRED, EXIT_AUTH_REFUSED),
        (ERR_INVALID_INPUT, EXIT_INVALID_INPUT),
        (ERR_AMBIGUOUS_INSTANCE_NAME, EXIT_INVALID_INPUT),
        (ERR_ARTIFACT_CORRUPTED, EXIT_INVALID_INPUT),
        (ERR_EXPERIMENTAL_OPT_IN_REQUIRED, EXIT_INVALID_INPUT),
        (ERR_MISSING_BASELINE, EXIT_INVALID_INPUT),
        (ERR_UNSUPPORTED_HOST, EXIT_UNSUPPORTED),
        (ERR_APP_FRAMEWORKS_UNAVAILABLE, EXIT_UNSUPPORTED),
        (ERR_CONCURRENCY_CONFLICT, EXIT_CONFLICT),
        (ERR_DEPENDENT_GUEST_ACTIVE, EXIT_CONFLICT),
        (ERR_DEBUG_LEASE_CONFLICT, EXIT_CONFLICT),
        (ERR_TIMEOUT, EXIT_TIMEOUT),
        (ERR_CANCELLATION_PENDING, EXIT_TIMEOUT),
        (ERR_CANCELLED, EXIT_CANCELLED),
        (ERR_PROBE_VERIFICATION_FAILED, EXIT_RUNTIME_FAILURE),
        (ERR_UNSAFE_MOUNT_DETECTED, EXIT_RUNTIME_FAILURE),
        (ERR_RUNTIME_EXECUTION_ERROR, EXIT_RUNTIME_FAILURE),
    ];
    for (code, expected_exit) in cases {
        let err = ErrorRecord::new(code, "Test error message", None);
        assert_eq!(
            err.exit_code(),
            expected_exit,
            "Error code {code} must map to exit code {expected_exit}"
        );
    }
}

#[test]
fn test_administrative_refusal_normalization() {
    // Canonical Exit Code 4 for unauthorized unattended elevation
    let auth_err = ErrorRecord::new(
        ERR_AUTH_REQUIRED,
        "Elevated host preparation requires interactive sudo credentials or pre-authorized token",
        Some(serde_json::json!({
            "operation": "image prepare",
            "unattended": true
        })),
    );
    assert_eq!(auth_err.exit_code(), EXIT_AUTH_REFUSED);
    assert_eq!(auth_err.exit_code(), 4);

    // Contrasted with Exit Code 2 for invalid syntax
    let syntax_err = ErrorRecord::new(
        ERR_INVALID_INPUT,
        "Missing required parameter --source",
        None,
    );
    assert_eq!(syntax_err.exit_code(), EXIT_INVALID_INPUT);
    assert_eq!(syntax_err.exit_code(), 2);
}

#[test]
fn test_unknown_error_code_fails_validation_closed() {
    let unknown_err = ErrorRecord::new("UNKNOWN_CUSTOM_ERROR", "Unrecognized failure", None);
    assert!(unknown_err.validate().is_err());
}
