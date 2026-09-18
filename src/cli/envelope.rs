//! Standard JSON output envelope (stdout) and stream log envelope (stderr).
//!
//! Governed by contracts/cli.md §3 and contracts/research.schema.json.

use crate::constants::research::{
    CANONICAL_ENVELOPE_SCHEMA, EXIT_AUTH_REFUSED, EXIT_CANCELLED, EXIT_CONFLICT,
    EXIT_INVALID_INPUT, EXIT_RUNTIME_FAILURE, EXIT_SUCCESS, EXIT_TIMEOUT, EXIT_UNSUPPORTED,
};
use crate::models::error::{ContractViolation, ErrorRecord};
use serde::{Deserialize, Serialize};

/// High-level operational status classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeStatus {
    Success,
    AlreadySatisfied,
    Accepted,
    Failed,
    Rejected,
    TimedOut,
    Cancelled,
}

impl EnvelopeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::AlreadySatisfied => "already_satisfied",
            Self::Accepted => "accepted",
            Self::Failed => "failed",
            Self::Rejected => "rejected",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Exact outcome identifiers matching the Process Exit Code Contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeOutcome {
    Completed,
    AlreadySatisfied,
    ProposalCreated,
    AuthRefused,
    InvalidInput,
    Unsupported,
    Conflict,
    Timeout,
    CancellationPending,
    Cancelled,
    ExecutionFailed,
}

impl EnvelopeOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::AlreadySatisfied => "already_satisfied",
            Self::ProposalCreated => "proposal_created",
            Self::AuthRefused => "auth_refused",
            Self::InvalidInput => "invalid_input",
            Self::Unsupported => "unsupported",
            Self::Conflict => "conflict",
            Self::Timeout => "timeout",
            Self::CancellationPending => "cancellation_pending",
            Self::Cancelled => "cancelled",
            Self::ExecutionFailed => "execution_failed",
        }
    }

    pub fn default_exit_code(&self) -> i32 {
        match self {
            Self::Completed | Self::AlreadySatisfied | Self::ProposalCreated => EXIT_SUCCESS,
            Self::ExecutionFailed => EXIT_RUNTIME_FAILURE,
            Self::InvalidInput => EXIT_INVALID_INPUT,
            Self::Unsupported => EXIT_UNSUPPORTED,
            Self::AuthRefused => EXIT_AUTH_REFUSED,
            Self::Conflict => EXIT_CONFLICT,
            Self::Timeout | Self::CancellationPending => EXIT_TIMEOUT,
            Self::Cancelled => EXIT_CANCELLED,
        }
    }
}

/// Finite, structured JSON envelope printed to stdout when `--json` is active.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputEnvelope {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub status: String,
    pub outcome: String,
    pub operation_id: String,
    pub data: Option<serde_json::Value>,
    pub error: Option<ErrorRecord>,
}

impl OutputEnvelope {
    pub fn new(
        status: EnvelopeStatus,
        outcome: EnvelopeOutcome,
        operation_id: impl Into<String>,
        data: Option<serde_json::Value>,
        error: Option<ErrorRecord>,
    ) -> Self {
        Self {
            schema: Some(CANONICAL_ENVELOPE_SCHEMA.to_string()),
            status: status.as_str().to_string(),
            outcome: outcome.as_str().to_string(),
            operation_id: operation_id.into(),
            data,
            error,
        }
    }

    pub fn success(operation_id: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        Self::new(
            EnvelopeStatus::Success,
            EnvelopeOutcome::Completed,
            operation_id,
            data,
            None,
        )
    }

    pub fn already_satisfied(
        operation_id: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            EnvelopeStatus::AlreadySatisfied,
            EnvelopeOutcome::AlreadySatisfied,
            operation_id,
            data,
            None,
        )
    }

    pub fn proposal_created(
        operation_id: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            EnvelopeStatus::Accepted,
            EnvelopeOutcome::ProposalCreated,
            operation_id,
            data,
            None,
        )
    }

    pub fn execution_failed(
        operation_id: impl Into<String>,
        error: ErrorRecord,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            EnvelopeStatus::Failed,
            EnvelopeOutcome::ExecutionFailed,
            operation_id,
            data,
            Some(error),
        )
    }

    pub fn rejected(
        outcome: EnvelopeOutcome,
        operation_id: impl Into<String>,
        error: ErrorRecord,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            EnvelopeStatus::Rejected,
            outcome,
            operation_id,
            data,
            Some(error),
        )
    }

    pub fn timed_out(
        operation_id: impl Into<String>,
        error: ErrorRecord,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            EnvelopeStatus::TimedOut,
            EnvelopeOutcome::Timeout,
            operation_id,
            data,
            Some(error),
        )
    }

    pub fn cancellation_pending(
        operation_id: impl Into<String>,
        error: ErrorRecord,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            EnvelopeStatus::TimedOut,
            EnvelopeOutcome::CancellationPending,
            operation_id,
            data,
            Some(error),
        )
    }

    pub fn cancelled(
        operation_id: impl Into<String>,
        error: ErrorRecord,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            EnvelopeStatus::Cancelled,
            EnvelopeOutcome::Cancelled,
            operation_id,
            data,
            Some(error),
        )
    }

    pub fn validate(&self) -> Result<(), ContractViolation> {
        if !self.operation_id.starts_with("op_")
            || self.operation_id.len() <= 3
            || !self.operation_id["op_".len()..]
                .chars()
                .all(|c| c.is_ascii_alphanumeric())
        {
            return Err(ContractViolation::InvalidEnvelope(format!(
                "operation_id '{}' must match pattern '^op_[0-9A-Za-z]+$'",
                self.operation_id
            )));
        }

        if let Some(d) = &self.data {
            if !d.is_null() && !d.is_object() {
                return Err(ContractViolation::InvalidEnvelope(format!(
                    "data must be an object or null, got: {d}"
                )));
            }
        }

        match self.status.as_str() {
            "success" => {
                if !matches!(
                    self.outcome.as_str(),
                    "completed" | "already_satisfied" | "proposal_created"
                ) {
                    return Err(ContractViolation::InvalidEnvelope(format!(
                        "status 'success' incompatible with outcome '{}'",
                        self.outcome
                    )));
                }
                if self.error.is_some() {
                    return Err(ContractViolation::InvalidEnvelope(
                        "status 'success' requires error to be null".to_string(),
                    ));
                }
            }
            "already_satisfied" => {
                if self.outcome != "already_satisfied" {
                    return Err(ContractViolation::InvalidEnvelope(format!(
                        "status 'already_satisfied' requires outcome 'already_satisfied', got '{}'",
                        self.outcome
                    )));
                }
                if self.error.is_some() {
                    return Err(ContractViolation::InvalidEnvelope(
                        "status 'already_satisfied' requires error to be null".to_string(),
                    ));
                }
            }
            "accepted" => {
                if self.outcome != "proposal_created" {
                    return Err(ContractViolation::InvalidEnvelope(format!(
                        "status 'accepted' requires outcome 'proposal_created', got '{}'",
                        self.outcome
                    )));
                }
                if self.error.is_some() {
                    return Err(ContractViolation::InvalidEnvelope(
                        "status 'accepted' requires error to be null".to_string(),
                    ));
                }
            }
            "failed" => {
                if self.outcome != "execution_failed" {
                    return Err(ContractViolation::InvalidEnvelope(format!(
                        "status 'failed' requires outcome 'execution_failed', got '{}'",
                        self.outcome
                    )));
                }
                let err = self.error.as_ref().ok_or_else(|| {
                    ContractViolation::InvalidEnvelope(
                        "status 'failed' requires non-null error ErrorRecord".to_string(),
                    )
                })?;
                err.validate()?;
            }
            "rejected" => {
                if !matches!(
                    self.outcome.as_str(),
                    "invalid_input" | "unsupported" | "auth_refused" | "conflict"
                ) {
                    return Err(ContractViolation::InvalidEnvelope(format!(
                        "status 'rejected' requires outcome invalid_input|unsupported|auth_refused|conflict, got '{}'",
                        self.outcome
                    )));
                }
                let err = self.error.as_ref().ok_or_else(|| {
                    ContractViolation::InvalidEnvelope(
                        "status 'rejected' requires non-null error ErrorRecord".to_string(),
                    )
                })?;
                err.validate()?;
            }
            "timed_out" => {
                if !matches!(self.outcome.as_str(), "timeout" | "cancellation_pending") {
                    return Err(ContractViolation::InvalidEnvelope(format!(
                        "status 'timed_out' requires outcome timeout|cancellation_pending, got '{}'",
                        self.outcome
                    )));
                }
                let err = self.error.as_ref().ok_or_else(|| {
                    ContractViolation::InvalidEnvelope(
                        "status 'timed_out' requires non-null error ErrorRecord".to_string(),
                    )
                })?;
                err.validate()?;
            }
            "cancelled" => {
                if self.outcome != "cancelled" {
                    return Err(ContractViolation::InvalidEnvelope(format!(
                        "status 'cancelled' requires outcome 'cancelled', got '{}'",
                        self.outcome
                    )));
                }
                let err = self.error.as_ref().ok_or_else(|| {
                    ContractViolation::InvalidEnvelope(
                        "status 'cancelled' requires non-null error ErrorRecord".to_string(),
                    )
                })?;
                err.validate()?;
            }
            other => {
                return Err(ContractViolation::InvalidEnvelope(format!(
                    "unknown envelope status: '{other}'"
                )));
            }
        }
        Ok(())
    }

    pub fn exit_code(&self) -> i32 {
        if let Some(err) = &self.error {
            err.exit_code()
        } else {
            match self.outcome.as_str() {
                "completed" | "already_satisfied" | "proposal_created" => EXIT_SUCCESS,
                "execution_failed" => EXIT_RUNTIME_FAILURE,
                "invalid_input" => EXIT_INVALID_INPUT,
                "unsupported" => EXIT_UNSUPPORTED,
                "auth_refused" => EXIT_AUTH_REFUSED,
                "conflict" => EXIT_CONFLICT,
                "timeout" | "cancellation_pending" => EXIT_TIMEOUT,
                "cancelled" => EXIT_CANCELLED,
                _ => EXIT_RUNTIME_FAILURE,
            }
        }
    }

    pub fn print_stdout(&self) -> anyhow::Result<()> {
        self.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
        let serialized = serde_json::to_string(self)?;
        println!("{serialized}");
        Ok(())
    }
}

/// Allowed stream log levels in research.schema.json#/definitions/StreamLogEnvelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DEBUG => "DEBUG",
            Self::INFO => "INFO",
            Self::WARN => "WARN",
            Self::ERROR => "ERROR",
        }
    }
}

/// Allowed stream log phases in research.schema.json#/definitions/StreamLogEnvelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogPhase {
    Preflight,
    Staging,
    Booting,
    Verifying,
    Settled,
    Recovering,
    Stopping,
    Cleanup,
    Teardown,
    Paused,
    Executing,
    Unknown,
}

impl LogPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preflight => "preflight",
            Self::Staging => "staging",
            Self::Booting => "booting",
            Self::Verifying => "verifying",
            Self::Settled => "settled",
            Self::Recovering => "recovering",
            Self::Stopping => "stopping",
            Self::Cleanup => "cleanup",
            Self::Teardown => "teardown",
            Self::Paused => "paused",
            Self::Executing => "executing",
            Self::Unknown => "unknown",
        }
    }
}

/// Operational progress log record streamed to stderr when `--json` is active.
/// Conforms strictly to contracts/research.schema.json#/definitions/StreamLogEnvelope.
/// Note: additionalProperties: false, no $schema permitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamLogEnvelope {
    pub timestamp: String,
    pub level: String,
    pub event: String,
    pub phase: String,
    pub message: String,
    pub operation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl StreamLogEnvelope {
    pub fn new(
        level: impl AsRef<str>,
        event: impl Into<String>,
        phase: impl AsRef<str>,
        message: impl Into<String>,
        operation_id: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        let details_val = match details {
            Some(serde_json::Value::Object(map)) => Some(serde_json::Value::Object(map)),
            Some(other) => {
                let mut map = serde_json::Map::new();
                map.insert("value".to_string(), other);
                Some(serde_json::Value::Object(map))
            }
            None => None,
        };
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.as_ref().to_uppercase(),
            event: event.into(),
            phase: phase.as_ref().to_lowercase(),
            message: message.into(),
            operation_id: operation_id.into(),
            details: details_val,
        }
    }

    pub fn validate(&self) -> Result<(), ContractViolation> {
        if chrono::DateTime::parse_from_rfc3339(&self.timestamp).is_err() {
            return Err(ContractViolation::InvalidTimestamp {
                field: "timestamp".to_string(),
                value: self.timestamp.clone(),
            });
        }
        match self.level.as_str() {
            "DEBUG" | "INFO" | "WARN" | "ERROR" => {}
            other => {
                return Err(ContractViolation::FieldViolation {
                    field: "level".to_string(),
                    reason: format!("invalid log level '{other}': must be DEBUG|INFO|WARN|ERROR"),
                });
            }
        }
        match self.phase.as_str() {
            "preflight" | "staging" | "booting" | "verifying" | "settled" | "recovering"
            | "stopping" | "cleanup" | "teardown" | "paused" | "executing" | "unknown" => {}
            other => {
                return Err(ContractViolation::FieldViolation {
                    field: "phase".to_string(),
                    reason: format!("invalid log phase '{other}'"),
                });
            }
        }
        if !self.operation_id.starts_with("op_")
            || self.operation_id.len() <= 3
            || !self.operation_id["op_".len()..]
                .chars()
                .all(|c| c.is_ascii_alphanumeric())
        {
            return Err(ContractViolation::InvalidEnvelope(format!(
                "operation_id '{}' must match pattern '^op_[0-9A-Za-z]+$'",
                self.operation_id
            )));
        }
        if let Some(d) = &self.details {
            if !d.is_object() {
                return Err(ContractViolation::FieldViolation {
                    field: "details".to_string(),
                    reason: "details must be a JSON object".to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn print_stderr(&self) -> anyhow::Result<()> {
        self.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
        let serialized = serde_json::to_string(self)?;
        eprintln!("{serialized}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_envelope_success_serialization() {
        let env = OutputEnvelope::success(
            "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
            Some(serde_json::json!({"status": "ok"})),
        );
        let json_str = serde_json::to_string(&env).unwrap();
        assert!(json_str.contains("\"status\":\"success\""));
        assert!(json_str.contains("\"outcome\":\"completed\""));
        assert!(json_str.contains("\"operation_id\":\"op_01J8F9W2Z0K4M1N5P6Q7R8S9T0\""));
        assert_eq!(env.exit_code(), 0);
    }

    #[test]
    fn test_stream_log_envelope_serialization() {
        let log = StreamLogEnvelope::new(
            "INFO",
            "guest_boot_started",
            "booting",
            "Guest boot sequence initiated",
            "op_01J8F9W2Z0K4M1N5P6Q7R8S9T0",
            None,
        );
        let json_str = serde_json::to_string(&log).unwrap();
        assert!(json_str.contains("\"level\":\"INFO\""));
        assert!(json_str.contains("\"phase\":\"booting\""));
    }
}
