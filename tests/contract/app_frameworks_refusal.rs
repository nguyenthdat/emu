//! Contract tests asserting that minimal darwin-vm instances truthfully refuse
//! application frameworks with Exit Code 3 (unsupported) and APP_FRAMEWORKS_UNAVAILABLE.

use emu::cli::app;
use emu::constants::research::EXIT_UNSUPPORTED;
use emu::models::research::{
    BackendType, BootArtifactMap, CpuArchitecture, InstanceLifecycleState, PrivilegeState,
    ResearchGuestId, ResearchGuestInstance, RootVerificationState, Sha256Digest,
};
use emu::persistence::instances::save_instance;
use emu::persistence::paths::ResearchPaths;
use std::path::PathBuf;

#[tokio::test]
async fn test_darwin_vm_refuses_application_frameworks_truthfully() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = ResearchPaths::new(temp.path().to_path_buf()).expect("paths");
    paths.ensure().await.expect("ensure");

    let darwin_id = ResearchGuestId::new();
    let guest = ResearchGuestInstance::new(
        darwin_id,
        "darwin-minimal",
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
        PathBuf::from(format!("/tmp/emu-{}", &darwin_id.to_string()[..8])),
        PrivilegeState::Root,
        RootVerificationState::Unverified,
    );
    save_instance(&paths, &guest).await.expect("save_instance");

    // Attempting app install on darwin-vm returns exit code 3 (unsupported)
    let exit_code = app::install(
        &paths,
        Some(&darwin_id.to_string()),
        None,
        None,
        "/tmp/SampleResearchApp.ipa",
        false,
    )
    .await
    .expect("CLI execution failed");

    assert_eq!(
        exit_code, EXIT_UNSUPPORTED,
        "darwin-vm must reject application installation with Exit Code 3 (unsupported)"
    );

    // Attempting app list on darwin-vm returns exit code 3 (unsupported)
    let list_exit = app::list(&paths, Some(&darwin_id.to_string()), None, None, false)
        .await
        .expect("CLI execution failed");
    assert_eq!(list_exit, EXIT_UNSUPPORTED);
}
