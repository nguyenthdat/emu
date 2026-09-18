//! Device managers module

pub mod android;
pub mod common;
pub mod darwin_vm;
pub mod inferno;
pub mod ios;

// Make mock module available for integration tests
#[cfg(any(test, feature = "test-utils"))]
pub mod mock;

pub use android::AndroidManager;
pub use darwin_vm::DarwinVmManager;
pub use inferno::InfernoManager;
pub use ios::IosManager;
