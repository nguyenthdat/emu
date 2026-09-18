//! Integration tests for dual-backend lifecycle management and display name disambiguation.

use emu::models::research::{
    BackendType, BootArtifactMap, CpuArchitecture, InstanceLifecycleState, PrivilegeState,
    ResearchGuestId, ResearchGuestInstance, RootVerificationState, Sha256Digest,
};
use emu::persistence::instances::{delete_instance, list_instances, load_instance, save_instance};
use emu::persistence::paths::ResearchPaths;
use std::path::PathBuf;

#[tokio::test]
async fn test_dual_backend_identical_display_names_and_isolation() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = ResearchPaths::new(temp.path().to_path_buf()).expect("paths");
    paths.ensure().await.expect("ensure");

    let boot_artifacts = BootArtifactMap::new(
        Sha256Digest::new(
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        ),
        Sha256Digest::new(
            "sha256:2222222222222222222222222222222222222222222222222222222222222222",
        ),
    );

    // 1. Create darwin-vm instance named "ios-sec-lab"
    let darwin_id = ResearchGuestId::new();
    let darwin_guest = ResearchGuestInstance::new(
        darwin_id,
        "ios-sec-lab",
        BackendType::DarwinVm,
        InstanceLifecycleState::Stopped,
        CpuArchitecture::Arm64,
        "Darwin 24.0.0",
        "24A335",
        Sha256Digest::new(
            "sha256:3333333333333333333333333333333333333333333333333333333333333333",
        ),
        Sha256Digest::new(
            "sha256:4444444444444444444444444444444444444444444444444444444444444444",
        ),
        boot_artifacts.clone(),
        PathBuf::from("/tmp/emu-darwin01"),
        PrivilegeState::Root,
        RootVerificationState::Unverified,
    );
    save_instance(&paths, &darwin_guest)
        .await
        .expect("Failed to save darwin guest");

    // 2. Create Inferno instance with identical display name "ios-sec-lab"
    let inferno_id = ResearchGuestId::new();
    let inferno_guest = ResearchGuestInstance::new(
        inferno_id,
        "ios-sec-lab",
        BackendType::Inferno,
        InstanceLifecycleState::Stopped,
        CpuArchitecture::Arm64,
        "iOS 14.0",
        "18A5351d",
        Sha256Digest::new(
            "sha256:5555555555555555555555555555555555555555555555555555555555555555",
        ),
        Sha256Digest::new(
            "sha256:6666666666666666666666666666666666666666666666666666666666666666",
        ),
        boot_artifacts,
        PathBuf::from("/tmp/emu-inferno1"),
        PrivilegeState::Root,
        RootVerificationState::Unverified,
    );
    save_instance(&paths, &inferno_guest)
        .await
        .expect("Failed to save inferno guest");

    // 3. Verify distinct IDs and list returns both
    assert_ne!(darwin_id, inferno_id);
    let all = list_instances(&paths).await.expect("Failed to list");
    assert_eq!(all.len(), 2);

    // 4. Independent lifecycle: mutate darwin instance to Running
    let mut darwin_mut = load_instance(&paths, &darwin_id.to_string())
        .await
        .unwrap()
        .unwrap();
    darwin_mut.lifecycle_state = InstanceLifecycleState::Running;
    save_instance(&paths, &darwin_mut).await.unwrap();

    // Verify Inferno instance is still Stopped
    let inferno_check = load_instance(&paths, &inferno_id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        inferno_check.lifecycle_state,
        InstanceLifecycleState::Stopped
    );

    // 5. Clean deletion
    let deleted = delete_instance(&paths, &darwin_id.to_string())
        .await
        .unwrap();
    assert!(deleted);
    let remaining = list_instances(&paths).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, inferno_id);
}
