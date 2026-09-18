//! Companion environment domain data model.
//!
//! Defined in accordance with contracts/research.schema.json and data-model.md §2.9.

use super::types::{InstanceLifecycleState, ResearchGuestId};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

/// Local companion Linux VM environment for Inferno restore and USB-over-IP bridging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionEnvironment {
    pub companion_id: Uuid,
    pub parent_guest_id: ResearchGuestId,
    pub lifecycle_state: InstanceLifecycleState,
    pub cpu_limit: u32,
    pub memory_limit_mb: u64,
    pub storage_limit_mb: u64,
    pub endpoint_socket_path: String,
    pub forwarded_ports: Vec<u16>,
    pub active_operation_ids: Vec<String>,
    pub live_guest_ids: Vec<Uuid>,
    pub active_workflows_count: u32,
    pub live_dependents_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl CompanionEnvironment {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        if self.cpu_limit < 1 {
            return Err(crate::models::error::ContractViolation::Constraint(
                "cpu_limit must be >= 1".to_string(),
            ));
        }
        if self.memory_limit_mb < 512 {
            return Err(crate::models::error::ContractViolation::Constraint(
                "memory_limit_mb must be >= 512".to_string(),
            ));
        }
        if self.storage_limit_mb < 1024 {
            return Err(crate::models::error::ContractViolation::Constraint(
                "storage_limit_mb must be >= 1024".to_string(),
            ));
        }
        if chrono::DateTime::parse_from_rfc3339(&self.created_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "created_at".to_string(),
                value: self.created_at.clone(),
            });
        }
        if chrono::DateTime::parse_from_rfc3339(&self.updated_at).is_err() {
            return Err(crate::models::error::ContractViolation::InvalidTimestamp {
                field: "updated_at".to_string(),
                value: self.updated_at.clone(),
            });
        }
        Ok(())
    }
}
