//! Integration tests for guest system daemon tracing on Inferno and Mach service enumeration.

use emu::services::research::GuestFileSystem;

#[tokio::test]
async fn test_system_daemon_mach_services_enumeration() {
    let services = GuestFileSystem::list_mach_services()
        .await
        .expect("list_mach_services failed");

    assert!(services.contains(&"com.apple.SpringBoard".to_string()));
    assert!(services.contains(&"com.apple.launchd".to_string()));
    assert!(services.contains(&"com.apple.securityd".to_string()));
}

#[tokio::test]
async fn test_in_guest_filesystem_operations_without_host_mount() {
    let guest_path = "/private/var/mobile/Library/Preferences/com.example.research.plist";
    let content = b"<plist version=\"1.0\"><dict><key>Test</key><true/></dict></plist>";

    // Direct in-guest write without host mount
    GuestFileSystem::write_path(guest_path, content)
        .await
        .expect("write_path failed");

    // Direct in-guest read without host mount
    let read_bytes = GuestFileSystem::read_path(guest_path)
        .await
        .expect("read_path failed");
    assert!(!read_bytes.is_empty());
}
