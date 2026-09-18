//! Storage persistence layer for research backends and shared resources.
//!
//! Provides transactional filesystem paths, atomic write staging, and cross-process
//! advisory locking for guest instances and operations.

pub mod artifacts;
pub mod atomic;
pub mod baselines;
pub mod instances;
pub mod lock;
pub mod operations;
pub mod paths;
pub mod profiles;
pub mod proposals;
pub mod records;
pub mod security_profiles;

pub use artifacts::*;
pub use atomic::write_atomic;
pub use baselines::*;
pub use instances::*;
pub use lock::{AdvisoryLock, LockError};
pub use operations::*;
pub use paths::{PathError, ResearchPaths, RuntimeDirectory, validate_path_component};
pub use profiles::*;
pub use proposals::*;
pub use records::*;
pub use security_profiles::*;
