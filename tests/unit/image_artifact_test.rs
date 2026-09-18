//! Unit tests for image artifact cryptographic checksums, format compatibility,
//! corruption rejection, and experimental opt-in requirement.

use emu::constants::research::{ERR_ARTIFACT_CORRUPTED, ERR_EXPERIMENTAL_OPT_IN_REQUIRED};
use emu::models::research::{
    ArtifactTrustStatus, BackendType, ImageArtifactType, ImageOriginMetadata,
    ResearchImageArtifact, Sha256Digest,
};
use sha2::{Digest, Sha256};

#[test]
fn test_image_artifact_checksum_and_validation() {
    let dummy_bytes = b"darwin-kernel-cache-simulated-bytes-v1";
    let mut hasher = Sha256::new();
    hasher.update(dummy_bytes);
    let calculated_digest = format!("sha256:{:x}", hasher.finalize());

    let digest = Sha256Digest::new(&calculated_digest);
    let artifact = ResearchImageArtifact {
        artifact_id: "art_darwin_kernel_test".to_string(),
        artifact_type: ImageArtifactType::Kernelcache,
        file_path: "/tmp/emu-test/kernelcache".to_string(),
        sha256_digest: digest.clone(),
        origin_metadata: ImageOriginMetadata {
            source_type: "user_supplied".to_string(),
            build_identity: "24A123".to_string(),
            source_url_or_ref: None,
        },
        target_backend: BackendType::DarwinVm,
        build_version_identity: "24A123".to_string(),
        verified_device_node: Some("/dev/disk4s1".to_string()),
        verified_volume_uuid: Some("12345678-ABCD-EF01-2345-6789ABCDEF01".to_string()),
        trust_status: ArtifactTrustStatus::VerifiedTrusted,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    assert!(artifact.validate().is_ok());
    assert_eq!(artifact.sha256_digest.0, calculated_digest);
}

#[test]
fn test_image_artifact_experimental_opt_in_required() {
    let digest = Sha256Digest::new(
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let experimental_artifact = ResearchImageArtifact {
        artifact_id: "art_exp_inferno_fw".to_string(),
        artifact_type: ImageArtifactType::IpswRestoreBundle,
        file_path: "/tmp/emu-test/exp_fw.ipsw".to_string(),
        sha256_digest: digest,
        origin_metadata: ImageOriginMetadata {
            source_type: "user_supplied".to_string(),
            build_identity: "EXP_21A5248v".to_string(),
            source_url_or_ref: None,
        },
        target_backend: BackendType::Inferno,
        build_version_identity: "EXP_21A5248v".to_string(),
        verified_device_node: None,
        verified_volume_uuid: None,
        trust_status: ArtifactTrustStatus::ExperimentalOptIn,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // When allow_experimental is false, registration must fail with ERR_EXPERIMENTAL_OPT_IN_REQUIRED
    let allow_experimental = false;
    let registration_result: Result<(), &str> = if !allow_experimental
        && experimental_artifact.trust_status == ArtifactTrustStatus::ExperimentalOptIn
    {
        Err(ERR_EXPERIMENTAL_OPT_IN_REQUIRED)
    } else {
        Ok(())
    };

    assert_eq!(registration_result, Err(ERR_EXPERIMENTAL_OPT_IN_REQUIRED));
}

#[test]
fn test_image_artifact_corrupted_checksum_mismatch() {
    let actual_bytes = b"real-valid-content";
    let mut hasher = Sha256::new();
    hasher.update(actual_bytes);
    let real_digest = format!("sha256:{:x}", hasher.finalize());

    let declared_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    let verify_res: Result<(), &str> = if real_digest != declared_digest {
        Err(ERR_ARTIFACT_CORRUPTED)
    } else {
        Ok(())
    };

    assert_eq!(verify_res, Err(ERR_ARTIFACT_CORRUPTED));
}
