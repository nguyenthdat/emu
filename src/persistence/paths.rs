//! Research storage hierarchy, path resolution, and secure runtime directories.

use anyhow::Context as _;
use std::path::{Path, PathBuf};

use crate::constants::research::storage::{
    DIR_ARTIFACTS, DIR_BASELINES, DIR_INSTANCES, DIR_LOCKS, DIR_OPERATIONS, DIR_PROFILES,
    DIR_PROPOSALS, DIR_RECORDS, DIR_SECURITY_PROFILES, EXT_EVENTS_JSONL, EXT_JSON,
    MAX_DARWIN_SUN_PATH, RUNTIME_DIR_PREFIX, SECURE_DIR_MODE, SUFFIX_DEVICE_LOCK, SUFFIX_OP_LOCK,
    SUFFIX_RUN_LOCK,
};

/// Typed errors for path validation and security traversal rejection.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathError {
    #[error("path traversal attempt rejected: '{0}'")]
    Traversal(String),

    #[error("path component contains invalid characters: '{0}'")]
    InvalidComponent(String),

    #[error("path component cannot be empty")]
    EmptyComponent,

    #[error(
        "socket path '{path}' length {len} bytes (including NUL) exceeds Darwin maximum of {max} bytes"
    )]
    SocketPathTooLong {
        path: PathBuf,
        len: usize,
        max: usize,
    },

    #[error("security violation: {0}")]
    SecurityViolation(String),
}

/// Validates that a filename or path component is safe and does not attempt directory traversal.
pub fn validate_path_component(name: &str) -> Result<&str, PathError> {
    if name.is_empty() {
        return Err(PathError::EmptyComponent);
    }
    if name.contains('\0') {
        return Err(PathError::InvalidComponent(name.to_string()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(PathError::Traversal(name.to_string()));
    }
    if name == "." || name == ".." {
        return Err(PathError::Traversal(name.to_string()));
    }
    for comp in Path::new(name).components() {
        match comp {
            std::path::Component::Normal(_) => {}
            _ => return Err(PathError::Traversal(name.to_string())),
        }
    }
    Ok(name)
}

/// Strongly-typed hierarchy of research persistence storage directories.
#[derive(Debug, Clone)]
pub struct ResearchPaths {
    root: PathBuf,
    instances: PathBuf,
    instance_locks: PathBuf,
    profiles: PathBuf,
    artifacts: PathBuf,
    baselines: PathBuf,
    records: PathBuf,
    operations: PathBuf,
    operation_locks: PathBuf,
    proposals: PathBuf,
    security_profiles: PathBuf,
}

impl ResearchPaths {
    /// Construct research paths rooted at an explicit absolute path.
    pub fn new(root: PathBuf) -> anyhow::Result<Self> {
        if !root.is_absolute() {
            anyhow::bail!(
                "ResearchPaths root must be an absolute path: '{}'",
                root.display()
            );
        }

        let instances = root.join(DIR_INSTANCES);
        let instance_locks = instances.join(DIR_LOCKS);
        let profiles = root.join(DIR_PROFILES);
        let artifacts = root.join(DIR_ARTIFACTS);
        let baselines = root.join(DIR_BASELINES);
        let records = root.join(DIR_RECORDS);
        let operations = root.join(DIR_OPERATIONS);
        let operation_locks = operations.join(DIR_LOCKS);
        let proposals = root.join(DIR_PROPOSALS);
        let security_profiles = root.join(DIR_SECURITY_PROFILES);

        Ok(Self {
            root,
            instances,
            instance_locks,
            profiles,
            artifacts,
            baselines,
            records,
            operations,
            operation_locks,
            proposals,
            security_profiles,
        })
    }

    /// Resolve platform-local storage directory: `dirs::data_local_dir()/emu/research`.
    pub fn platform() -> anyhow::Result<Self> {
        let data_dir = dirs::data_local_dir().ok_or_else(|| {
            anyhow::anyhow!("Platform local data directory could not be determined")
        })?;
        let root = data_dir.join("emu").join("research");
        Self::new(root)
    }

    /// Asynchronously ensure all directories in the research persistence hierarchy exist
    /// with restrictive permissions (0700) and fail-closed symlink rejection.
    pub async fn ensure(&self) -> anyhow::Result<()> {
        let all_dirs = [
            &self.root,
            &self.instances,
            &self.instance_locks,
            &self.profiles,
            &self.artifacts,
            &self.baselines,
            &self.records,
            &self.operations,
            &self.operation_locks,
            &self.proposals,
            &self.security_profiles,
        ];

        for dir in all_dirs {
            self.ensure_dir_secure(dir).await?;
        }

        Ok(())
    }

    /// Recursively ensure a directory within `self.root` exists with strict 0700 permissions
    /// while rejecting pre-existing or created symlinks (fail-closed secure storage containment).
    /// Permissions 0700 are strictly bounded to `self.root` and its descendants; ancestors
    /// of `self.root` are never modified.
    async fn ensure_dir_secure(&self, dir: &Path) -> anyhow::Result<()> {
        if !dir.starts_with(&self.root) {
            anyhow::bail!(
                "Security violation: attempt to ensure directory outside root: '{}'",
                dir.display()
            );
        }

        match tokio::fs::symlink_metadata(dir).await {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    anyhow::bail!(
                        "Security violation: persistence directory '{}' is an unsafe symlink",
                        dir.display()
                    );
                }
                if !meta.is_dir() {
                    anyhow::bail!(
                        "Security violation: persistence path '{}' exists but is not a directory",
                        dir.display()
                    );
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    tokio::fs::set_permissions(
                        dir,
                        std::fs::Permissions::from_mode(SECURE_DIR_MODE),
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to set 0700 permissions on '{}'", dir.display())
                    })?;
                }
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if dir == self.root {
                    if let Some(parent) = self.root.parent() {
                        tokio::fs::create_dir_all(parent).await.with_context(|| {
                            format!(
                                "Failed to create parent directories for '{}'",
                                self.root.display()
                            )
                        })?;
                    }
                } else if let Some(parent) = dir.parent() {
                    if parent.starts_with(&self.root) {
                        Box::pin(self.ensure_dir_secure(parent)).await?;
                    }
                }
                let mut builder = std::fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(SECURE_DIR_MODE);
                }
                builder.create(dir).with_context(|| {
                    format!("Failed to create secure directory '{}'", dir.display())
                })?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    tokio::fs::set_permissions(
                        dir,
                        std::fs::Permissions::from_mode(SECURE_DIR_MODE),
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to set 0700 permissions on '{}'", dir.display())
                    })?;
                }
                let meta = tokio::fs::symlink_metadata(dir).await?;
                if meta.file_type().is_symlink() {
                    anyhow::bail!(
                        "Security violation: created directory '{}' was a symlink",
                        dir.display()
                    );
                }
                Ok(())
            }
            Err(e) => {
                Err(e).with_context(|| format!("Failed to stat directory '{}'", dir.display()))
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn instances(&self) -> &Path {
        &self.instances
    }

    pub fn instance_locks(&self) -> &Path {
        &self.instance_locks
    }

    pub fn profiles(&self) -> &Path {
        &self.profiles
    }

    pub fn artifacts(&self) -> &Path {
        &self.artifacts
    }

    pub fn baselines(&self) -> &Path {
        &self.baselines
    }

    pub fn records(&self) -> &Path {
        &self.records
    }

    pub fn operations(&self) -> &Path {
        &self.operations
    }

    pub fn operation_locks(&self) -> &Path {
        &self.operation_locks
    }

    pub fn proposals(&self) -> &Path {
        &self.proposals
    }

    pub fn security_profiles(&self) -> &Path {
        &self.security_profiles
    }

    // Resolvers with typed traversal rejection

    pub fn instance_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.instances.join(format!("{name}.{EXT_JSON}")))
    }

    pub fn instance_run_lock_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.instance_locks.join(format!("{name}{SUFFIX_RUN_LOCK}")))
    }

    pub fn instance_device_lock_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self
            .instance_locks
            .join(format!("{name}{SUFFIX_DEVICE_LOCK}")))
    }

    pub fn profile_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.profiles.join(format!("{name}.{EXT_JSON}")))
    }

    pub fn artifact_path(&self, digest: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(digest)?;
        Ok(self.artifacts.join(format!("{name}.{EXT_JSON}")))
    }

    pub fn baseline_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.baselines.join(format!("{name}.{EXT_JSON}")))
    }

    pub fn record_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.records.join(format!("{name}.{EXT_JSON}")))
    }

    pub fn operation_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.operations.join(format!("{name}.{EXT_JSON}")))
    }

    pub fn operation_events_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.operations.join(format!("{name}.{EXT_EVENTS_JSONL}")))
    }

    pub fn operation_lock_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.operation_locks.join(format!("{name}{SUFFIX_OP_LOCK}")))
    }

    pub fn proposal_path(&self, digest: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(digest)?;
        Ok(self.proposals.join(format!("{name}.{EXT_JSON}")))
    }

    pub fn security_profile_path(&self, id: &str) -> Result<PathBuf, PathError> {
        let name = validate_path_component(id)?;
        Ok(self.security_profiles.join(format!("{name}.{EXT_JSON}")))
    }
}

/// Private owner-restricted temporary runtime directory for short Unix Domain Socket paths.
#[derive(Debug)]
pub struct RuntimeDirectory {
    path: PathBuf,
    #[cfg(unix)]
    owner_uid: u32,
    cleaned: bool,
}

impl RuntimeDirectory {
    /// Create a private 0700 runtime directory under the platform temporary directory
    /// with a short UUID name. Correctly resolves trusted platform alias `/tmp` -> `/private/tmp`
    /// on macOS.
    pub async fn create() -> anyhow::Result<Self> {
        Self::create_under(Path::new("/tmp")).await
    }

    /// Create a private 0700 runtime directory under a specified base directory.
    /// Safely handles the trusted platform alias `/tmp` on macOS by routing to its physical directory
    /// `/private/tmp`, while strictly rejecting arbitrary untrusted symlink bases.
    pub async fn create_under(base: &Path) -> anyhow::Result<Self> {
        // Resolve platform trusted temporary directory alias on macOS (/tmp -> /private/tmp)
        let resolved_base: PathBuf = if base == Path::new("/tmp") {
            #[cfg(target_os = "macos")]
            {
                PathBuf::from("/private/tmp")
            }
            #[cfg(not(target_os = "macos"))]
            {
                PathBuf::from("/tmp")
            }
        } else {
            let base_meta = tokio::fs::symlink_metadata(base).await.with_context(|| {
                format!("Failed to stat runtime base directory '{}'", base.display())
            })?;

            if base_meta.file_type().is_symlink() {
                anyhow::bail!(
                    "Security violation: runtime base directory '{}' is an untrusted symlink",
                    base.display()
                );
            }
            base.to_path_buf()
        };

        let base_meta = tokio::fs::symlink_metadata(&resolved_base)
            .await
            .with_context(|| {
                format!(
                    "Failed to stat runtime base directory '{}'",
                    resolved_base.display()
                )
            })?;

        if !base_meta.is_dir() {
            anyhow::bail!(
                "Runtime base directory '{}' is not a directory",
                resolved_base.display()
            );
        }

        for _ in 0..16 {
            let short_uuid = &uuid::Uuid::new_v4().simple().to_string()[..12];
            let candidate = resolved_base.join(format!("{RUNTIME_DIR_PREFIX}{short_uuid}"));

            // Reject if candidate already exists (including as an attacker symlink)
            if std::fs::symlink_metadata(&candidate).is_ok() {
                continue;
            }

            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(SECURE_DIR_MODE);
            }

            if let Err(e) = builder.create(&candidate) {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    continue;
                }
                return Err(e).with_context(|| {
                    format!(
                        "Failed to create runtime directory '{}'",
                        candidate.display()
                    )
                });
            }

            let meta = std::fs::symlink_metadata(&candidate).with_context(|| {
                format!(
                    "Failed to stat created runtime directory '{}'",
                    candidate.display()
                )
            })?;

            if meta.file_type().is_symlink() {
                let _ = std::fs::remove_file(&candidate);
                anyhow::bail!(
                    "Security violation: created runtime path was a symlink: '{}'",
                    candidate.display()
                );
            }

            if !meta.is_dir() {
                let _ = std::fs::remove_file(&candidate);
                anyhow::bail!(
                    "Created runtime path is not a directory: '{}'",
                    candidate.display()
                );
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &candidate,
                    std::fs::Permissions::from_mode(SECURE_DIR_MODE),
                );
                let meta = std::fs::symlink_metadata(&candidate)?;
                let mode = meta.permissions().mode() & 0o777;
                if mode != SECURE_DIR_MODE {
                    let _ = std::fs::remove_dir(&candidate);
                    anyhow::bail!(
                        "Runtime directory permissions must be 0{SECURE_DIR_MODE:o}, got 0{mode:o}"
                    );
                }
                let owner_uid = std::os::unix::fs::MetadataExt::uid(&meta);
                return Ok(Self {
                    path: candidate,
                    owner_uid,
                    cleaned: false,
                });
            }

            #[cfg(not(unix))]
            {
                return Ok(Self {
                    path: candidate,
                    cleaned: false,
                });
            }
        }

        anyhow::bail!("Failed to allocate unique runtime directory after 16 attempts")
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Construct a verified socket path under this runtime directory, ensuring
    /// the socket name is valid and total byte length (including terminating NUL) <= 104.
    pub fn socket_path(&self, name: &str) -> anyhow::Result<PathBuf> {
        let valid_name = validate_path_component(name)
            .map_err(|e| anyhow::anyhow!("Invalid socket name '{name}': {e}"))?;

        let candidate = self.path.join(valid_name);
        let path_bytes = candidate.as_os_str().as_encoded_bytes();

        // 104 bytes max including NUL terminator
        let total_len = path_bytes.len() + 1;
        if total_len > MAX_DARWIN_SUN_PATH {
            anyhow::bail!(
                "Socket path length {} bytes (including NUL) exceeds Darwin limit of {} bytes: '{}'",
                total_len,
                MAX_DARWIN_SUN_PATH,
                candidate.display()
            );
        }

        Ok(candidate)
    }

    /// Explicit asynchronous cleanup of the runtime directory and its contents.
    /// Only removes if the directory is genuinely owned by the creator UID and is not a symlink.
    pub async fn cleanup(mut self) -> anyhow::Result<()> {
        Self::cleanup_dir(
            &self.path,
            #[cfg(unix)]
            self.owner_uid,
        )
        .await?;
        self.cleaned = true;
        Ok(())
    }

    async fn cleanup_dir(path: &Path, #[cfg(unix)] expected_owner: u32) -> anyhow::Result<()> {
        let meta = match tokio::fs::symlink_metadata(path).await {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e).context("Failed to stat runtime directory for cleanup"),
        };

        if meta.file_type().is_symlink() {
            anyhow::bail!(
                "Security violation: refusing to remove runtime directory that is a symlink: '{}'",
                path.display()
            );
        }

        if !meta.is_dir() {
            anyhow::bail!("Runtime path is not a directory: '{}'", path.display());
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.uid() != expected_owner {
                anyhow::bail!(
                    "Security violation: runtime directory owner UID mismatch (expected {}, got {})",
                    expected_owner,
                    meta.uid()
                );
            }
        }

        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let entry_path = entry.path();
            let entry_meta = tokio::fs::symlink_metadata(&entry_path).await?;
            if entry_meta.is_dir() && !entry_meta.file_type().is_symlink() {
                tokio::fs::remove_dir_all(&entry_path).await?;
            } else {
                // Unlink all non-directory entries (regular files, symlinks, Unix domain sockets, FIFOs)
                // without following symlinks. remove_file uses unlink(2) which unlinks socket nodes.
                tokio::fs::remove_file(&entry_path).await?;
            }
        }

        tokio::fs::remove_dir(path)
            .await
            .with_context(|| format!("Failed to remove runtime directory '{}'", path.display()))?;

        Ok(())
    }

    fn sync_cleanup_dir(path: &Path, #[cfg(unix)] expected_owner: u32) -> std::io::Result<()> {
        let meta = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Not a directory or is a symlink",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.uid() != expected_owner {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "UID mismatch",
                ));
            }
        }
        if let Ok(read_dir) = std::fs::read_dir(path) {
            for entry in read_dir.flatten() {
                let entry_path = entry.path();
                if let Ok(entry_meta) = std::fs::symlink_metadata(&entry_path) {
                    if entry_meta.is_dir() && !entry_meta.file_type().is_symlink() {
                        let _ = std::fs::remove_dir_all(&entry_path);
                    } else {
                        let _ = std::fs::remove_file(&entry_path);
                    }
                }
            }
        }
        std::fs::remove_dir(path)
    }
}

impl Drop for RuntimeDirectory {
    fn drop(&mut self) {
        if !self.cleaned {
            #[cfg(unix)]
            let _ = Self::sync_cleanup_dir(&self.path, self.owner_uid);
            #[cfg(not(unix))]
            let _ = Self::sync_cleanup_dir(&self.path);
        }
    }
}
