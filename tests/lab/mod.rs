//! Dedicated laboratory trial test harness infrastructure for physical Apple Silicon runs.
//!
//! Excluded from automated cargo test execution. Invoked via `examples/ios_research_lab.rs`.
#![allow(unused_imports)]

pub mod config;
pub mod evidence;
pub mod gates;
pub mod runner;

pub use config::LabCohortConfig;
pub use evidence::LabEvidenceCollector;
pub use gates::ResearchGate;
pub use runner::LabRunner;
