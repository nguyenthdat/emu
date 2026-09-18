//! Inferno iOS research virtualization backend manager.

pub mod app_proxy;
pub mod details;
pub mod discovery;
pub mod lifecycle;
pub use app_proxy::ApplicationProxy;

use crate::managers::common::{DeviceConfig, DeviceManager};
use crate::models::research::{
    BackendType, BootArtifactMap, CpuArchitecture, InstanceLifecycleState, PrivilegeState,
    ResearchGuestId, ResearchGuestInstance, RootVerificationState, Sha256Digest,
};
use crate::persistence::paths::ResearchPaths;
use crate::utils::command_executor::CommandExecutor;
use anyhow::Result;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

pub struct InfernoManager {
    paths: Arc<ResearchPaths>,
    executor: Arc<dyn CommandExecutor>,
}

impl InfernoManager {
    pub fn new(paths: Arc<ResearchPaths>, executor: Arc<dyn CommandExecutor>) -> Self {
        Self { paths, executor }
    }

    pub fn paths(&self) -> &ResearchPaths {
        &self.paths
    }

    pub fn executor(&self) -> &Arc<dyn CommandExecutor> {
        &self.executor
    }

    pub fn app_frameworks_supported(&self) -> bool {
        true
    }
}

impl DeviceManager for InfernoManager {
    type Device = ResearchGuestInstance;

    fn list_devices(&self) -> impl Future<Output = Result<Vec<Self::Device>>> + Send {
        let paths = self.paths.clone();
        async move { discovery::discover_inferno_instances(&paths).await }
    }

    fn start_device(&self, identifier: &str) -> impl Future<Output = Result<()>> + Send {
        let paths = self.paths.clone();
        let id = identifier.to_string();
        async move { lifecycle::start_inferno_instance(&paths, &id).await }
    }

    fn stop_device(&self, identifier: &str) -> impl Future<Output = Result<()>> + Send {
        let paths = self.paths.clone();
        let id = identifier.to_string();
        async move { lifecycle::stop_inferno_instance(&paths, &id).await }
    }

    fn create_device(&self, config: &DeviceConfig) -> impl Future<Output = Result<()>> + Send {
        let paths = self.paths.clone();
        let name = config.name.clone();
        let version = config.version.clone();
        async move {
            let id = ResearchGuestId::new();
            let guest = ResearchGuestInstance::new(
                id,
                name,
                BackendType::Inferno,
                InstanceLifecycleState::Stopped,
                CpuArchitecture::Arm64,
                version,
                "inferno_18A5351d",
                Sha256Digest::new(
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                ),
                Sha256Digest::new(
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                ),
                BootArtifactMap::new(
                    Sha256Digest::new(
                        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                    ),
                    Sha256Digest::new(
                        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                    ),
                ),
                PathBuf::from(format!("/tmp/emu-{}", &id.to_string()[..8])),
                PrivilegeState::Root,
                RootVerificationState::Unverified,
            );
            crate::persistence::instances::save_instance(&paths, &guest).await?;
            Ok(())
        }
    }

    fn delete_device(&self, identifier: &str) -> impl Future<Output = Result<()>> + Send {
        let paths = self.paths.clone();
        let id = identifier.to_string();
        async move { lifecycle::delete_inferno_instance(&paths, &id).await }
    }

    fn wipe_device(&self, identifier: &str) -> impl Future<Output = Result<()>> + Send {
        let paths = self.paths.clone();
        let id = identifier.to_string();
        async move { lifecycle::wipe_inferno_instance(&paths, &id).await }
    }

    async fn is_available(&self) -> bool {
        cfg!(all(target_os = "macos", target_arch = "aarch64"))
    }
}
