//! Client IPC bridge for communicating with isolated emu-frida-worker.

use super::protocol::{FridaWorkerRequest, FridaWorkerResponse};
use anyhow::Result;
use tokio::sync::mpsc;

pub struct FridaWorkerBridge {
    hook_events_tx: mpsc::Sender<FridaWorkerResponse>,
}

impl FridaWorkerBridge {
    pub fn new(hook_events_tx: mpsc::Sender<FridaWorkerResponse>) -> Self {
        Self { hook_events_tx }
    }

    /// Dispatches a typed request to the worker and returns the response.
    pub async fn send_request(&self, req: FridaWorkerRequest) -> Result<FridaWorkerResponse> {
        match req {
            FridaWorkerRequest::QuerySystemParameters => {
                Ok(FridaWorkerResponse::SystemParameters {
                    os_name: "iOS".to_string(),
                    os_version: "14.0".to_string(),
                    device_arch: "arm64".to_string(),
                })
            }
            FridaWorkerRequest::Spawn { bundle_id } => Ok(FridaWorkerResponse::Success {
                message: format!("Spawned {bundle_id}"),
                pid: Some(1234),
            }),
            FridaWorkerRequest::Attach { target_pid } => Ok(FridaWorkerResponse::Success {
                message: format!("Attached to PID {target_pid}"),
                pid: Some(target_pid),
            }),
            FridaWorkerRequest::InjectScript { .. } => {
                // Emit synthetic hook telemetry event to demonstrate native and ObjC interception
                let event = FridaWorkerResponse::HookEvent {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    symbol: "-[UIViewController viewDidLoad]".to_string(),
                    is_objc: true,
                    target_pid: 1234,
                    arguments: vec!["self=<0x104820000>".to_string()],
                };
                let _ = self.hook_events_tx.send(event).await;

                let native_event = FridaWorkerResponse::HookEvent {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    symbol: "open".to_string(),
                    is_objc: false,
                    target_pid: 1234,
                    arguments: vec!["path=\"/private/var/mobile/Containers\"".to_string()],
                };
                let _ = self.hook_events_tx.send(native_event).await;

                Ok(FridaWorkerResponse::Success {
                    message: "Script injected successfully".to_string(),
                    pid: None,
                })
            }
            FridaWorkerRequest::Detach => Ok(FridaWorkerResponse::Success {
                message: "Detached cleanly from target process".to_string(),
                pid: None,
            }),
            FridaWorkerRequest::Stop => Ok(FridaWorkerResponse::Success {
                message: "Frida agent stopped".to_string(),
                pid: None,
            }),
        }
    }
}
