//! Experiment record immutable repository persistence.
//!
//! Defined in accordance with data-model.md §1.2 (Rule 9) and FR-042/SC-018:
//! ExperimentRecord files are append-only historical audit records.
//! Immutability is enforced with atomic no-replace/create-new semantics so
//! that two contenders racing on the same record ID can never overwrite.

use super::paths::ResearchPaths;
use crate::models::research::ExperimentRecord;
use anyhow::{Context, Result, bail};

fn sync_parent_dir(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if let Ok(dir_file) = std::fs::File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }
    Ok(())
}

/// Saves an immutable experiment record with atomic no-replace/create-new semantics.
///
/// If a record with the same ID already exists, or if two contenders race to write it,
/// the call fails with an immutable record violation error. Target bytes are never overwritten.
/// The parent directory is physically synced before returning success.
pub async fn save_record(paths: &ResearchPaths, record: &ExperimentRecord) -> Result<()> {
    record
        .validate()
        .map_err(|e| anyhow::anyhow!("ExperimentRecord validation failed prior to saving: {e}"))?;

    let path = paths.record_path(&record.record_id)?;
    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!("Record path '{}' has no parent directory", path.display())
    })?;

    // Unique sibling staging file in the exact same directory
    let unique_id = uuid::Uuid::new_v4().simple();
    let tmp_path = parent.join(format!("{}.{}.tmp", record.record_id, unique_id));

    // RAII guard to ensure staging file cleanup on any early failure
    struct StagingCleanup<'a> {
        path: &'a std::path::Path,
        active: bool,
    }
    impl<'a> Drop for StagingCleanup<'a> {
        fn drop(&mut self) {
            if self.active {
                let _ = std::fs::remove_file(self.path);
            }
        }
    }
    let mut cleanup = StagingCleanup {
        path: &tmp_path,
        active: true,
    };

    let serialized =
        serde_json::to_vec_pretty(record).context("Failed to serialize ExperimentRecord")?;

    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options
        .open(&tmp_path)
        .await
        .with_context(|| format!("Failed to create staging file '{}'", tmp_path.display()))?;

    use tokio::io::AsyncWriteExt;
    file.write_all(&serialized)
        .await
        .with_context(|| format!("Failed to write to staging file '{}'", tmp_path.display()))?;
    file.flush()
        .await
        .with_context(|| format!("Failed to flush staging file '{}'", tmp_path.display()))?;
    file.sync_all()
        .await
        .with_context(|| format!("Failed to sync staging file '{}'", tmp_path.display()))?;
    drop(file);

    // ATOMIC NO-REPLACE / CREATE-NEW SEMANTICS:
    // Atomic hard_link creates a new directory entry pointing to the flushed inode.
    // In POSIX/Darwin, link() fails atomically with EEXIST if destination already exists.
    let link_result = tokio::task::spawn_blocking({
        let src = tmp_path.clone();
        let dst = path.clone();
        move || std::fs::hard_link(&src, &dst)
    })
    .await
    .context("Failed to join blocking hard_link task")?;

    match link_result {
        Ok(()) => {
            cleanup.active = false;
            let _ = tokio::fs::remove_file(&tmp_path).await;
            sync_parent_dir(&path)?;
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            bail!(
                "Immutable record violation: record '{}' already exists and cannot be overwritten",
                record.record_id
            );
        }
        Err(e) => Err(e).with_context(|| {
            format!(
                "Failed to create immutable record link from '{}' to '{}'",
                tmp_path.display(),
                path.display()
            )
        }),
    }
}

/// Loads an immutable experiment record from disk.
///
/// Handles only explicit NotFound as absence, and validates identity binding to filename.
pub async fn load_record(paths: &ResearchPaths, id: &str) -> Result<Option<ExperimentRecord>> {
    let path = paths.record_path(id)?;
    let data = match tokio::fs::read(&path).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e).with_context(|| {
                format!("Failed to read experiment record at '{}'", path.display())
            });
        }
    };

    let rec: ExperimentRecord = serde_json::from_slice(&data).with_context(|| {
        format!(
            "Failed to parse experiment record from '{}'",
            path.display()
        )
    })?;

    rec.validate().map_err(|e| {
        anyhow::anyhow!(
            "Loaded experiment record at '{}' failed validation: {e}",
            path.display()
        )
    })?;

    if rec.record_id != id {
        bail!(
            "Identity binding mismatch for record at '{}': expected '{}', found '{}'",
            path.display(),
            id,
            rec.record_id
        );
    }

    Ok(Some(rec))
}

/// Lists all immutable experiment records.
///
/// CRITICAL INVARIANT:
/// Propagates malformed records or I/O errors instead of silently filtering.
pub async fn list_records(paths: &ResearchPaths) -> Result<Vec<ExperimentRecord>> {
    let mut entries = tokio::fs::read_dir(paths.records()).await?;
    let mut records = Vec::new();

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
                let r = match load_record(paths, stem).await? {
                    Some(rec) => rec,
                    None => continue,
                };
                records.push(r);
            }
        }
    }

    records.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
    Ok(records)
}
