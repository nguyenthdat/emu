//! Mutation proposal persistence, tombstone tracking, and two-step safety gate authorization.
//!
//! Defined in accordance with contracts/research.schema.json, data-model.md §1.2/§5.1, and FR-008.

use super::atomic::write_atomic;
use super::paths::{ResearchPaths, validate_path_component};
use crate::models::research::MutationProposal;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

fn sync_parent_dir(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if let Ok(dir_file) = std::fs::File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }
    Ok(())
}

/// Resolves the permanent consumed proposal tombstone path (`proposals/<digest>.consumed`).
pub fn proposal_tombstone_path(paths: &ResearchPaths, digest: &str) -> Result<PathBuf> {
    let name =
        validate_path_component(digest).context("Invalid proposal digest for tombstone path")?;
    Ok(paths.proposals().join(format!("{name}.consumed")))
}

/// Persists a validated mutation proposal to disk using atomic staged writes.
///
/// CRITICAL INVARIANT:
/// If this proposal digest was already consumed (tombstone exists), recreation is rejected.
pub async fn save_proposal(paths: &ResearchPaths, proposal: &MutationProposal) -> Result<()> {
    proposal
        .validate()
        .context("Proposal validation failed prior to saving")?;

    let tombstone = proposal_tombstone_path(paths, proposal.proposal_digest.as_str())?;
    if tombstone.exists() {
        bail!(
            "Proposal with digest '{}' was already consumed and cannot be recreated",
            proposal.proposal_digest.as_str()
        );
    }

    let path = paths.proposal_path(proposal.proposal_digest.as_str())?;
    let serialized =
        serde_json::to_vec_pretty(proposal).context("Failed to serialize MutationProposal")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(())
}

/// Loads and validates a mutation proposal from disk by its digest string.
///
/// Handles only explicit NotFound as absence, and validates identity binding.
pub async fn load_proposal(
    paths: &ResearchPaths,
    digest: &str,
) -> Result<Option<MutationProposal>> {
    let path = paths.proposal_path(digest)?;
    let data = match tokio::fs::read(&path).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Failed to read proposal at '{}'", path.display()));
        }
    };

    let prop: MutationProposal = serde_json::from_slice(&data)
        .with_context(|| format!("Failed to parse proposal from '{}'", path.display()))?;

    prop.validate()
        .with_context(|| format!("Loaded proposal with digest '{digest}' failed validation"))?;

    if prop.proposal_digest.as_str() != digest {
        bail!(
            "Identity binding mismatch for proposal at '{}': expected digest '{}', found '{}'",
            path.display(),
            digest,
            prop.proposal_digest.as_str()
        );
    }

    Ok(Some(prop))
}

/// Permanently deletes a proposal file from disk upon authorization consumption
/// and creates a durable no-replace tombstone marker, fsyncing the proposals directory.
///
/// CRITICAL INVARIANT:
/// 1. If the proposal file does not exist, returns an error (never treats missing as success).
/// 2. Creates a durable tombstone (`<digest>.consumed`) so the token cannot be reused or re-saved.
/// 3. Fsyncs the proposals directory to guarantee on-disk durability.
pub async fn mark_proposal_consumed(paths: &ResearchPaths, digest: &str) -> Result<()> {
    let path = paths.proposal_path(digest)?;
    if !path.exists() {
        bail!(
            "Proposal file at '{}' does not exist to be consumed",
            path.display()
        );
    }

    tokio::fs::remove_file(&path).await.with_context(|| {
        format!(
            "Failed to remove consumed proposal file at '{}'",
            path.display()
        )
    })?;

    // Create durable tombstone with create-new semantics
    let tombstone = proposal_tombstone_path(paths, digest)?;
    let tombstone_content = format!(
        "{{\"consumed_at\":\"{}\",\"digest\":\"{}\"}}\n",
        chrono::Utc::now().to_rfc3339(),
        digest
    );

    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }

    let mut f = options.open(&tombstone).await.with_context(|| {
        format!(
            "Failed to create proposal consumed tombstone at '{}'",
            tombstone.display()
        )
    })?;

    use tokio::io::AsyncWriteExt;
    f.write_all(tombstone_content.as_bytes()).await?;
    f.sync_all().await?;
    drop(f);

    sync_parent_dir(&path)?;
    Ok(())
}

/// Checks if a proposal has already been consumed by inspecting the durable tombstone.
pub fn is_proposal_consumed(paths: &ResearchPaths, digest: &str) -> Result<bool> {
    let tombstone = proposal_tombstone_path(paths, digest)?;
    Ok(tombstone.exists())
}

/// Lists all pending mutation proposals.
///
/// CRITICAL INVARIANT:
/// Propagates malformed proposal files or I/O errors instead of silently filtering.
pub async fn list_proposals(paths: &ResearchPaths) -> Result<Vec<MutationProposal>> {
    let mut entries = tokio::fs::read_dir(paths.proposals()).await?;
    let mut proposals = Vec::new();
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
                let p = match load_proposal(paths, stem).await? {
                    Some(prop) => prop,
                    None => continue,
                };
                proposals.push(p);
            }
        }
    }
    proposals.sort_by(|a, b| a.expires_at.cmp(&b.expires_at));
    Ok(proposals)
}
