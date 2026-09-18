//! Runtime details, SpringBoard status, and inspection for Inferno guests.

use crate::models::research::ResearchGuestInstance;
use crate::persistence::instances::load_instance;
use crate::persistence::paths::ResearchPaths;
use anyhow::{Context, Result};

pub async fn get_inferno_details(paths: &ResearchPaths, id: &str) -> Result<ResearchGuestInstance> {
    load_instance(paths, id)
        .await?
        .with_context(|| format!("Inferno guest '{id}' not found"))
}
