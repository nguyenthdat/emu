//! Research validation gate definitions (Gates G-01 through G-08, T-01 through T-08).

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchGate {
    G01HostPreflight,
    G02LifecycleNonRegression,
    G03DisplayNameDisambiguation,
    G04DestructiveAuthorization,
    G05ImageIntegrity,
    G06HostMountSecurity,
    G07CompanionLifecycleBinding,
    G08CancellationTruth,
    T01RootProofVerification,
    T02NegativeControlFalsification,
    T03StateInvalidationOnReboot,
    T04HostContainment,
    T05DynamicHooksCapture,
    T06FrameworkRefusalTruth,
    T07KernelDebugIntegrity,
    T08DebuggerDisconnectTruth,
}

#[allow(dead_code)]
impl ResearchGate {
    pub fn name(&self) -> &'static str {
        match self {
            Self::G01HostPreflight => "G-01: Host Preflight Entitlements",
            Self::G02LifecycleNonRegression => "G-02: Lifecycle 20-Trial Legacy Non-Regression",
            Self::G03DisplayNameDisambiguation => "G-03: Display Name Disambiguation",
            Self::G04DestructiveAuthorization => "G-04: Two-Step Safety Gate Authorization",
            Self::G05ImageIntegrity => "G-05: Firmware Artifact Integrity & Opt-In",
            Self::G06HostMountSecurity => "G-06: Host Mount Identity & Containment",
            Self::G07CompanionLifecycleBinding => "G-07: Companion VM Dependency Binding",
            Self::G08CancellationTruth => "G-08: Cooperative Cancellation Cessation Truth",
            Self::T01RootProofVerification => "T-01: iOS Root Proof UID 0 Verification",
            Self::T02NegativeControlFalsification => "T-02: Negative Control Falsification",
            Self::T03StateInvalidationOnReboot => "T-03: Stale Proof Invalidation on Reboot",
            Self::T04HostContainment => "T-04: Host Security Containment",
            Self::T05DynamicHooksCapture => "T-05: Dynamic Frida Hooks & Specificity",
            Self::T06FrameworkRefusalTruth => "T-06: Minimal Backend Framework Refusal",
            Self::T07KernelDebugIntegrity => "T-07: Kernel Debugging RSP Integrity",
            Self::T08DebuggerDisconnectTruth => "T-08: Debugger Disconnect Truthfulness",
        }
    }
}
