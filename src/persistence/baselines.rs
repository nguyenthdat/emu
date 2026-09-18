//! Recovery baseline repository persistence.
//!
//! Defined in accordance with data-model.md §2.13:
//! Manages recovery baseline records under advisory locks with validation
//! and identity binding.

use super::atomic::write_atomic;
use super::lock::AdvisoryLock;
use super::paths::{ResearchPaths, validate_path_component};
use crate::models::research::RecoveryBaseline;
use anyhow::{Context, Result, bail};

fn sync_parent_dir(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if let Ok(dir_file) = std::fs::File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }
    Ok(())
}

fn baseline_lock_path(paths: &ResearchPaths, id: &str) -> Result<std::path::PathBuf> {
    let name = validate_path_component(id)?;
    Ok(paths
        .operation_locks()
        .join(format!("{name}.baseline.lock")))
}

pub async fn save_baseline(paths: &ResearchPaths, baseline: &RecoveryBaseline) -> Result<()> {
    baseline
        .validate()
        .map_err(|e| anyhow::anyhow!("RecoveryBaseline validation failed prior to saving: {e}"))?;

    let lock_path = baseline_lock_path(paths, &baseline.baseline_id)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| {
            format!(
                "Failed to acquire lock for baseline '{}'",
                baseline.baseline_id
            )
        })?;

    let path = paths.baseline_path(&baseline.baseline_id)?;
    let serialized =
        serde_json::to_vec_pretty(baseline).context("Failed to serialize RecoveryBaseline")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(())
}

pub async fn update_baseline_with_lock<F>(
    paths: &ResearchPaths,
    id: &str,
    mutator: F,
) -> Result<RecoveryBaseline>
where
    F: FnOnce(&mut RecoveryBaseline) -> Result<()>,
{
    let lock_path = baseline_lock_path(paths, id)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| format!("Failed to acquire lock for baseline '{id}'"))?;

    let mut base = match load_baseline(paths, id).await? {
        Some(b) => b,
        None => bail!("Baseline '{id}' not found for update"),
    };

    mutator(&mut base)?;
    base.validate()
        .map_err(|e| anyhow::anyhow!("Mutated baseline '{id}' failed validation: {e}"))?;

    let path = paths.baseline_path(id)?;
    let serialized =
        serde_json::to_vec_pretty(&base).context("Failed to serialize RecoveryBaseline")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(base)
}

pub async fn load_baseline(paths: &ResearchPaths, id: &str) -> Result<Option<RecoveryBaseline>> {
    let path = paths.baseline_path(id)?;
    let data = match tokio::fs::read(&path).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Failed to read baseline at '{}'", path.display()));
        }
    };

    let base: RecoveryBaseline = serde_json::from_slice(&data)
        .with_context(|| format!("Failed to parse baseline from '{}'", path.display()))?;

    base.validate().map_err(|e| {
        anyhow::anyhow!(
            "Loaded baseline at '{}' failed validation: {e}",
            path.display()
        )
    })?;

    if base.baseline_id != id {
        bail!(
            "Identity binding mismatch for baseline at '{}': expected ID '{}', found '{}'",
            path.display(),
            id,
            base.baseline_id
        );
    }

    Ok(Some(base))
}

pub async fn list_baselines(paths: &ResearchPaths) -> Result<Vec<RecoveryBaseline>> {
    let mut entries = tokio::fs::read_dir(paths.baselines()).await?;
    let mut baselines = Vec::new();

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
                let b = match load_baseline(paths, stem).await? {
                    Some(base) => base,
                    None => continue,
                };
                baselines.push(b);
            }
        }
    }

    baselines.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(baselines)
}
