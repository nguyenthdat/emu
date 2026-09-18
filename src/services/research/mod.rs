//! Research service coordinators, reconcilers, and provenance tracking.

pub mod coordinator;
pub mod guest_fs;
pub mod image_prep;
pub mod provenance;
pub mod reconciler;
pub mod runtime;

pub mod verifier;

pub use coordinator::ResearchCoordinator;
pub use guest_fs::GuestFileSystem;
pub use image_prep::ImagePreparationWorker;
pub use provenance::ProvenanceTracker;
pub use reconciler::OperationReconciler;
pub use runtime::{BackendLaunchConfig, RuntimeDescriptor, SupervisorAction, SupervisorClient};
pub use verifier::RootVerifier;
