//! Marker tables consumed by the scenario-flow lint checks.
//!
//! Purpose: hold the term lists that drive the flow heuristics apart from the
//! check logic, so both stay small and reviewable.
//!
//! Responsibilities: expose the qualifier, imperative-mechanic, and negative
//! marker tables to the parent `flow` module.
//!
//! Rationale: the tables are pure data with no behaviour; isolating them keeps
//! `flow` focused on finding construction and under the file-size limit.

/// Qualitative terms that must carry an adjacent numeric bound.
pub(super) const QUALIFIER_TERMS: &[&str] = &[
    "fast",
    "scalable",
    "user-friendly",
    "easy",
    "simple",
    "intuitive",
];

/// UI-mechanic markers that must not appear in WHEN/THEN/AND steps.
pub(super) const IMPERATIVE_MARKERS: &[&str] = &[
    "click",
    "navigates to",
    "fills",
    "button",
    "modal",
    "css",
    "selector",
    "hover",
    "scroll",
];

/// Language marking a scenario as exercising an error, rejection, or
/// boundary violation.
pub(super) const NEGATIVE_MARKERS: &[&str] = &[
    "exits non-zero",
    "reject",
    "error",
    "invalid",
    "fail",
    "denied",
    "violation",
    "exceed",
];
