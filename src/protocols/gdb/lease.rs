//! Kernel debug lease domain model and concurrency contract.
//!
//! Defined in accordance with FR-027, FR-028, FR-029, D-05, and contracts/research.schema.json.

use crate::models::research::{DebugLeaseState, LeaseId, ResearchGuestId};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Exclusive lease for kernel debugging session to prevent concurrent debugger interference
/// and hypervisor runstate corruption.
///
/// Matches canonical `KernelDebugLease` schema with `additionalProperties: false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelDebugLease {
    pub lease_id: LeaseId,
    pub guest_id: ResearchGuestId,
    pub session_id: String,
    pub client_identity: String,
    pub lease_state: DebugLeaseState,
    pub acquired_at: String,
    pub heartbeat_at: String,
    pub qmp_cont_blocked: bool,
    pub gdb_endpoint_path: String,
}

impl KernelDebugLease {
    /// Creates a new active `KernelDebugLease` with default socket endpoint.
    pub fn new(guest_id: ResearchGuestId, client_identity: impl Into<String>) -> Self {
        let endpoint = format!("/tmp/emu-{guest_id}/gdb.sock");
        Self::with_endpoint(guest_id, client_identity, endpoint)
    }

    /// Creates a new active `KernelDebugLease` with an explicit socket endpoint path.
    pub fn with_endpoint(
        guest_id: ResearchGuestId,
        client_identity: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            lease_id: LeaseId::new(),
            guest_id,
            session_id: format!("sess_{}", uuid::Uuid::new_v4().simple()),
            client_identity: client_identity.into(),
            lease_state: DebugLeaseState::Active,
            acquired_at: now.clone(),
            heartbeat_at: now,
            qmp_cont_blocked: true,
            gdb_endpoint_path: endpoint.into(),
        }
    }

    /// Returns `true` if the lease is in `Active` state.
    pub fn is_active(&self) -> bool {
        self.lease_state == DebugLeaseState::Active
    }

    /// Returns `true` if the lease is paused due to client disconnection (`DisconnectedPaused`).
    pub fn is_disconnected_paused(&self) -> bool {
        self.lease_state == DebugLeaseState::DisconnectedPaused
    }

    /// Updates the heartbeat timestamp of this lease.
    pub fn heartbeat(&mut self) {
        self.heartbeat_at = chrono::Utc::now().to_rfc3339();
    }

    /// Transitions this lease to `DisconnectedPaused`.
    ///
    /// FR-029: Preserves paused state on client disconnect; silent resumption is prohibited.
    pub fn disconnect_paused(&mut self) {
        self.lease_state = DebugLeaseState::DisconnectedPaused;
        self.heartbeat();
    }

    /// Explicitly releases this lease.
    pub fn release(&mut self) {
        self.lease_state = DebugLeaseState::Released;
        self.heartbeat();
    }

    /// Marks this lease as expired.
    pub fn expire(&mut self) {
        self.lease_state = DebugLeaseState::Expired;
        self.heartbeat();
    }
}

/// Thread-safe in-memory registry enforcing exclusive `KernelDebugLease` concurrency.
#[derive(Debug, Default)]
pub struct KernelDebugLeaseRegistry {
    leases: RwLock<HashMap<ResearchGuestId, KernelDebugLease>>,
}

impl KernelDebugLeaseRegistry {
    /// Creates a new empty lease registry.
    pub fn new() -> Self {
        Self {
            leases: RwLock::new(HashMap::new()),
        }
    }

    /// Attempts to acquire an exclusive debug lease for a target guest.
    ///
    /// Fails with `DEBUG_LEASE_CONFLICT` if an active or disconnected-paused lease is already held.
    pub fn acquire(
        self: &Arc<Self>,
        guest_id: ResearchGuestId,
        client_identity: impl Into<String>,
        endpoint: Option<String>,
    ) -> Result<KernelDebugLeaseGuard> {
        let mut map = self
            .leases
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire lease registry lock"))?;

        if let Some(existing) = map.get(&guest_id) {
            if existing.is_active() || existing.is_disconnected_paused() {
                bail!(
                    "DEBUG_LEASE_CONFLICT: guest {} already has an active kernel debug lease held by session '{}'",
                    guest_id,
                    existing.session_id
                );
            }
        }

        let lease = match endpoint {
            Some(ep) => KernelDebugLease::with_endpoint(guest_id, client_identity, ep),
            None => KernelDebugLease::new(guest_id, client_identity),
        };

        map.insert(guest_id, lease.clone());

        Ok(KernelDebugLeaseGuard {
            registry: Arc::clone(self),
            lease: Some(lease),
            auto_disconnect_on_drop: true,
        })
    }

    /// Returns a copy of the lease for `guest_id` if present.
    pub fn get_lease(&self, guest_id: &ResearchGuestId) -> Option<KernelDebugLease> {
        self.leases.read().ok()?.get(guest_id).cloned()
    }

    /// Checks whether QMP `cont` is allowed on the target guest.
    ///
    /// D-05: While a `KernelDebugLease` is active or disconnected_paused, QMP `cont` is
    /// strictly blocked to prevent hypervisor runstate corruption.
    pub fn check_qmp_cont_allowed(&self, guest_id: &ResearchGuestId) -> Result<()> {
        if let Some(lease) = self.get_lease(guest_id) {
            if lease.is_active() || lease.is_disconnected_paused() {
                bail!(
                    "DEBUG_LEASE_CONFLICT: QMP cont blocked by active kernel debug lease {} (session '{}')",
                    lease.lease_id,
                    lease.session_id
                );
            }
        }
        Ok(())
    }

    /// Updates heartbeat timestamp for an active lease.
    pub fn heartbeat(&self, guest_id: &ResearchGuestId) -> Result<()> {
        let mut map = self
            .leases
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire lease registry lock"))?;

        if let Some(lease) = map.get_mut(guest_id) {
            lease.heartbeat();
            Ok(())
        } else {
            bail!("No active kernel debug lease found for guest {guest_id}");
        }
    }

    /// Transitions an active lease to `DisconnectedPaused`.
    pub fn disconnect_paused(&self, guest_id: &ResearchGuestId) -> Result<()> {
        let mut map = self
            .leases
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire lease registry lock"))?;

        if let Some(lease) = map.get_mut(guest_id) {
            lease.disconnect_paused();
            Ok(())
        } else {
            bail!("No active kernel debug lease found for guest {guest_id}");
        }
    }

    /// Releases an active lease in the registry.
    pub fn release(&self, guest_id: &ResearchGuestId) -> Option<KernelDebugLease> {
        let mut map = self.leases.write().ok()?;
        if let Some(lease) = map.get_mut(guest_id) {
            lease.release();
            Some(lease.clone())
        } else {
            None
        }
    }
}

/// RAII guard representing ownership of an exclusive `KernelDebugLease`.
///
/// On drop, if not explicitly released, transitions the lease to `DisconnectedPaused`
/// in accordance with FR-029 / SC-006 to prevent silent guest execution resumption.
#[derive(Debug)]
pub struct KernelDebugLeaseGuard {
    registry: Arc<KernelDebugLeaseRegistry>,
    lease: Option<KernelDebugLease>,
    auto_disconnect_on_drop: bool,
}

impl KernelDebugLeaseGuard {
    /// Returns a reference to the active `KernelDebugLease`.
    pub fn lease(&self) -> &KernelDebugLease {
        self.lease.as_ref().expect("Lease guard must contain lease")
    }

    /// Updates the heartbeat timestamp for this lease.
    pub fn heartbeat(&mut self) -> Result<()> {
        if let Some(lease) = &mut self.lease {
            lease.heartbeat();
            self.registry.heartbeat(&lease.guest_id)?;
        }
        Ok(())
    }

    /// Transitions the lease to `DisconnectedPaused`.
    pub fn disconnect_paused(&mut self) -> Result<()> {
        if let Some(lease) = &mut self.lease {
            lease.disconnect_paused();
            self.registry.disconnect_paused(&lease.guest_id)?;
        }
        Ok(())
    }

    /// Explicitly releases the lease and consumes the guard.
    pub fn release(mut self) -> Result<KernelDebugLease> {
        self.auto_disconnect_on_drop = false;
        if let Some(mut lease) = self.lease.take() {
            lease.release();
            self.registry.release(&lease.guest_id);
            Ok(lease)
        } else {
            bail!("Lease already released");
        }
    }
}

impl Drop for KernelDebugLeaseGuard {
    fn drop(&mut self) {
        if self.auto_disconnect_on_drop {
            if let Some(lease) = &self.lease {
                if lease.is_active() {
                    let _ = self.registry.disconnect_paused(&lease.guest_id);
                }
            }
        }
    }
}
