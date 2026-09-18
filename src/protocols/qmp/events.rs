//! Asynchronous QMP event parsing and broadcast channel.
//!
//! Defined in accordance with RFC 9003 and plan.md §Structure.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QmpLifecycleEvent {
    Shutdown { guest: bool, reason: Option<String> },
    Reset { guest: bool, reason: Option<String> },
    Stop,
    Resume,
    Other { event: String },
}

impl QmpLifecycleEvent {
    /// Parses an incoming QMP event string and data payload into a typed lifecycle event.
    pub fn from_event_name_and_data(event: &str, data: Option<&serde_json::Value>) -> Self {
        match event {
            "SHUTDOWN" => {
                let guest = data
                    .and_then(|d| d.get("guest"))
                    .and_then(|g| g.as_bool())
                    .unwrap_or(false);
                let reason = data
                    .and_then(|d| d.get("reason"))
                    .and_then(|r| r.as_str())
                    .map(String::from);
                Self::Shutdown { guest, reason }
            }
            "RESET" => {
                let guest = data
                    .and_then(|d| d.get("guest"))
                    .and_then(|g| g.as_bool())
                    .unwrap_or(false);
                let reason = data
                    .and_then(|d| d.get("reason"))
                    .and_then(|r| r.as_str())
                    .map(String::from);
                Self::Reset { guest, reason }
            }
            "STOP" => Self::Stop,
            "RESUME" => Self::Resume,
            other => Self::Other {
                event: other.to_string(),
            },
        }
    }

    /// Returns `true` if this event indicates guest shutdown.
    pub fn is_shutdown(&self) -> bool {
        matches!(self, Self::Shutdown { .. })
    }

    /// Returns `true` if this event indicates vCPU stop.
    pub fn is_stop(&self) -> bool {
        matches!(self, Self::Stop)
    }

    /// Returns `true` if this event indicates vCPU resume.
    pub fn is_resume(&self) -> bool {
        matches!(self, Self::Resume)
    }
}

pub type QmpEventSender = broadcast::Sender<QmpLifecycleEvent>;
pub type QmpEventReceiver = broadcast::Receiver<QmpLifecycleEvent>;

pub fn create_event_channel(capacity: usize) -> (QmpEventSender, QmpEventReceiver) {
    broadcast::channel(capacity)
}
