//! Guest security profile repository persistence.
//!
//! Defined in accordance with data-model.md §2.10:
//! Manages guest security profile records under advisory locks with validation
//! and identity binding.

use super::atomic::write_atomic;
use super::lock::AdvisoryLock;
use super::paths::{ResearchPaths, validate_path_component};
use crate::models::research::GuestSecurityProfile;
use anyhow::{Context, Result, bail};

fn sync_parent_dir(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if let Ok(dir_file) = std::fs::File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }
    Ok(())
}

fn security_profile_lock_path(paths: &ResearchPaths, id: &str) -> Result<std::path::PathBuf> {
    let name = validate_path_component(id)?;
    Ok(paths.operation_locks().join(format!("{name}.secprof.lock")))
}

pub async fn save_security_profile(
    paths: &ResearchPaths,
    profile: &GuestSecurityProfile,
) -> Result<()> {
    profile.validate().map_err(|e| {
        anyhow::anyhow!("GuestSecurityProfile validation failed prior to saving: {e}")
    })?;

    let lock_path = security_profile_lock_path(paths, &profile.profile_id)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| {
            format!(
                "Failed to acquire lock for security profile '{}'",
                profile.profile_id
            )
        })?;

    let path = paths.security_profile_path(&profile.profile_id)?;
    let serialized =
        serde_json::to_vec_pretty(profile).context("Failed to serialize GuestSecurityProfile")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(())
}

pub async fn update_security_profile_with_lock<F>(
    paths: &ResearchPaths,
    id: &str,
    mutator: F,
) -> Result<GuestSecurityProfile>
where
    F: FnOnce(&mut GuestSecurityProfile) -> Result<()>,
{
    let lock_path = security_profile_lock_path(paths, id)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| format!("Failed to acquire lock for security profile '{id}'"))?;

    let mut prof = match load_security_profile(paths, id).await? {
        Some(p) => p,
        None => bail!("Security profile '{id}' not found for update"),
    };

    mutator(&mut prof)?;
    prof.validate()
        .map_err(|e| anyhow::anyhow!("Mutated security profile '{id}' failed validation: {e}"))?;

    let path = paths.security_profile_path(id)?;
    let serialized =
        serde_json::to_vec_pretty(&prof).context("Failed to serialize GuestSecurityProfile")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(prof)
}

pub async fn load_security_profile(
    paths: &ResearchPaths,
    id: &str,
) -> Result<Option<GuestSecurityProfile>> {
    let path = paths.security_profile_path(id)?;
    let data = match tokio::fs::read(&path).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e).with_context(|| {
                format!("Failed to read security profile at '{}'", path.display())
            });
        }
    };

    let prof: GuestSecurityProfile = serde_json::from_slice(&data)
        .with_context(|| format!("Failed to parse security profile from '{}'", path.display()))?;

    prof.validate().map_err(|e| {
        anyhow::anyhow!(
            "Loaded security profile at '{}' failed validation: {e}",
            path.display()
        )
    })?;

    if prof.profile_id != id {
        bail!(
            "Identity binding mismatch for security profile at '{}': expected ID '{}', found '{}'",
            path.display(),
            id,
            prof.profile_id
        );
    }

    Ok(Some(prof))
}

pub async fn list_security_profiles(paths: &ResearchPaths) -> Result<Vec<GuestSecurityProfile>> {
    let mut entries = tokio::fs::read_dir(paths.security_profiles()).await?;
    let mut profiles = Vec::new();

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
                let p = match load_security_profile(paths, stem).await? {
                    Some(prof) => prof,
                    None => continue,
                };
                profiles.push(p);
            }
        }
    }

    profiles.sort_by(|a, b| a.profile_id.cmp(&b.profile_id));
    Ok(profiles)
}
