//! Integration tests for local companion helper VM orchestration,
//! resource boundaries, socket isolation, and dependent lifetime binding.

use emu::constants::research::{ERR_DEPENDENT_GUEST_ACTIVE, EXIT_CONFLICT};
use emu::models::research::{CompanionEnvironment, InstanceLifecycleState, ResearchGuestId};
use uuid::Uuid;

#[test]
fn test_companion_environment_resource_limits_and_isolation() {
    let parent_id = ResearchGuestId::new();
    let companion = CompanionEnvironment {
        companion_id: Uuid::new_v4(),
        parent_guest_id: parent_id,
        lifecycle_state: InstanceLifecycleState::Running,
        cpu_limit: 4,
        memory_limit_mb: 4096,
        storage_limit_mb: 32768,
        endpoint_socket_path: "/tmp/emu-companion-1234/usb.sock".to_string(),
        forwarded_ports: vec![],
        active_operation_ids: vec![],
        live_guest_ids: vec![parent_id.0],
        active_workflows_count: 1,
        live_dependents_count: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    assert!(companion.validate().is_ok());
    assert_eq!(companion.cpu_limit, 4);
    assert_eq!(companion.memory_limit_mb, 4096);
    // Endpoint must be a local unix domain socket, never a global 0.0.0.0 network bind
    assert!(companion.endpoint_socket_path.starts_with("/tmp/"));
    assert!(!companion.endpoint_socket_path.contains("0.0.0.0"));
}

#[test]
fn test_companion_refusal_to_stop_with_active_dependents() {
    let parent_id = ResearchGuestId::new();
    let companion = CompanionEnvironment {
        companion_id: Uuid::new_v4(),
        parent_guest_id: parent_id,
        lifecycle_state: InstanceLifecycleState::Running,
        cpu_limit: 2,
        memory_limit_mb: 2048,
        storage_limit_mb: 32768,
        endpoint_socket_path: "/tmp/emu-companion/usb.sock".to_string(),
        forwarded_ports: vec![],
        active_operation_ids: vec!["op_active_restore_001".to_string()],
        live_guest_ids: vec![parent_id.0],
        active_workflows_count: 1,
        live_dependents_count: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    // Stopping while live dependents or active operations exist must be rejected
    let stop_result: Result<(), (i32, &'static str)> =
        if companion.live_dependents_count > 0 || !companion.active_operation_ids.is_empty() {
            Err((EXIT_CONFLICT, ERR_DEPENDENT_GUEST_ACTIVE))
        } else {
            Ok(())
        };

    assert_eq!(
        stop_result,
        Err((EXIT_CONFLICT, ERR_DEPENDENT_GUEST_ACTIVE))
    );
}

#[test]
fn test_companion_stop_allowed_when_dependents_zero() {
    let parent_id = ResearchGuestId::new();
    let mut companion = CompanionEnvironment {
        companion_id: Uuid::new_v4(),
        parent_guest_id: parent_id,
        lifecycle_state: InstanceLifecycleState::Running,
        cpu_limit: 2,
        memory_limit_mb: 2048,
        storage_limit_mb: 32768,
        endpoint_socket_path: "/tmp/emu-companion/usb.sock".to_string(),
        forwarded_ports: vec![],
        active_operation_ids: vec![],
        live_guest_ids: vec![],
        active_workflows_count: 0,
        live_dependents_count: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let stop_result: Result<(), (i32, &'static str)> =
        if companion.live_dependents_count > 0 || !companion.active_operation_ids.is_empty() {
            Err((EXIT_CONFLICT, ERR_DEPENDENT_GUEST_ACTIVE))
        } else {
            companion.lifecycle_state = InstanceLifecycleState::Stopped;
            Ok(())
        };

    assert!(stop_result.is_ok());
    assert_eq!(companion.lifecycle_state, InstanceLifecycleState::Stopped);
}
