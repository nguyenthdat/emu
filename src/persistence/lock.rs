//! Cross-process advisory locking engine via fs4::FileExt on dedicated, permanent lock files.

use std::path::{Path, PathBuf};

use crate::constants::research::storage::{
    LOCK_CONTENTION_EXIT_CODE, SECURE_DIR_MODE, SECURE_FILE_MODE,
};

/// Strongly-typed error representing advisory lock failures.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("concurrency conflict: lock on '{path}' is held by another process (exit code 5)")]
    Contended { path: PathBuf },

    #[error("io error for lock on '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl LockError {
    /// Maps lock error to CLI exit code (5 for contention conflict, 1 for I/O failure).
    pub fn exit_code(&self) -> i32 {
        match self {
            LockError::Contended { .. } => LOCK_CONTENTION_EXIT_CODE,
            LockError::Io { .. } => 1,
        }
    }

    pub fn is_contended(&self) -> bool {
        matches!(self, LockError::Contended { .. })
    }

    pub fn path(&self) -> &Path {
        match self {
            LockError::Contended { path } => path,
            LockError::Io { path, .. } => path,
        }
    }
}

/// An owned advisory lock held on a dedicated, permanent file.
///
/// CRITICAL INVARIANT (data-model.md §1.2, plan.md §Storage):
/// Dedicated lock files are NEVER unlinked or deleted while held or upon drop.
/// Ownership is governed strictly by kernel advisory flock semantics via `fs4::FileExt`.
/// Retaining lock files on disk prevents inode recycling race conditions.
#[derive(Debug)]
pub struct AdvisoryLock {
    file: std::fs::File,
    path: PathBuf,
}

impl AdvisoryLock {
    /// Non-blocking acquisition of an exclusive advisory lock on `path`.
    /// Returns `LockError::Contended` (exit code 5) if the lock is held by another process.
    /// Rejects symlink targets and symlink parent directories to enforce secure storage containment.
    pub async fn try_acquire(path: &Path) -> Result<Self, LockError> {
        let parent = path.parent().ok_or_else(|| LockError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Lock path has no parent directory",
            ),
        })?;

        // Ensure lock directory exists
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| LockError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    parent,
                    std::fs::Permissions::from_mode(SECURE_DIR_MODE),
                );
            }
        }

        // Verify parent is not an unsafe symlink
        let parent_meta = std::fs::symlink_metadata(parent).map_err(|e| LockError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        if parent_meta.file_type().is_symlink() {
            return Err(LockError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Security violation: lock parent directory is an unsafe symlink",
                ),
            });
        }

        // Verify target path without following symlinks: reject symlinked lockfiles
        if let Ok(target_meta) = std::fs::symlink_metadata(path) {
            if target_meta.file_type().is_symlink() {
                return Err(LockError::Io {
                    path: path.to_path_buf(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Security violation: lock file is an unsafe symlink",
                    ),
                });
            }
            if !target_meta.is_file() {
                return Err(LockError::Io {
                    path: path.to_path_buf(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Lock path is not a regular file",
                    ),
                });
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    path,
                    std::fs::Permissions::from_mode(SECURE_FILE_MODE),
                );
            }
        }

        // Open or create the permanent dedicated lock file
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(SECURE_FILE_MODE);
        }

        let file = options.open(path).map_err(|e| LockError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Non-blocking try_lock explicitly calling fs4::FileExt to avoid collision
        // with newer Rust std::fs::File::try_lock (stabilized in 1.89+).
        match fs4::FileExt::try_lock(&file) {
            Ok(()) => Ok(Self {
                file,
                path: path.to_path_buf(),
            }),
            Err(fs4::TryLockError::WouldBlock) => Err(LockError::Contended {
                path: path.to_path_buf(),
            }),
            Err(fs4::TryLockError::Error(e)) => Err(LockError::Io {
                path: path.to_path_buf(),
                source: e,
            }),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file(&self) -> &std::fs::File {
        &self.file
    }

    /// Explicitly release the advisory lock. Consumes `self`.
    /// Does NOT delete or unlink the file on disk.
    pub fn release(self) -> Result<(), LockError> {
        fs4::FileExt::unlock(&self.file).map_err(|e| LockError::Io {
            path: self.path.clone(),
            source: e,
        })
    }
}

impl Drop for AdvisoryLock {
    fn drop(&mut self) {
        // Unlock explicitly; CRITICALLY, DO NOT unlink or delete the lock file!
        let _ = fs4::FileExt::unlock(&self.file);
    }
}
