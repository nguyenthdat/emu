//! Research image artifact and boot artifact mapping models.
//!
//! Defined in accordance with contracts/research.schema.json.

use super::types::{
    ArtifactTrustStatus, BackendType, ImageArtifactType, ImageOriginMetadata, Sha256Digest,
};
use serde::{Deserialize, Serialize};

/// Map of cryptographic SHA-256 digests for all boot-critical artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootArtifactMap {
    pub kernelcache: Sha256Digest,
    pub devicetree: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trustcache: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ramdisk: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_disk: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sptm_firmware: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txm_firmware: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sep_firmware: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nvram_template: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipsw_restore_bundle: Option<Sha256Digest>,
}

impl BootArtifactMap {
    pub fn new(kernelcache: Sha256Digest, devicetree: Sha256Digest) -> Self {
        Self {
            kernelcache,
            devicetree,
            trustcache: None,
            ramdisk: None,
            root_disk: None,
            sptm_firmware: None,
            txm_firmware: None,
            sep_firmware: None,
            nvram_template: None,
            ipsw_restore_bundle: None,
        }
    }

    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        self.kernelcache.validate()?;
        self.devicetree.validate()?;
        if let Some(d) = &self.trustcache {
            d.validate()?;
        }
        if let Some(d) = &self.ramdisk {
            d.validate()?;
        }
        if let Some(d) = &self.root_disk {
            d.validate()?;
        }
        if let Some(d) = &self.sptm_firmware {
            d.validate()?;
        }
        if let Some(d) = &self.txm_firmware {
            d.validate()?;
        }
        if let Some(d) = &self.sep_firmware {
            d.validate()?;
        }
        if let Some(d) = &self.nvram_template {
            d.validate()?;
        }
        if let Some(d) = &self.ipsw_restore_bundle {
            d.validate()?;
        }
        Ok(())
    }
}

/// Metadata and validation record for a research firmware or disk image artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchImageArtifact {
    pub artifact_id: String,
    pub artifact_type: ImageArtifactType,
    pub file_path: String,
    pub sha256_digest: Sha256Digest,
    pub origin_metadata: ImageOriginMetadata,
    pub target_backend: BackendType,
    pub build_version_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_device_node: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_volume_uuid: Option<String>,
    pub trust_status: ArtifactTrustStatus,
    pub created_at: String,
}

impl ResearchImageArtifact {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        self.sha256_digest.validate()?;
        self.origin_metadata.validate()?;
        if chrono::DateTime::parse_from_rfc3339(&self.created_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "created_at".to_string(),
                value: self.created_at.clone(),
            });
        }
        Ok(())
    }
}
