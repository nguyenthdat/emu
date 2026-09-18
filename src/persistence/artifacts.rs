//! Image artifact catalog repository persistence.
//!
//! Defined in accordance with data-model.md §2.4:
//! Manages image and firmware artifact metadata with advisory locks, validation,
//! and identity binding.

use super::atomic::write_atomic;
use super::lock::AdvisoryLock;
use super::paths::{ResearchPaths, validate_path_component};
use crate::models::research::ResearchImageArtifact;
use anyhow::{Context, Result, bail};

fn sync_parent_dir(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if let Ok(dir_file) = std::fs::File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }
    Ok(())
}

fn artifact_lock_path(paths: &ResearchPaths, digest: &str) -> Result<std::path::PathBuf> {
    let safe_digest = digest.replace(':', "_");
    validate_path_component(&safe_digest)?;
    Ok(paths
        .operation_locks()
        .join(format!("{safe_digest}.artifact.lock")))
}

pub async fn save_artifact(paths: &ResearchPaths, artifact: &ResearchImageArtifact) -> Result<()> {
    artifact.validate().map_err(|e| {
        anyhow::anyhow!("ResearchImageArtifact validation failed prior to saving: {e}")
    })?;

    let lock_path = artifact_lock_path(paths, artifact.sha256_digest.as_str())?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| {
            format!(
                "Failed to acquire lock for artifact '{}'",
                artifact.sha256_digest.as_str()
            )
        })?;

    let path = paths.artifact_path(artifact.sha256_digest.as_str())?;
    let serialized =
        serde_json::to_vec_pretty(artifact).context("Failed to serialize ResearchImageArtifact")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(())
}

pub async fn update_artifact_with_lock<F>(
    paths: &ResearchPaths,
    digest: &str,
    mutator: F,
) -> Result<ResearchImageArtifact>
where
    F: FnOnce(&mut ResearchImageArtifact) -> Result<()>,
{
    let lock_path = artifact_lock_path(paths, digest)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| format!("Failed to acquire lock for artifact '{digest}'"))?;

    let mut art = match load_artifact(paths, digest).await? {
        Some(a) => a,
        None => bail!("Artifact '{digest}' not found for update"),
    };

    mutator(&mut art)?;
    art.validate()
        .map_err(|e| anyhow::anyhow!("Mutated artifact '{digest}' failed validation: {e}"))?;

    let path = paths.artifact_path(digest)?;
    let serialized =
        serde_json::to_vec_pretty(&art).context("Failed to serialize ResearchImageArtifact")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(art)
}

pub async fn load_artifact(
    paths: &ResearchPaths,
    digest: &str,
) -> Result<Option<ResearchImageArtifact>> {
    let path = paths.artifact_path(digest)?;
    let data = match tokio::fs::read(&path).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Failed to read artifact at '{}'", path.display()));
        }
    };

    let art: ResearchImageArtifact = serde_json::from_slice(&data)
        .with_context(|| format!("Failed to parse artifact from '{}'", path.display()))?;

    art.validate().map_err(|e| {
        anyhow::anyhow!(
            "Loaded artifact at '{}' failed validation: {e}",
            path.display()
        )
    })?;

    if art.sha256_digest.as_str() != digest {
        bail!(
            "Identity binding mismatch for artifact at '{}': expected digest '{}', found '{}'",
            path.display(),
            digest,
            art.sha256_digest.as_str()
        );
    }

    Ok(Some(art))
}

pub async fn list_artifacts(paths: &ResearchPaths) -> Result<Vec<ResearchImageArtifact>> {
    let mut entries = tokio::fs::read_dir(paths.artifacts()).await?;
    let mut artifacts = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let file_type = entry
            .file_type()
            .await
            .with_context(|| format!("Failed to stat entry '{}'", entry.path().display()))?;

        if file_type.is_file() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                let art = match load_artifact(paths, stem).await? {
                    Some(a) => a,
                    None => continue,
                };
                artifacts.push(art);
            }
        }
    }

    artifacts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(artifacts)
}
