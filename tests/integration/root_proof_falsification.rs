//! Integration tests for root proof verification, negative control falsification,
//! and boot-session invalidation.

use emu::models::research::{
    BackendType, BootArtifactMap, BootSessionId, CpuArchitecture, InstanceLifecycleState,
    PrivilegeState, ResearchGuestId, ResearchGuestInstance, RootVerificationState, Sha256Digest,
};
use emu::persistence::instances::{load_instance, save_instance};
use emu::persistence::paths::ResearchPaths;
use emu::services::research::RootVerifier;
use std::path::PathBuf;

#[tokio::test]
async fn test_root_proof_positive_verification_and_negative_control() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = ResearchPaths::new(temp.path().to_path_buf()).expect("paths");
    paths.ensure().await.expect("ensure");

    let guest_id = ResearchGuestId::new();
    let mut guest = ResearchGuestInstance::new(
        guest_id,
        "ios-sec-lab",
        BackendType::DarwinVm,
        InstanceLifecycleState::Running,
        CpuArchitecture::Arm64,
        "Darwin 24.0.0",
        "darwin_bootkc",
        Sha256Digest::new(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ),
        Sha256Digest::new(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ),
        BootArtifactMap::new(
            Sha256Digest::new(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            ),
            Sha256Digest::new(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ),
        PathBuf::from(format!("/tmp/emu-{}", &guest_id.to_string()[..8])),
        PrivilegeState::Root,
        RootVerificationState::Unverified,
    );
    save_instance(&paths, &guest)
        .await
        .expect("Failed to save guest");

    let verifier = RootVerifier::new(&paths);
    let boot_session_1 = BootSessionId::new();

    // 1. Positive root verification succeeds when negative control is denied
    let result = verifier
        .verify_guest(&mut guest, boot_session_1, false)
        .await
        .expect("Verification execution failed");
    let evidence = result.expect("Positive probe must pass");

    assert_eq!(evidence.verification_state, RootVerificationState::Verified);
    assert_eq!(evidence.verified_uid, Some(0));
    assert_eq!(guest.observed_privilege, RootVerificationState::Verified);

    // 2. Simulated unexpected negative control pass triggers falsification
    let boot_session_2 = BootSessionId::new();
    let leak_result = verifier
        .verify_guest(&mut guest, boot_session_2, true)
        .await
        .expect("Verification execution failed");
    assert!(
        leak_result.is_err(),
        "Falsification gate must reject unexpected unprivileged success"
    );
    assert_eq!(guest.observed_privilege, RootVerificationState::Unverified);

    // Verify persisted state is unverified
    let reloaded = load_instance(&paths, &guest_id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        reloaded.observed_privilege,
        RootVerificationState::Unverified
    );
}
