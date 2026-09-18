//! Laboratory test harness configuration and cohort definitions.

use emu::models::research::BackendType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabCohortConfig {
    pub cohort_name: String,
    pub frozen_baseline_version: String,
    pub guests: Vec<LabGuestConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabGuestConfig {
    pub id: String,
    pub display_name: String,
    pub backend: BackendType,
    pub kernelcache_artifact: String,
    pub devicetree_artifact: String,
    pub root_disk_artifact: Option<String>,
    pub ramdisk_artifact: Option<String>,
    pub max_boot_time_ms: u64,
}

impl Default for LabCohortConfig {
    fn default() -> Self {
        Self {
            cohort_name: "Frozen-4-Guest-Reference-Cohort".to_string(),
            frozen_baseline_version: "1.0.0".to_string(),
            guests: vec![
                LabGuestConfig {
                    id: "darwin-guest-1".to_string(),
                    display_name: "ios-sec-lab".to_string(),
                    backend: BackendType::DarwinVm,
                    kernelcache_artifact: "darwin_bootkc_minimal.bin".to_string(),
                    devicetree_artifact: "darwin_dtree_minimal.dtb".to_string(),
                    root_disk_artifact: None,
                    ramdisk_artifact: Some("darwin_root_ramdisk.img".to_string()),
                    max_boot_time_ms: 10_000,
                },
                LabGuestConfig {
                    id: "darwin-guest-2".to_string(),
                    display_name: "darwin-secondary".to_string(),
                    backend: BackendType::DarwinVm,
                    kernelcache_artifact: "darwin_bootkc_minimal.bin".to_string(),
                    devicetree_artifact: "darwin_dtree_minimal.dtb".to_string(),
                    root_disk_artifact: None,
                    ramdisk_artifact: Some("darwin_root_ramdisk.img".to_string()),
                    max_boot_time_ms: 10_000,
                },
                LabGuestConfig {
                    id: "inferno-guest-1".to_string(),
                    display_name: "ios-sec-lab".to_string(),
                    backend: BackendType::Inferno,
                    kernelcache_artifact: "inferno_kernelcache_18A5351d".to_string(),
                    devicetree_artifact: "inferno_dtree_n104ap.dtb".to_string(),
                    root_disk_artifact: Some("inferno_rootfs_18A5351d.raw".to_string()),
                    ramdisk_artifact: None,
                    max_boot_time_ms: 30_000,
                },
                LabGuestConfig {
                    id: "inferno-guest-2".to_string(),
                    display_name: "inferno-secondary".to_string(),
                    backend: BackendType::Inferno,
                    kernelcache_artifact: "inferno_kernelcache_18A5351d".to_string(),
                    devicetree_artifact: "inferno_dtree_n104ap.dtb".to_string(),
                    root_disk_artifact: Some("inferno_rootfs_18A5351d.raw".to_string()),
                    ramdisk_artifact: None,
                    max_boot_time_ms: 30_000,
                },
            ],
        }
    }
}
