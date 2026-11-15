//! Spec quality linter (`ah lint`).
//!
//! Purpose: statically analyze OpenSpec scenario files for authoring defects
//! (vague qualifiers, imperative steps, conjunctive bloat, missing negative
//! scenarios, missing non-goals, unresolved ambiguity, entangled specs) and
//! emit advisory findings in the shared finding schema.
//!
//! Responsibilities:
//! - define `LintFinding` reusing the shared finding schema fields
//! - run the check registry over the spec walker output
//! - produce a `LintOutput` consumable by the CLI (human + JSON envelope)
//!
//! Rationale: `ah check` verifies spec-test correspondence, but nothing
//! evaluates intrinsic scenario quality at authoring time. Lint findings are
//! advisory (warning severity) in v1 — `ah lint` never gates.
//!
//! Registry note: this module is the registry — espectacular-rty/aar append
//! checks additively to [`checks`]; no check implementations live here.
//!
//! Layout: this root owns the data types and the public paths. The engine
//! (`engine`), the check registry (`registry`), repo-path resolution
//! (`paths`), and the human report (`report`) live in submodules.

pub mod bridge;
pub mod checks;
mod engine;
#[cfg(test)]
mod firing;
mod paths;
mod registry;
mod report;
#[cfg(test)]
mod tests;
pub mod walker;

#[cfg(test)]
pub(crate) use engine::run_lint_with;
pub use engine::{exit_code, merge_outputs, run_change_overlay, run_lint_query};
pub use paths::{changes_dir_for, specs_dir_for};
pub(crate) use registry::{lint_config_for, select_checks};
pub use report::print_report;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use walker::SpecFile;

/// Finding severity. Lint checks emit `Warning` in v1; `Error` is reserved
/// for structural issues such as malformed spec files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Warning,
    Error,
}

/// A lint finding reusing the shared finding schema fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintFinding {
    pub kind: String,
    pub severity: Severity,
    pub spec_path: String,
    pub scenario_id: String,
    pub message: String,
    pub suggestion: String,
    pub suggested_action: String,
    pub playbook_command: String,
}

impl LintFinding {
    /// Build an error-severity finding — structural issues such as malformed
    /// spec files (deployed lint spec C-finding-schema; espectacular-pt8).
    pub fn error(
        kind: &str,
        spec_path: &str,
        scenario_id: &str,
        message: &str,
        suggestion: &str,
        suggested_action: &str,
        playbook_command: &str,
    ) -> Self {
        LintFinding {
            kind: kind.to_string(),
            severity: Severity::Error,
            spec_path: spec_path.to_string(),
            scenario_id: scenario_id.to_string(),
            message: message.to_string(),
            suggestion: suggestion.to_string(),
            suggested_action: suggested_action.to_string(),
            playbook_command: playbook_command.to_string(),
        }
    }

    /// Build a warning-severity finding — what lint checks emit in v1. The
    /// error constructor is reserved for structural issues (malformed spec
    /// files, espectacular-pt8).
    pub fn warning(
        kind: &str,
        spec_path: &str,
        scenario_id: &str,
        message: &str,
        suggestion: &str,
        suggested_action: &str,
        playbook_command: &str,
    ) -> Self {
        LintFinding {
            kind: kind.to_string(),
            severity: Severity::Warning,
            spec_path: spec_path.to_string(),
            scenario_id: scenario_id.to_string(),
            message: message.to_string(),
            suggestion: suggestion.to_string(),
            suggested_action: suggested_action.to_string(),
            playbook_command: playbook_command.to_string(),
        }
    }
}

/// Lint result: findings plus per-kind counts (mirrors `ah check`'s summary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintOutput {
    pub findings: Vec<LintFinding>,
    pub counts_by_kind: BTreeMap<String, usize>,
}

/// A lint check: inspects one walked spec file and appends findings.
///
/// Implementations live in `src/lint/checks/` (espectacular-rty/aar) and are
/// registered in [`checks`]. Advisory-only: emit `Severity::Warning`.
pub trait LintCheck: Send + Sync {
    /// Stable check kind — matches finding `kind` values and `--check` args.
    fn kind(&self) -> &'static str;
    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>);
}
