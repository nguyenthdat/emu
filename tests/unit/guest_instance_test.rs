//! Unit tests for ResearchGuestInstance, identifier formatting, and Darwin 104-byte socket limit.

use emu::constants::research::MAX_DARWIN_SUN_PATH;
use emu::models::research::{
    BackendType, BootArtifactMap, CpuArchitecture, InstanceLifecycleState, PrivilegeState,
    ResearchGuestId, ResearchGuestInstance, RootVerificationState, Sha256Digest,
};
use std::path::PathBuf;

#[test]
fn test_research_guest_id_uuidv4_uniqueness() {
    let id1 = ResearchGuestId::new();
    let id2 = ResearchGuestId::new();
    assert_ne!(id1, id2, "Generated IDs must be unique");
    assert_eq!(id1.as_uuid().get_version_num(), 4, "Must be UUIDv4");

    // String formatting
    let id_str = id1.to_string();
    assert_eq!(id_str.len(), 36);
    let parsed: ResearchGuestId = id_str.parse().expect("Failed to parse UUID");
    assert_eq!(id1, parsed);
}

#[test]
fn test_socket_path_darwin_sun_path_104_byte_limit() {
    let temp_uuid = uuid::Uuid::new_v4().simple().to_string();
    let short_uuid = &temp_uuid[..8];
    let runtime_dir = PathBuf::from(format!("/tmp/emu-{short_uuid}"));

    let sockets = [
        "qmp.sock",
        "gdb.sock",
        "console.sock",
        "supervisor.sock",
        "inferno-usb.sock",
    ];
    for sock_name in sockets {
        let full_path = runtime_dir.join(sock_name);
        let path_bytes = full_path.as_os_str().as_encoded_bytes();
        assert!(
            path_bytes.len() < MAX_DARWIN_SUN_PATH,
            "Socket path '{}' length {} exceeds Darwin limit {}",
            full_path.display(),
            path_bytes.len(),
            MAX_DARWIN_SUN_PATH
        );
    }
}

#[test]
fn test_guest_instance_serialization_roundtrip() {
    let guest = ResearchGuestInstance::new(
        ResearchGuestId::new(),
        "ios-sec-lab",
        BackendType::DarwinVm,
        InstanceLifecycleState::Stopped,
        CpuArchitecture::Arm64,
        "Darwin 24.0.0",
        "24A335",
        Sha256Digest::new(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        Sha256Digest::new(
            "sha256:fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        ),
        BootArtifactMap::new(
            Sha256Digest::new(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            Sha256Digest::new(
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
        ),
        PathBuf::from("/tmp/emu-12345678"),
        PrivilegeState::Root,
        RootVerificationState::Unverified,
    );

    let serialized = serde_json::to_string(&guest).expect("Serialization failed");
    let deserialized: ResearchGuestInstance =
        serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(guest.id, deserialized.id);
    assert_eq!(guest.display_name, deserialized.display_name);
    assert_eq!(guest.backend, deserialized.backend);
}
