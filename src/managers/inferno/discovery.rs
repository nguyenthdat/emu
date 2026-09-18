//! Discovery logic for Inferno iOS research virtualization instances.

use crate::models::research::{BackendType, ResearchGuestInstance};
use crate::persistence::instances::list_instances;
use crate::persistence::paths::ResearchPaths;
use anyhow::Result;

pub async fn discover_inferno_instances(
    paths: &ResearchPaths,
) -> Result<Vec<ResearchGuestInstance>> {
    let all = list_instances(paths).await?;
    let filtered = all
        .into_iter()
        .filter(|inst| inst.backend == BackendType::Inferno)
        .collect();
    Ok(filtered)
}
