//! Shared identifier newtypes, domain enums, and auxiliary value objects
//! for iOS Root VM and Darwin Security Research Backends.
//!
//! Defined in accordance with contracts/research.schema.json and data-model.md.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ============================================================================
// Strongly-Typed Identifier Newtypes
// ============================================================================

macro_rules! define_uuid_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
            pub fn simple(&self) -> uuid::fmt::Simple {
                self.0.simple()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }
    };
}

macro_rules! define_string_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

define_uuid_newtype!(
    ResearchGuestId,
    "Unique identifier for a research guest instance."
);
define_uuid_newtype!(
    BootSessionId,
    "Unique identifier for a guest boot session (invalidated on reboot)."
);
define_uuid_newtype!(
    EvidenceId,
    "Unique identifier for root proof or verification evidence."
);
define_uuid_newtype!(ProposalId, "Unique identifier for a mutation proposal.");
define_uuid_newtype!(
    LeaseId,
    "Unique identifier for an exclusive kernel debug lease."
);
define_uuid_newtype!(
    RecordId,
    "Unique identifier for an immutable experiment trial record."
);

define_string_newtype!(ArtifactId, "Identifier for an image or firmware artifact.");
define_string_newtype!(
    ApplicationId,
    "Identifier for an imported or installed application artifact."
);
define_string_newtype!(
    InstrumentationSessionId,
    "Identifier for an active Frida instrumentation session."
);
define_string_newtype!(
    RecoveryBaselineId,
    "Identifier for a clean recovery baseline snapshot."
);
/// Cryptographic SHA-256 digest string (`sha256:<hex>`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256Digest(pub String);

impl Sha256Digest {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        let s = self.0.as_str();
        if !s.starts_with("sha256:") {
            return Err(crate::models::error::ContractViolation::InvalidDigest(
                s.to_string(),
            ));
        }
        let hex_part = &s["sha256:".len()..];
        if hex_part.len() != 64
            || !hex_part
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(crate::models::error::ContractViolation::InvalidDigest(
                s.to_string(),
            ));
        }
        Ok(())
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Sha256Digest {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Sha256Digest {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::str::FromStr for Sha256Digest {
    type Err = crate::models::error::ContractViolation;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let digest = Self::new(s);
        digest.validate()?;
        Ok(digest)
    }
}

// ============================================================================
// Domain Enums (contracts/research.schema.json)
// ============================================================================

/// Target virtualization backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    #[serde(rename = "darwin-vm")]
    DarwinVm,
    #[serde(rename = "Inferno")]
    Inferno,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DarwinVm => write!(f, "darwin-vm"),
            Self::Inferno => write!(f, "Inferno"),
        }
    }
}

/// Lifecycle state of a research guest instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceLifecycleState {
    Stopped,
    Booting,
    Running,
    Paused,
    Recovering,
    Stopping,
    Error,
    Unknown,
}

/// CPU architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CpuArchitecture {
    Arm64,
}

/// Privilege state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeState {
    Root,
    Unprivileged,
}

/// State of root privilege verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootVerificationState {
    Unverified,
    Verifying,
    Verified,
    Invalidated,
}

/// Dynamic instrumentation runtime attachment state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FridaAttachmentState {
    Detached,
    Preparing,
    Ready,
    Attached,
    DetachedClean,
    Failed,
    Invalidated,
}

/// State of an exclusive kernel debug lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugLeaseState {
    Active,
    Released,
    Expired,
    DisconnectedPaused,
}

/// Backend or platform support status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportStatus {
    Supported,
    Unsupported,
    Experimental,
}

/// Type of research image artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageArtifactType {
    Kernelcache,
    Ramdisk,
    Devicetree,
    Trustcache,
    RootDisk,
    SptmFirmware,
    TxmFirmware,
    SepFirmware,
    NvramTemplate,
    IpswRestoreBundle,
}

/// Cryptographic trust classification of an image artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactTrustStatus {
    VerifiedTrusted,
    ExperimentalOptIn,
    DigestMismatchCorrupt,
    UnverifiedUntrusted,
}

/// Lifecycle status of an owned iOS application deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppDeploymentStatus {
    Imported,
    Installed,
    Running,
    Stopped,
    Removed,
    Incompatible,
}

/// Code signing enforcement policy in guest security profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeSigningMode {
    Enforced,
    AdhocPermitted,
    Disabled,
}

/// Apple Mobile File Integrity (AMFI) status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmfiStatus {
    Enforced,
    DeveloperMode,
    AmfiGetOutOfMyWay,
}

/// Guest sandbox enforcement mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    Enforced,
    AuditOnly,
    Relaxed,
}

/// Filesystem mount mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountMode {
    ReadOnly,
    ReadWrite,
}

/// Kernel privilege level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelPrivilegeLevel {
    Stock,
    RootConsoleEnabled,
    KernelDebugEnabled,
}

/// Standard mutation types requiring Two-Step Safety Gate authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationType {
    DiskWipe,
    GuestWipe,
    InstanceDelete,
    BaselineRestore,
    BaselineDelete,
    SecurityPolicyRelax,
    SecurityApply,
    SecurityRevert,
    ContainerWrite,
    ContainerExport,
    FsWrite,
    FsExport,
    ImagePrepareElevation,
    ImageOverwrite,
    ImagePrepare,
    ProfileApply,
}

/// High-level lifecycle status of a multi-stage operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Created,
    Preflight,
    Staged,
    Executing,
    Verifying,
    Completed,
    Failed,
    CancellationPending,
    Cancelled,
}

// ============================================================================
// Auxiliary Value Objects (contracts/research.schema.json)
// ============================================================================

/// Outcome of an empirical root verification probe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbeOutcome {
    pub path: String,
    pub status: String,
    pub target_user: String,
    pub executed_euid: i64,
    pub expected_denial: bool,
    pub helper_executed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syscall_errno: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syscall_error_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_verified: Option<bool>,
    pub output: String,
}

impl ProbeOutcome {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        match self.status.as_str() {
            "success" | "denied" | "error" | "unverified" => Ok(()),
            other => Err(crate::models::error::ContractViolation::FieldViolation {
                field: "status".to_string(),
                reason: format!(
                    "invalid probe status '{other}': must be success|denied|error|unverified"
                ),
            }),
        }
    }
}

/// Hook status metrics for an active dynamic instrumentation session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookStatus {
    pub native_hooks_count: u64,
    pub objc_hooks_count: u64,
    pub events_intercepted: u64,
}

/// Debug telemetry metrics recorded during kernel debugging trials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugTelemetryRecord {
    pub breakpoints_hit_count: u64,
    pub step_operations_count: u64,
    pub memory_reads_count: u64,
    pub memory_writes_count: u64,
    pub register_reads_count: u64,
    pub disconnect_paused_observed: bool,
}

/// Source origin provenance for an image artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageOriginMetadata {
    pub source_type: String,
    pub build_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url_or_ref: Option<String>,
}

impl ImageOriginMetadata {
    pub fn validate(&self) -> Result<(), crate::models::error::ContractViolation> {
        match self.source_type.as_str() {
            "user_supplied" | "prepared_recovery" => Ok(()),
            other => Err(crate::models::error::ContractViolation::FieldViolation {
                field: "source_type".to_string(),
                reason: format!(
                    "invalid source_type '{other}': must be user_supplied|prepared_recovery"
                ),
            }),
        }
    }
}

/// Host binary prerequisite check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryPrerequisite {
    pub binary_name: String,
    pub found: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
}

/// Entitlement prerequisite check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementPrerequisite {
    pub name: String,
    pub granted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Detailed backend capability status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDetail {
    pub root_shell: SupportStatus,
    pub kernel_debug: SupportStatus,
    pub app_frameworks: SupportStatus,
    pub dynamic_instrumentation: SupportStatus,
    pub companion_bridge: SupportStatus,
}

/// Guest process record conforming to research.schema.json#/definitions/GuestProcessRecord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestProcessRecord {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub user: String,
    pub uid: u32,
    pub arch: CpuArchitecture,
}

/// Guest Mach service record conforming to research.schema.json#/definitions/GuestMachServiceRecord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestMachServiceRecord {
    pub service_name: String,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
}
