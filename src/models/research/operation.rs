//! Operation record domain data model and state machine transitions.
//!
//! Defined in accordance with contracts/research.schema.json, data-model.md §3.1, and FR-046/FR-047.

use super::types::{BackendType, OperationStatus, Sha256Digest};
use crate::models::error::ErrorRecord;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Strongly-typed error for invalid operation state transitions.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum OperationTransitionError {
    #[error("Cannot transition from terminal state {current:?} to {attempted:?}")]
    TerminalState {
        current: OperationStatus,
        attempted: OperationStatus,
    },

    #[error("Invalid operation transition from {current:?} to {attempted:?}")]
    InvalidTransition {
        current: OperationStatus,
        attempted: OperationStatus,
    },

    #[error("Operation is already in terminal state {0:?}")]
    AlreadyTerminal(OperationStatus),

    #[error(
        "Cancellation can only be acknowledged when status is CancellationPending, but current status is {0:?}"
    )]
    CancellationNotPending(OperationStatus),
}

/// Strongly-typed error for operation validation failures.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum OperationValidationError {
    #[error("Operation ID '{0}' is invalid: must match pattern ^op_[0-9A-Za-z]+$")]
    InvalidOperationId(String),

    #[error("Operation type cannot be empty")]
    EmptyOperationType,

    #[error("Target instance ID '{0}' is not a valid UUID")]
    InvalidTargetInstanceId(String),

    #[error("Progress percent {0} exceeds maximum of 100")]
    ProgressOutOfRange(u8),

    #[error("Timestamp '{0}' is not a valid RFC3339 datetime")]
    InvalidTimestamp(String),

    #[error("Consumed proposal digest '{0}' is not a valid SHA-256 digest")]
    InvalidProposalDigest(String),
}

/// Durable operation journal tracking the lifecycle of an asynchronous research workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationRecord {
    pub operation_id: String,
    pub operation_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendType>,
    pub status: OperationStatus,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_percent: Option<u8>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub committed_artifacts: Vec<String>,
    #[serde(default)]
    pub residual_resources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumed_proposal_digest: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorRecord>,
}

impl OperationRecord {
    /// Constructs a new operation journal entry in `Created` status.
    pub fn new(
        operation_id: impl Into<String>,
        operation_type: impl Into<String>,
        target_instance_id: Option<String>,
        backend: Option<BackendType>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            operation_id: operation_id.into(),
            operation_type: operation_type.into(),
            target_instance_id,
            backend,
            status: OperationStatus::Created,
            phase: "preflight".to_string(),
            progress_percent: Some(0),
            created_at: now,
            started_at: None,
            completed_at: None,
            committed_artifacts: Vec::new(),
            residual_resources: Vec::new(),
            consumed_proposal_digest: None,
            error: None,
        }
    }

    /// Returns `true` if the operation is in a terminal state (`Completed`, `Failed`, or `Cancelled`).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            OperationStatus::Completed | OperationStatus::Failed | OperationStatus::Cancelled
        )
    }

    /// Performs a guarded state transition to `next` status, updating the operational phase.
    ///
    /// Valid transitions adhere to data-model.md §3.1:
    /// - `Created` -> `Preflight`, `Staged`, `Executing`, `Failed`, `CancellationPending`
    /// - `Preflight` -> `Staged`, `Executing`, `Failed`, `CancellationPending`
    /// - `Staged` -> `Executing`, `Failed`, `CancellationPending`
    /// - `Executing` -> `Verifying`, `Completed`, `Failed`, `CancellationPending`
    /// - `Verifying` -> `Completed`, `Failed`, `CancellationPending`
    /// - `CancellationPending` -> `Cancelled`, `Failed`
    /// - Transitions to `Cancelled` are strictly restricted to `CancellationPending` via worker acknowledgment.
    /// - Terminal states (`Completed`, `Failed`, `Cancelled`) reject all transitions.
    pub fn transition_to(
        &mut self,
        next: OperationStatus,
        phase: impl Into<String>,
    ) -> Result<(), OperationTransitionError> {
        if self.is_terminal() {
            return Err(OperationTransitionError::TerminalState {
                current: self.status,
                attempted: next,
            });
        }

        let is_valid = match (self.status, next) {
            // Self-transitions allowed for phase / progress updates
            (s, n) if s == n => true,

            // Transitions from Created
            (
                OperationStatus::Created,
                OperationStatus::Preflight
                | OperationStatus::Staged
                | OperationStatus::Executing
                | OperationStatus::Failed
                | OperationStatus::CancellationPending,
            ) => true,

            // Transitions from Preflight
            (
                OperationStatus::Preflight,
                OperationStatus::Staged
                | OperationStatus::Executing
                | OperationStatus::Failed
                | OperationStatus::CancellationPending,
            ) => true,

            // Transitions from Staged
            (
                OperationStatus::Staged,
                OperationStatus::Executing
                | OperationStatus::Failed
                | OperationStatus::CancellationPending,
            ) => true,

            // Transitions from Executing
            (
                OperationStatus::Executing,
                OperationStatus::Verifying
                | OperationStatus::Completed
                | OperationStatus::Failed
                | OperationStatus::CancellationPending,
            ) => true,

            // Transitions from Verifying
            (
                OperationStatus::Verifying,
                OperationStatus::Completed
                | OperationStatus::Failed
                | OperationStatus::CancellationPending,
            ) => true,

            // Transitions from CancellationPending (requires worker acknowledgment)
            (
                OperationStatus::CancellationPending,
                OperationStatus::Cancelled | OperationStatus::Failed,
            ) => true,

            _ => false,
        };

        if !is_valid {
            return Err(OperationTransitionError::InvalidTransition {
                current: self.status,
                attempted: next,
            });
        }

        self.status = next;
        self.phase = phase.into();
        Ok(())
    }

    /// Requests cancellation of this operation.
    ///
    /// CRITICAL INVARIANT (FR-046, data-model.md §1.2):
    /// Cancel only records `CancellationPending` until the worker acknowledges the signal.
    /// It never jumps directly to `Cancelled` from an executing background worker.
    pub fn request_cancellation(&mut self) -> Result<(), OperationTransitionError> {
        if self.is_terminal() {
            return Err(OperationTransitionError::AlreadyTerminal(self.status));
        }
        if self.status == OperationStatus::CancellationPending {
            return Ok(());
        }
        self.transition_to(OperationStatus::CancellationPending, "cancelling")
    }

    /// Acknowledges cancellation after the worker process cleans up residual resources.
    pub fn acknowledge_cancellation(&mut self) -> Result<(), OperationTransitionError> {
        if self.status != OperationStatus::CancellationPending {
            return Err(OperationTransitionError::CancellationNotPending(
                self.status,
            ));
        }
        self.status = OperationStatus::Cancelled;
        self.phase = "cancelled".to_string();
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    /// Marks the operation as started (status `Executing`), setting `started_at`.
    pub fn start(&mut self, phase: impl Into<String>) -> Result<(), OperationTransitionError> {
        self.transition_to(OperationStatus::Executing, phase)?;
        if self.started_at.is_none() {
            self.started_at = Some(chrono::Utc::now().to_rfc3339());
        }
        Ok(())
    }

    /// Completes the operation successfully (status `Completed`), setting `completed_at` and 100%.
    pub fn complete(&mut self, phase: impl Into<String>) -> Result<(), OperationTransitionError> {
        self.transition_to(OperationStatus::Completed, phase)?;
        self.progress_percent = Some(100);
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    /// Fails the operation with structured error details, setting `completed_at`.
    ///
    /// CRITICAL INVARIANT (FR-047):
    /// Failed operations are marked `Failed`. Automatic retries on mutating steps are strictly prohibited.
    pub fn fail(
        &mut self,
        error: ErrorRecord,
        phase: impl Into<String>,
    ) -> Result<(), OperationTransitionError> {
        self.transition_to(OperationStatus::Failed, phase)?;
        self.error = Some(error);
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    /// Records the consumed proposal digest for two-step safety gate authorization tracking.
    pub fn record_consumed_proposal(&mut self, digest: Sha256Digest) {
        self.consumed_proposal_digest = Some(digest);
    }

    /// Validates internal schema invariants of the OperationRecord.
    pub fn validate(&self) -> Result<(), OperationValidationError> {
        if !is_valid_operation_id(&self.operation_id) {
            return Err(OperationValidationError::InvalidOperationId(
                self.operation_id.clone(),
            ));
        }

        if self.operation_type.trim().is_empty() {
            return Err(OperationValidationError::EmptyOperationType);
        }

        if let Some(inst_id) = &self.target_instance_id {
            if uuid::Uuid::parse_str(inst_id).is_err() {
                return Err(OperationValidationError::InvalidTargetInstanceId(
                    inst_id.clone(),
                ));
            }
        }

        if let Some(pct) = self.progress_percent {
            if pct > 100 {
                return Err(OperationValidationError::ProgressOutOfRange(pct));
            }
        }

        if chrono::DateTime::parse_from_rfc3339(&self.created_at).is_err() {
            return Err(OperationValidationError::InvalidTimestamp(
                self.created_at.clone(),
            ));
        }

        if let Some(started) = &self.started_at {
            if chrono::DateTime::parse_from_rfc3339(started).is_err() {
                return Err(OperationValidationError::InvalidTimestamp(started.clone()));
            }
        }

        if let Some(completed) = &self.completed_at {
            if chrono::DateTime::parse_from_rfc3339(completed).is_err() {
                return Err(OperationValidationError::InvalidTimestamp(
                    completed.clone(),
                ));
            }
        }

        if let Some(digest) = &self.consumed_proposal_digest {
            if !is_valid_sha256_digest(digest.as_str()) {
                return Err(OperationValidationError::InvalidProposalDigest(
                    digest.as_str().to_string(),
                ));
            }
        }

        Ok(())
    }
}

fn is_valid_operation_id(id: &str) -> bool {
    if !id.starts_with("op_") || id.len() <= 3 {
        return false;
    }
    id[3..].chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_valid_sha256_digest(s: &str) -> bool {
    if !s.starts_with("sha256:") || s.len() != 71 {
        return false;
    }
    s[7..].chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}
