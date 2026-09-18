//! Operations journal and event stream persistence.
//!
//! Defined in accordance with contracts/research.schema.json, data-model.md §1.2/§3.1/§7.7, and FR-046/FR-047.

use super::atomic::write_atomic;
use super::lock::AdvisoryLock;
use super::paths::{ResearchPaths, validate_path_component};
use crate::cli::envelope::StreamLogEnvelope;
use crate::models::research::{OperationRecord, OperationStatus};
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

/// Default number of events returned in a paginated event query.
pub const DEFAULT_EVENT_PAGE_LIMIT: usize = 50;
/// Hard ceiling for maximum events returned in a single query.
pub const MAX_EVENT_PAGE_LIMIT: usize = 1000;
/// Hard byte ceiling for a single paginated events response (1 MiB).
pub const MAX_EVENT_PAGE_BYTES: usize = 1024 * 1024;

/// Resolves the dedicated, permanent journal advisory lock path (`operations/locks/<id>.journal.lock`).
///
/// CRITICAL INVARIANT (data-model.md §1.2):
/// The journal lock is distinct from the worker lifetime ownership lock (`.op.lock`).
/// This ensures callers can update the journal (e.g. for cancellation or status checks)
/// without contending with an actively executing worker holding `.op.lock`.
pub fn operation_journal_lock_path(paths: &ResearchPaths, operation_id: &str) -> Result<PathBuf> {
    let name = validate_path_component(operation_id)
        .context("Invalid operation ID for journal lock path")?;
    Ok(paths.operation_locks().join(format!("{name}.journal.lock")))
}

fn sync_parent_dir(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if let Ok(dir_file) = std::fs::File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }
    Ok(())
}

/// Saves an operation journal entry under its dedicated `.journal.lock`, enforcing validation
/// and guarded state transitions against previously persisted state.
pub async fn save_operation(paths: &ResearchPaths, op: &OperationRecord) -> Result<()> {
    op.validate()
        .context("Validation failed for OperationRecord prior to saving")?;

    let lock_path = operation_journal_lock_path(paths, &op.operation_id)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| {
            format!(
                "Failed to acquire journal lock for operation '{}'",
                op.operation_id
            )
        })?;

    // Perform locked reload and state transition check
    if let Some(prev) = load_operation(paths, &op.operation_id).await? {
        if prev.is_terminal() && op.status != prev.status {
            bail!(
                "Cannot update terminal operation '{}' in state {:?}",
                op.operation_id,
                prev.status
            );
        }
        if prev.consumed_proposal_digest.is_some() && op.consumed_proposal_digest.is_none() {
            bail!(
                "Cannot erase consumed proposal digest from operation '{}'",
                op.operation_id
            );
        }
        let mut checker = prev.clone();
        checker.transition_to(op.status, &op.phase).with_context(|| {
            format!(
                "Guarded transition rejected in persistence: cannot transition '{}' from {:?} to {:?}",
                op.operation_id, prev.status, op.status
            )
        })?;
    } else if op.status != OperationStatus::Created {
        bail!(
            "Initial creation of operation '{}' must be in Created state, found {:?}",
            op.operation_id,
            op.status
        );
    }

    save_operation_unlocked(paths, op).await
}

/// Saves an operation journal entry when the journal advisory lock is already held.
pub async fn save_operation_unlocked(paths: &ResearchPaths, op: &OperationRecord) -> Result<()> {
    op.validate()
        .context("Validation failed for OperationRecord prior to saving")?;

    let path = paths.operation_path(&op.operation_id)?;
    let serialized =
        serde_json::to_vec_pretty(op).context("Failed to serialize OperationRecord")?;
    write_atomic(&path, &serialized).await?;
    sync_parent_dir(&path)?;
    Ok(())
}

/// Atomically mutates an operation record under its permanent dedicated `.journal.lock`.
pub async fn update_operation_with_lock<F>(
    paths: &ResearchPaths,
    operation_id: &str,
    mutator: F,
) -> Result<OperationRecord>
where
    F: FnOnce(&mut OperationRecord) -> Result<()>,
{
    let lock_path = operation_journal_lock_path(paths, operation_id)?;
    let _lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .with_context(|| {
            format!("Failed to acquire exclusive journal lock for operation '{operation_id}'")
        })?;

    let mut op = match load_operation(paths, operation_id).await? {
        Some(o) => o,
        None => bail!("Cannot update non-existent operation '{operation_id}'"),
    };

    let prev = op.clone();
    mutator(&mut op)?;

    // Guard invariants
    if prev.is_terminal() && op.status != prev.status {
        bail!(
            "Cannot update terminal operation '{operation_id}' in state {:?}",
            prev.status
        );
    }
    if prev.consumed_proposal_digest.is_some() && op.consumed_proposal_digest.is_none() {
        bail!("Cannot erase consumed proposal digest from operation '{operation_id}'");
    }
    let mut checker = prev.clone();
    checker.transition_to(op.status, &op.phase).with_context(|| {
        format!(
            "Guarded transition rejected in persistence: cannot transition '{operation_id}' from {:?} to {:?}",
            prev.status, op.status
        )
    })?;

    op.validate()
        .context("Mutated OperationRecord is invalid")?;

    save_operation_unlocked(paths, &op).await?;
    Ok(op)
}

/// Loads and validates an operation journal entry from disk.
///
/// CRITICAL INVARIANTS:
/// 1. Only handles explicit `NotFound` as absence; propagates other I/O errors.
/// 2. Validates schema and binds record identity to filename.
pub async fn load_operation(
    paths: &ResearchPaths,
    operation_id: &str,
) -> Result<Option<OperationRecord>> {
    let path = paths.operation_path(operation_id)?;
    let data = match tokio::fs::read(&path).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e).with_context(|| {
                format!("Failed to read operation record at '{}'", path.display())
            });
        }
    };

    let op: OperationRecord = serde_json::from_slice(&data)
        .with_context(|| format!("Failed to parse operation record from '{}'", path.display()))?;

    op.validate().with_context(|| {
        format!(
            "Loaded operation record from '{}' is invalid",
            path.display()
        )
    })?;

    if op.operation_id != operation_id {
        bail!(
            "Identity binding mismatch for operation record at '{}': expected '{}', found '{}'",
            path.display(),
            operation_id,
            op.operation_id
        );
    }

    Ok(Some(op))
}

/// Lists all operations in the repository.
///
/// CRITICAL INVARIANT:
/// Propagates malformed records or I/O failures instead of silently filtering them out.
pub async fn list_operations(paths: &ResearchPaths) -> Result<Vec<OperationRecord>> {
    let mut entries = tokio::fs::read_dir(paths.operations()).await?;
    let mut operations = Vec::new();

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
                let op = match load_operation(paths, stem).await? {
                    Some(o) => o,
                    None => continue,
                };
                operations.push(op);
            }
        }
    }

    operations.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(operations)
}

/// Appends a diagnostic log event to the operation's `.events.jsonl` stream.
pub async fn append_operation_event(
    paths: &ResearchPaths,
    operation_id: &str,
    event: &StreamLogEnvelope,
) -> Result<()> {
    let path = paths.operation_events_path(operation_id)?;
    let mut serialized =
        serde_json::to_vec(event).context("Failed to serialize StreamLogEnvelope for append")?;
    serialized.push(b'\n');

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .with_context(|| format!("Failed to open events file at '{}'", path.display()))?;

    file.write_all(&serialized).await?;
    file.sync_data().await?;
    sync_parent_dir(&path)?;
    Ok(())
}

/// Reads a page of diagnostic log events using bounded line-by-line streaming.
///
/// CRITICAL INVARIANTS:
/// 1. Cursors are handled deterministically (0-indexed line offset).
/// 2. Bounded by `limit` (max lines) and `max_bytes` budget BEFORE emitting even the first event.
/// 3. Returns empty if `limit == Some(0)` or `max_bytes == Some(0)`.
/// 4. NEVER skips corruption: blank lines or malformed/partial JSON lines return descriptive errors
///    without advancing past them.
pub async fn read_operation_events(
    paths: &ResearchPaths,
    operation_id: &str,
    cursor: Option<usize>,
    limit: Option<usize>,
    max_bytes: Option<usize>,
) -> Result<(Vec<StreamLogEnvelope>, usize)> {
    let start_cursor = cursor.unwrap_or(0);
    let page_limit = limit
        .unwrap_or(DEFAULT_EVENT_PAGE_LIMIT)
        .min(MAX_EVENT_PAGE_LIMIT);
    let byte_budget = max_bytes.unwrap_or(MAX_EVENT_PAGE_BYTES);

    if page_limit == 0 || byte_budget == 0 {
        return Ok((Vec::new(), start_cursor));
    }

    let path = paths.operation_events_path(operation_id)?;
    let file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), 0));
        }
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Failed to open events file at '{}'", path.display()));
        }
    };

    use tokio::io::AsyncBufReadExt;
    let mut reader = tokio::io::BufReader::new(file);
    let mut current_line_idx = 0;
    let mut lines_consumed = 0;
    let mut accumulated_bytes = 0;
    let mut events = Vec::new();

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        if current_line_idx < start_cursor {
            current_line_idx += 1;
            line.clear();
            continue;
        }

        let line_len = line.len();

        // Reject blank or whitespace-only lines immediately without advancing past them
        if line.trim().is_empty() {
            bail!(
                "Corrupted event record at line {} in '{}': blank records are strictly prohibited",
                current_line_idx + 1,
                path.display()
            );
        }

        // Enforce byte budget BEFORE emitting even the first event!
        if accumulated_bytes + line_len > byte_budget {
            break;
        }

        // Parse record strictly as StreamLogEnvelope
        let ev = serde_json::from_str::<StreamLogEnvelope>(line.trim()).with_context(|| {
            format!(
                "Corrupted event record at line {} in '{}': record is not valid JSON",
                current_line_idx + 1,
                path.display()
            )
        })?;

        events.push(ev);
        accumulated_bytes += line_len;
        lines_consumed += 1;
        current_line_idx += 1;
        line.clear();

        if events.len() >= page_limit {
            break;
        }
    }

    let next_cursor = start_cursor + lines_consumed;
    Ok((events, next_cursor))
}
