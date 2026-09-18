//! Discovery logic for minimal Darwin virtual machines.

use crate::models::research::{BackendType, ResearchGuestInstance};
use crate::persistence::instances::list_instances;
use crate::persistence::paths::ResearchPaths;
use anyhow::Result;

pub async fn discover_darwin_instances(
    paths: &ResearchPaths,
) -> Result<Vec<ResearchGuestInstance>> {
    let all = list_instances(paths).await?;
    let filtered = all
        .into_iter()
        .filter(|inst| inst.backend == BackendType::DarwinVm)
        .collect();
    Ok(filtered)
}
