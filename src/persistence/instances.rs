//! Research guest instance repository persistence.
//!
//! Defined in accordance with data-model.md §2.2 and §1.2:
//! Manages guest instance state records under exclusive device locks with validation,
//! identity binding, and explicit error propagation.

use super::atomic::write_atomic;
use super::lock::AdvisoryLock;
use super::paths::ResearchPaths;
use crate::models::research::ResearchGuestInstance;
use anyhow::{Context, Result, bail};

fn sync_parent_dir(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if let Ok(dir_file) = std::fs::File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }
    Ok(())
}

/// Persists a validated research guest instance under its permanent device advisory lock.
pub async fn save_instance(paths: &ResearchPaths, instance: &ResearchGuestInstance) -> Result<()> {
    instance.validate().map_err(|e| {
        anyhow::anyhow!("ResearchGuestInstance validation failed prior to saving: {e}")
    })?;

    let id_str = instance.id.to_string();
    let lock_path = paths.instance_device_lock_path(&id_str)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| format!("Failed to acquire device lock for instance '{id_str}'"))?;

    save_instance_unlocked(paths, instance).await
}

/// Persists an instance record when the device lock is already held.
pub async fn save_instance_unlocked(
    paths: &ResearchPaths,
    instance: &ResearchGuestInstance,
) -> Result<()> {
    instance.validate().map_err(|e| {
        anyhow::anyhow!("ResearchGuestInstance validation failed prior to saving: {e}")
    })?;

    let path = paths.instance_path(&instance.id.to_string())?;
    let serialized =
        serde_json::to_vec_pretty(instance).context("Failed to serialize ResearchGuestInstance")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(())
}

/// Atomically mutates an instance record under its permanent device advisory lock.
pub async fn update_instance_with_lock<F>(
    paths: &ResearchPaths,
    id: &str,
    mutator: F,
) -> Result<ResearchGuestInstance>
where
    F: FnOnce(&mut ResearchGuestInstance) -> Result<()>,
{
    let lock_path = paths.instance_device_lock_path(id)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| format!("Failed to acquire device lock for instance '{id}'"))?;

    let mut inst = match load_instance(paths, id).await? {
        Some(i) => i,
        None => bail!("Instance '{id}' not found for update"),
    };

    mutator(&mut inst)?;
    inst.validate()
        .map_err(|e| anyhow::anyhow!("Mutated instance '{id}' failed validation: {e}"))?;

    save_instance_unlocked(paths, &inst).await?;
    Ok(inst)
}

/// Loads and validates a research guest instance from disk.
///
/// Handles only explicit NotFound as absence, and binds record identity to filename.
pub async fn load_instance(
    paths: &ResearchPaths,
    id: &str,
) -> Result<Option<ResearchGuestInstance>> {
    let path = paths.instance_path(id)?;
    let data = match tokio::fs::read(&path).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Failed to read instance at '{}'", path.display()));
        }
    };

    let inst: ResearchGuestInstance = serde_json::from_slice(&data)
        .with_context(|| format!("Failed to parse instance from '{}'", path.display()))?;

    inst.validate().map_err(|e| {
        anyhow::anyhow!(
            "Loaded instance at '{}' failed validation: {e}",
            path.display()
        )
    })?;

    if inst.id.to_string() != id {
        bail!(
            "Identity binding mismatch for instance at '{}': expected ID '{}', found '{}'",
            path.display(),
            id,
            inst.id
        );
    }

    Ok(Some(inst))
}

/// Lists all research guest instances.
///
/// CRITICAL INVARIANT:
/// Propagates malformed records and I/O errors instead of silently filtering.
pub async fn list_instances(paths: &ResearchPaths) -> Result<Vec<ResearchGuestInstance>> {
    let mut entries = tokio::fs::read_dir(paths.instances()).await?;
    let mut instances = Vec::new();

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
                let inst = match load_instance(paths, stem).await? {
                    Some(i) => i,
                    None => continue,
                };
                instances.push(inst);
            }
        }
    }

    instances.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(instances)
}

/// Deletes an instance file under its device advisory lock.
pub async fn delete_instance(paths: &ResearchPaths, id: &str) -> Result<bool> {
    let lock_path = paths.instance_device_lock_path(id)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| format!("Failed to acquire device lock to delete instance '{id}'"))?;

    let path = paths.instance_path(id)?;
    match tokio::fs::remove_file(&path).await {
        Ok(()) => {
            sync_parent_dir(&path)?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e)
            .with_context(|| format!("Failed to delete instance file at '{}'", path.display())),
    }
}
