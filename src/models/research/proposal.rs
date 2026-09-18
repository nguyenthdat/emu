//! Mutation proposal domain data model and two-step safety gate authorization context.
//!
//! Defined in accordance with contracts/research.schema.json, data-model.md §2.14, and FR-008.

use super::types::{BackendType, MutationType, ProposalId, Sha256Digest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

/// All-zero SHA-256 digest string strictly prohibited as a real config revision.
pub const ALL_ZERO_CONFIG_REVISION: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// Strongly-typed errors for proposal validation, digest calculation, and authorization verification.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProposalError {
    #[error("All-zero SHA-256 digest is strictly prohibited as a valid config revision")]
    AllZeroConfigRevision,

    #[error("Invalid SHA-256 digest format: '{0}' (must match ^sha256:[a-f0-9]{{64}}$)")]
    InvalidDigestFormat(String),

    #[error("Proposal ID is invalid or empty: '{0}'")]
    InvalidProposalId(String),

    #[error("Target instance ID is not a valid UUID: '{0}'")]
    InvalidTargetInstanceId(String),

    #[error("Affected paths cannot be empty for a destructive proposal")]
    EmptyAffectedPaths,

    #[error("Destructive field must be true for MutationProposal")]
    NotDestructive,

    #[error("Proposal parameters must be a JSON object")]
    ParametersNotObject,

    #[error("Proposal expires_at '{0}' is not a valid RFC3339 timestamp")]
    InvalidExpiresAt(String),

    #[error("Proposal '{digest}' expired at {expires_at}")]
    Expired { digest: String, expires_at: String },

    #[error("Proposal digest mismatch: declared '{declared}', computed '{computed}'")]
    DigestMismatch { declared: String, computed: String },

    #[error("Proposal operation type mismatch: expected {expected:?}, found {found:?}")]
    OperationTypeMismatch {
        expected: MutationType,
        found: MutationType,
    },

    #[error("Proposal target instance mismatch: expected {expected:?}, found {found:?}")]
    TargetInstanceMismatch {
        expected: Option<String>,
        found: Option<String>,
    },

    #[error("Proposal target resource mismatch: expected {expected:?}, found {found:?}")]
    TargetResourceMismatch {
        expected: Option<String>,
        found: Option<String>,
    },

    #[error("Proposal backend mismatch: expected {expected}, found {found}")]
    BackendMismatch {
        expected: BackendType,
        found: BackendType,
    },

    #[error("Proposal config revision mismatch: expected '{expected}', found '{found}'")]
    ConfigRevisionMismatch { expected: String, found: String },

    #[error("Proposal affected paths mismatch: expected {expected:?}, found {found:?}")]
    AffectedPathsMismatch {
        expected: Vec<String>,
        found: Vec<String>,
    },

    #[error("Proposal parameters mismatch")]
    ParametersMismatch,

    #[error("Proposal not found with digest '{0}'")]
    NotFound(String),
}

/// Request parameters to construct a new MutationProposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposalRequest {
    pub operation_type: MutationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_resource: Option<String>,
    pub backend: BackendType,
    pub config_revision: Sha256Digest,
    pub affected_paths: Vec<String>,
    pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
}

/// Execution authorization context submitted by a caller to consume a proposal.
///
/// Ensures that the mutating command matches the exact target, backend,
/// config revision, paths, parameters, and valid expiry before any destructive effects occur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationContext {
    pub proposal_digest: Sha256Digest,
    pub operation_type: MutationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_resource: Option<String>,
    pub backend: BackendType,
    pub config_revision: Sha256Digest,
    pub affected_paths: Vec<String>,
    pub parameters: serde_json::Value,
}

/// Two-Step Safety Gate mutation proposal pending explicit authorization.
///
/// Defined in accordance with contracts/research.schema.json.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutationProposal {
    pub proposal_id: String,
    pub proposal_digest: Sha256Digest,
    pub config_revision: Sha256Digest,
    pub operation_type: MutationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_resource: Option<String>,
    pub backend: BackendType,
    pub affected_paths: Vec<String>,
    pub destructive: bool,
    pub expires_at: String,
    pub parameters: serde_json::Value,
}

#[derive(Serialize)]
struct CanonicalProposalPayload<'a> {
    proposal_id: &'a str,
    config_revision: &'a str,
    operation_type: &'a str,
    target_instance_id: Option<&'a str>,
    target_resource: Option<&'a str>,
    backend: &'a str,
    affected_paths: &'a [String],
    destructive: bool,
    expires_at: &'a str,
    parameters: &'a serde_json::Value,
}

fn canonicalize_json_value(val: &serde_json::Value) -> serde_json::Value {
    match val {
        serde_json::Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), canonicalize_json_value(v));
            }
            serde_json::to_value(sorted).unwrap_or_else(|_| val.clone())
        }
        serde_json::Value::Array(arr) => {
            let canon: Vec<_> = arr.iter().map(canonicalize_json_value).collect();
            serde_json::Value::Array(canon)
        }
        _ => val.clone(),
    }
}

/// Computes the cryptographic proposal digest over the normalized proposal payload.
///
/// Binds target, backend, configuration revision, affected paths, parameters,
/// and expiration timestamp:
/// `proposal_digest = "sha256:" + hex(SHA-256(CanonicalJSON(ProposalPayload)))`
#[allow(clippy::too_many_arguments)]
pub fn compute_proposal_digest(
    proposal_id: &str,
    config_revision: &Sha256Digest,
    operation_type: MutationType,
    target_instance_id: Option<&str>,
    target_resource: Option<&str>,
    backend: BackendType,
    affected_paths: &[String],
    destructive: bool,
    expires_at: &str,
    parameters: &serde_json::Value,
) -> Result<Sha256Digest, ProposalError> {
    if config_revision.as_str() == ALL_ZERO_CONFIG_REVISION {
        return Err(ProposalError::AllZeroConfigRevision);
    }
    if !is_valid_sha256_digest(config_revision.as_str()) {
        return Err(ProposalError::InvalidDigestFormat(
            config_revision.as_str().to_string(),
        ));
    }

    let mut sorted_paths = affected_paths.to_vec();
    sorted_paths.sort();

    let canon_params = canonicalize_json_value(parameters);

    let op_json = serde_json::to_string(&operation_type)
        .map_err(|e| ProposalError::InvalidDigestFormat(e.to_string()))?;
    let op_str = op_json.trim_matches('"');

    let backend_str = match backend {
        BackendType::DarwinVm => "darwin-vm",
        BackendType::Inferno => "Inferno",
    };

    let payload = CanonicalProposalPayload {
        proposal_id,
        config_revision: config_revision.as_str(),
        operation_type: op_str,
        target_instance_id,
        target_resource,
        backend: backend_str,
        affected_paths: &sorted_paths,
        destructive,
        expires_at,
        parameters: &canon_params,
    };

    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|e| ProposalError::InvalidDigestFormat(e.to_string()))?;

    let mut hasher = Sha256::new();
    hasher.update(&payload_bytes);
    let digest_hex = format!("{:x}", hasher.finalize());

    Ok(Sha256Digest::new(format!("sha256:{digest_hex}")))
}

fn is_valid_sha256_digest(s: &str) -> bool {
    if !s.starts_with("sha256:") || s.len() != 71 {
        return false;
    }
    s[7..].chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

impl MutationProposal {
    /// Creates a new mutation proposal from a structured `ProposalRequest`.
    pub fn create_from_request(
        req: ProposalRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Self, ProposalError> {
        if req.config_revision.as_str() == ALL_ZERO_CONFIG_REVISION {
            return Err(ProposalError::AllZeroConfigRevision);
        }
        if !is_valid_sha256_digest(req.config_revision.as_str()) {
            return Err(ProposalError::InvalidDigestFormat(
                req.config_revision.as_str().to_string(),
            ));
        }
        if req.affected_paths.is_empty() {
            return Err(ProposalError::EmptyAffectedPaths);
        }
        if !req.parameters.is_object() {
            return Err(ProposalError::ParametersNotObject);
        }
        if let Some(inst_id) = &req.target_instance_id {
            if uuid::Uuid::parse_str(inst_id).is_err() {
                return Err(ProposalError::InvalidTargetInstanceId(inst_id.clone()));
            }
        }

        let proposal_id = ProposalId::new().to_string();
        let ttl_secs = req.ttl_seconds.unwrap_or(900); // 15 minutes default
        let expires_at = (now + chrono::Duration::seconds(ttl_secs as i64)).to_rfc3339();

        let mut sorted_paths = req.affected_paths.clone();
        sorted_paths.sort();

        let proposal_digest = compute_proposal_digest(
            &proposal_id,
            &req.config_revision,
            req.operation_type,
            req.target_instance_id.as_deref(),
            req.target_resource.as_deref(),
            req.backend,
            &sorted_paths,
            true,
            &expires_at,
            &req.parameters,
        )?;

        let proposal = Self {
            proposal_id,
            proposal_digest,
            config_revision: req.config_revision,
            operation_type: req.operation_type,
            target_instance_id: req.target_instance_id,
            target_resource: req.target_resource,
            backend: req.backend,
            affected_paths: sorted_paths,
            destructive: true,
            expires_at,
            parameters: canonicalize_json_value(&req.parameters),
        };

        proposal.validate()?;
        Ok(proposal)
    }

    /// Validates internal schema invariants of the MutationProposal.
    pub fn validate(&self) -> Result<(), ProposalError> {
        if !self.destructive {
            return Err(ProposalError::NotDestructive);
        }
        if self.config_revision.as_str() == ALL_ZERO_CONFIG_REVISION {
            return Err(ProposalError::AllZeroConfigRevision);
        }
        if !is_valid_sha256_digest(self.config_revision.as_str()) {
            return Err(ProposalError::InvalidDigestFormat(
                self.config_revision.as_str().to_string(),
            ));
        }
        if !is_valid_sha256_digest(self.proposal_digest.as_str()) {
            return Err(ProposalError::InvalidDigestFormat(
                self.proposal_digest.as_str().to_string(),
            ));
        }
        if uuid::Uuid::parse_str(&self.proposal_id).is_err() {
            return Err(ProposalError::InvalidProposalId(self.proposal_id.clone()));
        }
        if let Some(inst_id) = &self.target_instance_id {
            if uuid::Uuid::parse_str(inst_id).is_err() {
                return Err(ProposalError::InvalidTargetInstanceId(inst_id.clone()));
            }
        }
        if self.affected_paths.is_empty() {
            return Err(ProposalError::EmptyAffectedPaths);
        }
        if chrono::DateTime::parse_from_rfc3339(&self.expires_at).is_err() {
            return Err(ProposalError::InvalidExpiresAt(self.expires_at.clone()));
        }
        if !self.parameters.is_object() {
            return Err(ProposalError::ParametersNotObject);
        }

        // Recompute digest to ensure binding integrity
        let computed = compute_proposal_digest(
            &self.proposal_id,
            &self.config_revision,
            self.operation_type,
            self.target_instance_id.as_deref(),
            self.target_resource.as_deref(),
            self.backend,
            &self.affected_paths,
            self.destructive,
            &self.expires_at,
            &self.parameters,
        )?;

        if self.proposal_digest != computed {
            return Err(ProposalError::DigestMismatch {
                declared: self.proposal_digest.as_str().to_string(),
                computed: computed.as_str().to_string(),
            });
        }

        Ok(())
    }

    /// Verifies that this proposal matches the incoming `AuthorizationContext` exactly
    /// and has not expired.
    pub fn verify_authorization(
        &self,
        ctx: &AuthorizationContext,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ProposalError> {
        self.validate()?;

        if self.proposal_digest != ctx.proposal_digest {
            return Err(ProposalError::DigestMismatch {
                declared: self.proposal_digest.as_str().to_string(),
                computed: ctx.proposal_digest.as_str().to_string(),
            });
        }

        if self.operation_type != ctx.operation_type {
            return Err(ProposalError::OperationTypeMismatch {
                expected: ctx.operation_type,
                found: self.operation_type,
            });
        }

        if self.target_instance_id != ctx.target_instance_id {
            return Err(ProposalError::TargetInstanceMismatch {
                expected: ctx.target_instance_id.clone(),
                found: self.target_instance_id.clone(),
            });
        }

        if self.target_resource != ctx.target_resource {
            return Err(ProposalError::TargetResourceMismatch {
                expected: ctx.target_resource.clone(),
                found: self.target_resource.clone(),
            });
        }

        if self.backend != ctx.backend {
            return Err(ProposalError::BackendMismatch {
                expected: ctx.backend,
                found: self.backend,
            });
        }

        if self.config_revision != ctx.config_revision {
            return Err(ProposalError::ConfigRevisionMismatch {
                expected: ctx.config_revision.as_str().to_string(),
                found: self.config_revision.as_str().to_string(),
            });
        }

        let mut self_paths = self.affected_paths.clone();
        self_paths.sort();
        let mut ctx_paths = ctx.affected_paths.clone();
        ctx_paths.sort();
        if self_paths != ctx_paths {
            return Err(ProposalError::AffectedPathsMismatch {
                expected: ctx.affected_paths.clone(),
                found: self.affected_paths.clone(),
            });
        }

        let self_canon_params = canonicalize_json_value(&self.parameters);
        let ctx_canon_params = canonicalize_json_value(&ctx.parameters);
        if self_canon_params != ctx_canon_params {
            return Err(ProposalError::ParametersMismatch);
        }

        let expiry = chrono::DateTime::parse_from_rfc3339(&self.expires_at)
            .map_err(|_| ProposalError::InvalidExpiresAt(self.expires_at.clone()))?
            .with_timezone(&chrono::Utc);

        if now > expiry {
            return Err(ProposalError::Expired {
                digest: self.proposal_digest.as_str().to_string(),
                expires_at: self.expires_at.clone(),
            });
        }

        Ok(())
    }
}
