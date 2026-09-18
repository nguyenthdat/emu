//! Dynamic instrumentation session domain data model.
//!
//! Defined in accordance with contracts/research.schema.json and data-model.md §2.7.

use super::types::{
    BootSessionId, FridaAttachmentState, HookStatus, ResearchGuestId, Sha256Digest,
};
use serde::{Deserialize, Serialize};

/// Active or historical Frida dynamic instrumentation session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentationSession {
    pub session_id: String,
    pub guest_id: ResearchGuestId,
    pub boot_session_id: BootSessionId,
    pub target_process_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_bundle_id: Option<String>,
    pub agent_package_version: String,
    pub agent_integrity_hash: Sha256Digest,
    pub tls_server_cert_pinned: bool,
    pub session_token_configured: bool,
    pub injected_script_hashes: Vec<Sha256Digest>,
    pub hook_status: HookStatus,
    pub attachment_state: FridaAttachmentState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attached_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detached_at: Option<String>,
    pub captured_hook_events_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_process_id: Option<i64>,
    pub control_process_hooked: bool,
    pub diagnostics: Vec<String>,
}

impl InstrumentationSession {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        self.agent_integrity_hash.validate()?;
        if self.agent_package_version != "17.18.0" {
            return Err(crate::models::error::ContractViolation::FieldViolation {
                field: "agent_package_version".to_string(),
                reason: format!("must be '17.18.0', got '{}'", self.agent_package_version),
            });
        }
        if self.control_process_hooked {
            return Err(crate::models::error::ContractViolation::Constraint(
                "control_process_hooked must be false (attaching to control processes is forbidden)".to_string(),
            ));
        }
        for h in &self.injected_script_hashes {
            h.validate()?;
        }
        if let Some(ts) = &self.attached_at {
            if chrono::DateTime::parse_from_rfc3339(ts).is_err() {
                return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                    field: "attached_at".to_string(),
                    value: ts.clone(),
                });
            }
        }
        if let Some(ts) = &self.detached_at {
            if chrono::DateTime::parse_from_rfc3339(ts).is_err() {
                return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                    field: "detached_at".to_string(),
                    value: ts.clone(),
                });
            }
        }
        Ok(())
    }
}
