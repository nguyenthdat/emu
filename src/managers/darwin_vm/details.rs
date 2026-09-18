//! Runtime details and inspection for Darwin VMs.

use crate::models::research::ResearchGuestInstance;
use crate::persistence::instances::load_instance;
use crate::persistence::paths::ResearchPaths;
use anyhow::{Context, Result};

pub async fn get_darwin_details(paths: &ResearchPaths, id: &str) -> Result<ResearchGuestInstance> {
    load_instance(paths, id)
        .await?
        .with_context(|| format!("Darwin VM '{id}' not found"))
}
