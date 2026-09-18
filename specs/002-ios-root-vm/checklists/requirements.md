# Specification Quality Checklist: iOS Root VM and Darwin Security Research Backends

**Purpose**: Requirements quality review for the iOS Root VM and Darwin Security Research Backends feature specification
**Created**: 2026-09-17
**Feature**: [spec.md](../spec.md)

**Note**: This checklist was generated during `/speckit.specify` from the resolved checklist template for requirements quality review.
**Review Ownership**: This checklist is a reviewer-owned requirements-quality review artifact. Mark an item `[x]` only when the reviewer determines the requirements-quality criterion is satisfied.
**Marker Semantics**: `[x]` means the criterion has been reviewed and satisfied for requirements quality. It does not mean implementation work is complete.

## Content Quality

- [x] CHK001 No implementation details (languages, frameworks, APIs)
- [x] CHK002 Focused on user value and business needs
- [x] CHK003 Written for non-technical stakeholders
- [x] CHK004 All mandatory sections completed

## Requirement Completeness

- [x] CHK005 No [NEEDS CLARIFICATION] markers remain
- [x] CHK006 Requirements are testable and unambiguous
- [x] CHK007 Success criteria are measurable
- [x] CHK008 Success criteria are technology-agnostic (no implementation details)
- [x] CHK009 All acceptance scenarios are defined
- [x] CHK010 Edge cases are identified
- [x] CHK011 Scope is clearly bounded
- [x] CHK012 Dependencies and assumptions identified

## Feature Readiness

- [x] CHK013 All functional requirements have clear acceptance criteria
- [x] CHK014 User scenarios cover primary flows
- [x] CHK015 Feature meets measurable outcomes defined in Success Criteria
- [x] CHK016 No implementation details leak into specification

## Notes

### Review Provenance & Iteration History

#### Iteration 1 Baseline Assessment (IosSpecReview & Main Review — 7/16 PASS, 9/16 FAIL)

The initial draft specification was reviewed by `IosSpecReview` and evaluated by Main, recording a baseline verdict of 7/16 passed:

- **Passed Criteria (7/16)**: CHK002, CHK003, CHK004, CHK005, CHK008, CHK010, CHK014.
- **Failed Criteria (9/16)**:
  - `CHK001`: Leaked implementation assumptions (e.g., rigid hypervisor entitlement assumptions, hardcoded volume path matching).
  - `CHK006`: Ambiguous waiting and cancellation semantics; missing explicit falsification and failure criteria for root proof.
  - `CHK007`: Unjustified fixed duration metrics (e.g., hardcoded 10s companion teardown and 60s/180s recovery deadlines without predeclared cohort baselines).
  - `CHK009`: Missing acceptance scenarios for companion retention during active guest dependency, partial cleanup failures, and script runtime faults.
  - `CHK011`: Incompletely bounded companion lifecycle (OR-condition loophole); missing explicit negative controls.
  - `CHK012`: Missing Corellium architectural aspiration references, upstream project source documentation, and guest security profile assumptions.
  - `CHK013`: Missing acceptance mapping for full Frida runtime management (stop/remove) and kernel debug state edit/restoration.
  - `CHK015`: Success criteria lacked backend-named deep research acceptance gates and substituted test-suite passes for observed legacy non-regression.
  - `CHK016`: Storage overlay and transport archive mechanisms leaked into assumptions without backend-agnostic formulation.

#### Iteration 2 Interim User Expansion

Following the user's explicit direction (`"ios thì cũng na ná android, cũng có frida các kiểu nhưng mà hay vì có root bằng kernelsu thì iso ngâm cứu cách mà Corellium cung cấm root vm, tại mình cần truy cập sâu vào APP/thệ thống để research"`), the specification was expanded to cover real iOS applications, complete Frida dynamic instrumentation, and deep system/kernel research capabilities, addressing initial structural gaps.

#### Iteration 3 Final Comprehensive Repair & Main Semantic Review (PASS 16/16)

Main conducted the final semantic review on Iteration 3 with all ten directive groups and five mechanical corrections applied:

1. **Frida Runtime Lifecycle & Target Integrity**: FR-017 covers the complete lifecycle (prepare, install, configure, start, inspect, stop, remove); FR-018 requires actual hook evidence before reporting readiness; script errors report observed target process status (running, terminated, faulted, or unknown) without crashing the VM; app signature validation evaluates against declared `GuestSecurityProfile`; and backend configurations lacking application frameworks report deficiencies truthfully (FR-023, SC-003).
2. **Deep System & Kernel Research**: FR-027 includes controlled test state modification and state restoration across reference configurations for both named backends (SC-004); FR-029 and Story 4 Scenario 3 report actual observed state (paused, running, or unknown) upon debugger disconnect without false resumption claims; system daemon Frida tracing is established on `Inferno` (SC-005); and `GuestSecurityProfile` selection, application, and reversion with guest-only relaxations are formally codified (FR-030).
3. **Companion VM Lifecycle & Communication**: Communication is specified as same-host access-controlled without external control/debug endpoints (FR-039); companion instances terminate ONLY when no active restore workflow remains AND no live dependent guest sessions remain (FR-038, Story 6 Scenario 2); and continuing helper processes are retained during caller wait timeouts or active dependent sessions (Story 6 Scenarios 2 and 4).
4. **Root Proof Rigor & Falsification**: FR-010 through FR-015 tie verification to exact guest and boot session identity; missing, stale, or contradictory evidence—or an unexpected negative control pass—marks root status unverified (Story 2 Scenario 2, FR-014); and containment verifies zero unauthorized host elevation/modification in declared fixtures (SC-007).
5. **Experimental Opt-In & Disposable Cleanup**: Unverified images require explicit opt-in and an available verified baseline (FR-032); cleanup failures report residual mounts and files explicitly, marking images ineligible for ready status (FR-036, Story 5 Scenario 5); and missing baselines safely refuse restoration without automatic deletion (FR-043, Story 7 Scenario 3).
6. **Profiles, Experiment Records & Replication**: Exported profiles automatically exclude host credentials and secrets while guest data requires explicit authorization (FR-041); profile import validates configuration but marks privilege and instrumentation unverified until proven during fresh live execution (Story 7 Scenario 4, SC-013); and immutable `ExperimentRecord` generation is an explicit requirement (FR-042, SC-018).
7. **Waiting Truth, Cancellation & Determinism**: Waiting distinguishes cancellation-pending from confirmed cessation, confirming cessation only at verified safe boundaries (FR-046, Story 8 Scenario 3); observation stop does not cancel guest operations (Story 8 Scenario 4); deterministic desired-state mutations prohibit automatic retries on failed steps (FR-047); and CLI automation requires zero manual TUI steps (FR-048).
8. **Reference Acceptance Set**: A formal Reference Acceptance Set section defines the bounding 4-guest cohort (2 per backend, sequential allowed), frozen host resources, upstream revisions, artifact hashes, and predeclared deadlines before measurement.
9. **Acceptance Coverage Matrix & SC Scoping**: An explicit Acceptance Coverage Matrix maps all FR groups to primary user stories and positive/negative test oracles; all 16 critical negative cases are evaluated across the cohort (SC-019); SC-008 establishes an outcome-based 20-trial legacy lifecycle non-regression evaluation preserving constitution performance budgets; and all eight Android 001 specification artifacts (specification, plan, research, data model, quickstart, two contract files, and requirements checklist) are verified byte-for-byte preserved.
10. **Public Corellium & Upstream Citations**: Assumptions cite Corellium's public architecture ("root is built into their controlled virtual stack, no exploit chain required") as an architectural benchmark and aspiration without commercial API or feature parity claims; upstream links for darwin-vm and Inferno are documented; and persistent image preparation replaces ungrounded storage mechanisms.

#### Final Review Verdict

- **Review Outcome**: Main final semantic review — PASS (16/16 criteria satisfied).
- **Marker Semantics**: All 16 criteria (CHK001–CHK016) marked `[x]` indicating requirements-quality review satisfaction. This artifact documents specification quality only and does not constitute an implementation proof.
- **Final Coverage Counts**:
  - **User Stories**: 8 prioritized user stories (3 P1, 2 P2, 1 P3, 1 P4, 1 P5; 36 total acceptance scenarios).
  - **Functional Requirements**: 48 functional requirements (FR-001 through FR-048).
  - **Success Criteria**: 19 measurable, technology-agnostic outcomes (SC-001 through SC-019).
  - **Edge Cases**: 16 identified edge-case categories.
  - **Key Entities**: 11 core domain entities.
