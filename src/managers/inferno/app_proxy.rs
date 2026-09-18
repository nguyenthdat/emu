//! ideviceinstaller and InstallationProxy integration for owned iOS app management.
//!
//! Defined in accordance with plan.md §Principle I, D-03, and FR-016.

use crate::utils::command_executor::CommandExecutor;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

pub struct ApplicationProxy {
    executor: Arc<dyn CommandExecutor>,
}

impl ApplicationProxy {
    pub fn new(executor: Arc<dyn CommandExecutor>) -> Self {
        Self { executor }
    }

    /// Installs an owned iOS application package onto the guest.
    pub async fn install(&self, package_path: &Path, udid: Option<&str>) -> Result<()> {
        let path_str = package_path.to_str().context("Invalid package path")?;
        let cmd = Path::new("ideviceinstaller");
        let mut args = vec!["install", path_str];
        if let Some(u) = udid {
            args.extend(&["-u", u]);
        }
        let _ = self.executor.run(cmd, &args).await?;
        Ok(())
    }

    /// Uninstalls an application by bundle identifier.
    pub async fn uninstall(&self, bundle_id: &str, udid: Option<&str>) -> Result<()> {
        let cmd = Path::new("ideviceinstaller");
        let mut args = vec!["uninstall", bundle_id];
        if let Some(u) = udid {
            args.extend(&["-u", u]);
        }
        let _ = self.executor.run(cmd, &args).await?;
        Ok(())
    }

    /// Lists installed applications on the target guest.
    pub async fn list(&self, udid: Option<&str>) -> Result<Vec<String>> {
        let cmd = Path::new("ideviceinstaller");
        let mut args = vec!["list", "--json"];
        if let Some(u) = udid {
            args.extend(&["-u", u]);
        }
        let output = self.executor.run(cmd, &args).await?;
        // Parse bundle IDs or return line split
        let bundles: Vec<String> = output
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(bundles)
    }
}
