//! Integration tests for owned iOS application lifecycle, Frida 17.18.0 dynamic hooks,
//! target specificity, and authorized container export.

use emu::cli::{app, tool};
use emu::constants::research::{EXIT_AUTH_REFUSED, EXIT_SUCCESS};
use emu::models::research::{
    BackendType, BootArtifactMap, CpuArchitecture, InstanceLifecycleState, PrivilegeState,
    ResearchGuestId, ResearchGuestInstance, RootVerificationState, Sha256Digest,
};
use emu::persistence::instances::save_instance;
use emu::persistence::paths::ResearchPaths;
use std::path::PathBuf;

#[tokio::test]
async fn test_inferno_app_lifecycle_and_frida_hooks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = ResearchPaths::new(temp.path().to_path_buf()).expect("paths");
    paths.ensure().await.expect("ensure");

    let inferno_id = ResearchGuestId::new();
    let guest = ResearchGuestInstance::new(
        inferno_id,
        "inferno-research",
        BackendType::Inferno,
        InstanceLifecycleState::Running,
        CpuArchitecture::Arm64,
        "iOS 14.0",
        "18A5351d",
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
        PathBuf::from(format!("/tmp/emu-{}", &inferno_id.to_string()[..8])),
        PrivilegeState::Root,
        RootVerificationState::Unverified,
    );
    save_instance(&paths, &guest).await.expect("save_instance");

    // 1. App Installation succeeds on Inferno
    let install_exit = app::install(
        &paths,
        Some(&inferno_id.to_string()),
        None,
        None,
        "/tmp/SampleResearchApp.ipa",
        false,
    )
    .await
    .expect("install failed");
    assert_eq!(install_exit, EXIT_SUCCESS);

    // 2. App List succeeds
    let list_exit = app::list(&paths, Some(&inferno_id.to_string()), None, None, false)
        .await
        .expect("list failed");
    assert_eq!(list_exit, EXIT_SUCCESS);

    // 3. Frida Attach hooks native and Objective-C methods
    let attach_exit = tool::frida_attach(
        &paths,
        Some(&inferno_id.to_string()),
        None,
        None,
        Some(1234),
        Some("Interceptor.attach(...)"),
        false,
    )
    .await
    .expect("frida attach failed");
    assert_eq!(attach_exit, EXIT_SUCCESS);

    // 4. Frida Detach leaves target process running
    let detach_exit = tool::frida_detach(&paths, Some(&inferno_id.to_string()), None, None, false)
        .await
        .expect("frida detach failed");
    assert_eq!(detach_exit, EXIT_SUCCESS);

    // 5. Container Export without --authorize-export is refused with Exit Code 4
    let export_unauth = app::container_export(
        &paths,
        Some(&inferno_id.to_string()),
        None,
        None,
        "com.example.researchapp",
        "/tmp/exported_container",
        false,
        false,
    )
    .await
    .expect("container_export failed");
    assert_eq!(export_unauth, EXIT_AUTH_REFUSED);

    // 6. Container Export with --authorize-export succeeds with Exit Code 0
    let export_auth = app::container_export(
        &paths,
        Some(&inferno_id.to_string()),
        None,
        None,
        "com.example.researchapp",
        "/tmp/exported_container",
        true,
        false,
    )
    .await
    .expect("container_export authorized failed");
    assert_eq!(export_auth, EXIT_SUCCESS);
}
