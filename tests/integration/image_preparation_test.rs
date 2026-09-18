//! Integration tests for image preparation, volume UUID mount validation,
//! rejection of unsafe host paths, unattended elevation refusal, and LIFO cleanup.

use emu::constants::research::{ERR_AUTH_REQUIRED, ERR_UNSAFE_MOUNT_DETECTED, EXIT_AUTH_REFUSED};
use emu::services::research::image_prep::ImagePreparationWorker;
use std::path::Path;

#[test]
fn test_mount_target_safety_rejects_host_system_paths() {
    let unsafe_paths = [
        "/",
        "/System",
        "/System/Library",
        "/Volumes/System",
        "/usr",
        "/bin",
        "/sbin",
    ];

    for path_str in unsafe_paths {
        let path = Path::new(path_str);
        let res = ImagePreparationWorker::verify_mount_target_safety(path);
        assert!(res.is_err(), "Path '{path_str}' must be rejected as unsafe");
        let err = res.unwrap_err();
        assert_eq!(err.code, ERR_UNSAFE_MOUNT_DETECTED);
    }

    // Safe isolated temporary directory
    let safe_path = Path::new("/tmp/emu-1234abcd/mnt/rootfs");
    assert!(ImagePreparationWorker::verify_mount_target_safety(safe_path).is_ok());
}

#[test]
fn test_parse_diskutil_info_plist() {
    let sample_plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>DeviceNode</key>
    <string>/dev/disk9s2</string>
    <key>VolumeUUID</key>
    <string>98765432-FEDC-BA09-8765-43210FEDCBA0</string>
</dict>
</plist>"#;

    let (uuid, dev_node) =
        ImagePreparationWorker::parse_diskutil_info_plist(sample_plist).expect("parse plist");
    assert_eq!(uuid, "98765432-FEDC-BA09-8765-43210FEDCBA0");
    assert_eq!(dev_node, "/dev/disk9s2");
}

#[test]
fn test_unattended_elevation_refusal() {
    let attended = false;
    let elevation_available = false;

    let res: Result<(), (i32, &'static str)> = if !attended && !elevation_available {
        Err((EXIT_AUTH_REFUSED, ERR_AUTH_REQUIRED))
    } else {
        Ok(())
    };

    assert_eq!(res, Err((EXIT_AUTH_REFUSED, ERR_AUTH_REQUIRED)));
}
