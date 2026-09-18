//! Research domain models, entities, and value objects.

pub mod app;
pub mod artifact;
pub mod backend;
pub mod baseline;
pub mod companion;
pub mod guest;
pub mod instrumentation;
pub mod operation;
pub mod profile;
pub mod proof;
pub mod proposal;
pub mod record;
pub mod security_profile;
pub mod types;

pub use crate::protocols::gdb::KernelDebugLease;
pub use app::*;
pub use artifact::*;
pub use backend::*;
pub use baseline::*;
pub use companion::*;
pub use guest::*;
pub use instrumentation::*;
pub use operation::*;
pub use profile::*;
pub use proof::*;
pub use proposal::*;
pub use record::*;
pub use security_profile::*;
pub use types::*;
