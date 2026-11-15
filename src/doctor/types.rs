//! Domain types for doctor detections, suggestions, and outcomes.

use crate::adapters::DetectionSource;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FrameworkDetection {
    pub name: String,
    pub detection_source: DetectionSource,
}

/// A suggestion to enable a detected-but-unconfigured capability.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CapabilitySuggestion {
    pub capability: String,
    pub detail: String,
    pub apply_command: String,
}

/// A suggested next step for the current session (advisory-only, never a
/// gate blocker) — e.g. run `ah lint` before it has been run once.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SessionSuggestion {
    pub detail: String,
    pub apply_command: String,
}

/// Outcome of a doctor run: genesis report + domain-specific detections and suggestions.
#[derive(Debug)]
pub struct DoctorOutcome {
    /// The genesis doctor report with all check results.
    pub genesis_report: genesis::doctor::DoctorReport,
    /// Framework detections (domain-specific, not in genesis).
    pub detections: Vec<FrameworkDetection>,
    /// Capability suggestions (domain-specific, not in genesis).
    pub suggestions: Vec<CapabilitySuggestion>,
    /// Session step suggestions (advisory-only; never affect health).
    pub session_suggestions: Vec<SessionSuggestion>,
}

#[derive(Debug)]
pub enum DoctorEnableResult {
    Written { path: String, table_name: String },
    AlreadyEnabled,
}

pub(super) const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(super) const KNOWN_CAPABILITIES: &[&str] = &[
    "pytest", "cargo", "vitest", "mutation", "property", "snapshot",
];
