//! Transactional atomic file writes using sibling .tmp staging, sync_all, and atomic rename.

use anyhow::Context as _;
use std::path::Path;

use crate::constants::research::storage::{EXT_TMP, SECURE_FILE_MODE};

/// Write data atomically to `path` using a unique sibling `.tmp` file, `File::sync_all`,
/// and atomic rename, ensuring directory durability and preserving prior target on failure.
pub async fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!("Target path '{}' has no parent directory", path.display())
    })?;

    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Target path '{}' has no filename", path.display()))?
        .to_string_lossy();

    // Unique sibling .tmp file to prevent concurrent writers from colliding or mixing bytes
    let unique_id = uuid::Uuid::new_v4().simple();
    let tmp_name = format!("{file_name}.{unique_id}.{EXT_TMP}");
    let tmp_path = parent.join(tmp_name);

    // RAII guard to clean up the temporary file if any failure occurs prior to successful rename
    struct TmpCleanup<'a> {
        path: &'a Path,
        active: bool,
    }
    impl<'a> Drop for TmpCleanup<'a> {
        fn drop(&mut self) {
            if self.active {
                let _ = std::fs::remove_file(self.path);
            }
        }
    }

    let mut cleanup_guard = TmpCleanup {
        path: &tmp_path,
        active: true,
    };

    // Open sibling temporary file with restrictive permissions (0600 on Unix)
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(SECURE_FILE_MODE);

    let mut file = options
        .open(&tmp_path)
        .await
        .with_context(|| format!("Failed to create staging file '{}'", tmp_path.display()))?;

    use tokio::io::AsyncWriteExt;
    file.write_all(bytes).await.with_context(|| {
        format!(
            "Failed to write data to staging file '{}'",
            tmp_path.display()
        )
    })?;

    file.flush()
        .await
        .with_context(|| format!("Failed to flush staging file '{}'", tmp_path.display()))?;

    // Physical sync to storage
    file.sync_all()
        .await
        .with_context(|| format!("Failed to sync staging file '{}'", tmp_path.display()))?;

    drop(file);

    // Atomic rename over target path
    tokio::fs::rename(&tmp_path, path).await.with_context(|| {
        format!(
            "Failed to atomically rename temporary file '{}' to target '{}'",
            tmp_path.display(),
            path.display()
        )
    })?;

    // Disarm cleanup guard now that rename succeeded
    cleanup_guard.active = false;

    // Ensure parent directory durability on Unix with propagated errors
    #[cfg(unix)]
    {
        let parent_dir = tokio::fs::File::open(parent).await.with_context(|| {
            format!(
                "Failed to open parent directory '{}' to ensure durability after rename of '{}'",
                parent.display(),
                path.display()
            )
        })?;
        parent_dir.sync_all().await.with_context(|| {
            format!(
                "Failed to sync parent directory '{}' to ensure durability after rename of '{}'",
                parent.display(),
                path.display()
            )
        })?;
    }

    Ok(())
}
