//! Persistence, operation journal, and proposal constants for research workflows.

use std::time::Duration;

/// Default validity window for mutation proposals pending authorization (15 minutes).
pub const DEFAULT_PROPOSAL_TTL: Duration = Duration::from_secs(900);

/// Default number of events returned in a paginated event query.
pub const DEFAULT_EVENT_PAGE_LIMIT: usize = 50;

/// Hard ceiling for maximum events returned in a single query.
pub const MAX_EVENT_PAGE_LIMIT: usize = 1000;

/// Hard byte ceiling for a single paginated events response (1 MiB).
pub const MAX_EVENT_PAGE_BYTES: usize = 1024 * 1024;

/// All-zero SHA-256 digest string strictly prohibited as a real config revision.
pub const ALL_ZERO_SHA256_DIGEST: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// Initial phase name for newly dispatched operations.
pub const PHASE_PREFLIGHT: &str = "preflight";

/// Staging phase name.
pub const PHASE_STAGED: &str = "staged";

/// Execution phase name.
pub const PHASE_EXECUTING: &str = "executing";

/// Verification phase name.
pub const PHASE_VERIFYING: &str = "verifying";

/// Teardown phase name.
pub const PHASE_TEARDOWN: &str = "teardown";

/// Settled completed phase name.
pub const PHASE_COMPLETED: &str = "completed";

/// Maximum retry count for reconciler operations (strictly 0 per FR-047).
pub const MAX_MUTATION_RETRIES: usize = 0;
