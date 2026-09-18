//! Typed Frida worker IPC protocol definitions and stdio line-delimited JSON framing.
//!
//! Defined in accordance with research.md §D-02, plan.md §Structure, and FR-017.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum FridaWorkerRequest {
    QuerySystemParameters,
    Spawn { bundle_id: String },
    Attach { target_pid: u32 },
    InjectScript { script_source: String },
    Detach,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum FridaWorkerResponse {
    Success {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pid: Option<u32>,
    },
    SystemParameters {
        os_name: String,
        os_version: String,
        device_arch: String,
    },
    HookEvent {
        timestamp: String,
        symbol: String,
        is_objc: bool,
        target_pid: u32,
        arguments: Vec<String>,
    },
    Error {
        code: String,
        message: String,
    },
}
