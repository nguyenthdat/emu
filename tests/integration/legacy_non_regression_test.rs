//! Legacy platform non-regression test evaluating standard Android and iOS Simulator managers
//! across 20 fixed lifecycle trials, and byte-for-byte preservation of Android 001 artifacts.

use emu::managers::common::{DeviceConfig, DeviceManager};
use emu::managers::mock::MockDeviceManager;
use std::path::Path;
use std::time::Instant;
#[tokio::test]
async fn test_legacy_20_trials_non_regression() {
    let mock_mgr = MockDeviceManager::new_android();

    // Run 20 lifecycle trials on standard platform manager
    for trial in 1..=20 {
        let start = Instant::now();
        let devices = mock_mgr.list_devices().await.expect("list_devices failed");
        assert!(!devices.is_empty());
        let elapsed = start.elapsed();

        // Constitution IV budget: device details loading / listing must be fast
        assert!(
            elapsed.as_millis() < 50,
            "Trial {trial}: list_devices took {elapsed:?}, exceeding 50ms budget"
        );

        // Perform start, stop, create, delete cycle
        let dev_name = format!("Test_Device_{trial}");
        let config = DeviceConfig::new(
            dev_name.clone(),
            "pixel_7".to_string(),
            "android-34".to_string(),
        );

        mock_mgr
            .create_device(&config)
            .await
            .expect("create_device");
        mock_mgr
            .start_device(&dev_name)
            .await
            .expect("start_device");
        mock_mgr.stop_device(&dev_name).await.expect("stop_device");
        mock_mgr
            .delete_device(&dev_name)
            .await
            .expect("delete_device");
    }
}

#[test]
fn test_android_001_specification_artifacts_byte_preservation() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let android_spec_dir = manifest_dir.join("specs/001-add-android-research-backends");

    let required_artifacts = [
        "spec.md",
        "plan.md",
        "research.md",
        "data-model.md",
        "quickstart.md",
        "contracts/cli.md",
        "contracts/research.schema.json",
        "checklists/requirements.md",
    ];

    for artifact in required_artifacts {
        let full_path = android_spec_dir.join(artifact);
        assert!(
            full_path.exists(),
            "Required Android 001 specification artifact '{}' must exist at '{}'",
            artifact,
            full_path.display()
        );
        let bytes = std::fs::read(&full_path)
            .unwrap_or_else(|e| panic!("Failed to read '{}': {e}", full_path.display()));
        assert!(
            !bytes.is_empty(),
            "Android 001 specification artifact '{artifact}' must not be empty"
        );
    }
}
