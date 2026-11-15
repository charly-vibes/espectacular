//! Finding and report shapes shared by every check phase.
//!
//! Purpose: own the `ah check` output schema (`CheckOutput`, `Scope`,
//! `Summary`, `ReportFinding`, `ScenarioContext`, `Finding`) and the single
//! constructor every phase emits through.
//!
//! Responsibilities: finding constructors per category, the severity /
//! category policy (only non-gating kinds are warnings), the suggested-action
//! table, and the specs-root markdown path helper.
//!
//! Rationale: one constructor keeps `suggested_action`, `playbook_command`,
//! and `severity` consistent across structural, execution, overlay, and
//! custom-runner findings — and keeps the serialized schema stable.

use super::selection::Selection;
use crate::openspec::Scenario;
use crate::quality;
use crate::runner::TestResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub spec_path: String,
    pub scenario_id: String,
    pub kind: String,
    pub category: String,
    pub source_location: Option<String>,
}

impl Finding {
    #[allow(dead_code)]
    pub(crate) fn structural(spec_path: &str, scenario_id: &str, kind: &str) -> Self {
        Finding {
            spec_path: spec_path.to_string(),
            scenario_id: scenario_id.to_string(),
            kind: kind.to_string(),
            category: "structural".to_string(),
            source_location: None,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CheckOutput {
    pub scope: Scope,
    pub summary: Summary,
    pub findings: Vec<ReportFinding>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub quality_findings: Vec<quality::QualityFinding>,
    /// Test-selection report (espectacular-0k8 Layer 1): present only when
    /// a changed-file set narrowed the executed contract tests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<Selection>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Scope {
    pub deployed: bool,
    pub changes: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub structural: usize,
    pub execution: usize,
    pub passed: usize,
    pub counts_by_kind: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ReportFinding {
    pub kind: String,
    pub category: String,
    pub spec: String,
    pub spec_path: String,
    pub scenario: ScenarioContext,
    pub suggested_action: String,
    pub playbook_command: String,
    /// Gate severity: "error" for structural/execution findings (gate-failing),
    /// "warning" for non-gating findings (exit zero on warnings only).
    /// External custom-runner findings that omit it default to "error".
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_prose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<TestResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn default_severity() -> String {
    "error".to_string()
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ScenarioContext {
    pub id: String,
    pub title: String,
    pub body_markdown: String,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn report_finding(
    kind: &str,
    category: &str,
    spec: String,
    spec_path: String,
    scenario: ScenarioContext,
    scenario_prose: Option<String>,
    test: Option<TestResult>,
    message: Option<String>,
) -> ReportFinding {
    let suggested_action = suggested_action_for(kind).to_string();
    let severity = if kind == "missing-liveness-timeout" || kind == "batch-fallback" {
        "warning"
    } else {
        "error"
    };
    let category = if severity == "warning" {
        "warning"
    } else {
        category
    };
    ReportFinding {
        kind: kind.to_string(),
        category: category.to_string(),
        spec,
        spec_path,
        scenario,
        suggested_action: suggested_action.clone(),
        playbook_command: format!("ah explain {suggested_action}"),
        severity: severity.to_string(),
        scenario_prose,
        test,
        message,
    }
}

pub(crate) fn structural_report(
    scenario: &Scenario,
    specs_root: &Path,
    kind: &str,
    message: Option<String>,
) -> ReportFinding {
    report_finding(
        kind,
        "structural",
        scenario.spec_path.clone(),
        spec_markdown_path(specs_root, &scenario.spec_path),
        scenario_context(scenario),
        Some(scenario.body.clone()),
        None,
        message,
    )
}

pub(crate) fn execution_report(
    scenario: &Scenario,
    specs_root: &Path,
    test: TestResult,
) -> ReportFinding {
    report_finding(
        "test-failing",
        "execution",
        scenario.spec_path.clone(),
        spec_markdown_path(specs_root, &scenario.spec_path),
        scenario_context(scenario),
        Some(scenario.body.clone()),
        Some(test),
        None,
    )
}

pub(crate) fn execution_report_message(
    scenario: &Scenario,
    specs_root: &Path,
    message: &str,
) -> ReportFinding {
    report_finding(
        "test-failing",
        "execution",
        scenario.spec_path.clone(),
        spec_markdown_path(specs_root, &scenario.spec_path),
        scenario_context(scenario),
        Some(scenario.body.clone()),
        None,
        Some(message.to_string()),
    )
}

pub(crate) fn no_tests_ran_report(
    scenario: &Scenario,
    specs_root: &Path,
    test: TestResult,
) -> ReportFinding {
    report_finding(
        "no-tests-ran",
        "execution",
        scenario.spec_path.clone(),
        spec_markdown_path(specs_root, &scenario.spec_path),
        scenario_context(scenario),
        Some(scenario.body.clone()),
        Some(test),
        None,
    )
}

/// Named, non-gating degradation signal for a batched invocation that fell
/// back to per-binding execution (batch-runner-spawns task 2.2,
/// C-batch-fallback): severity warning keeps the gate green when the
/// per-binding re-runs pass, while the reason makes the fallback auditable.
pub(crate) fn batch_fallback_report(
    scenario: &Scenario,
    specs_root: &Path,
    reason: &str,
) -> ReportFinding {
    report_finding(
        "batch-fallback",
        "warning",
        scenario.spec_path.clone(),
        spec_markdown_path(specs_root, &scenario.spec_path),
        scenario_context(scenario),
        Some(scenario.body.clone()),
        None,
        Some(format!(
            "batched runner invocation degraded; binding re-ran per-contract ({reason})"
        )),
    )
}

/// The scenario context every scenario-scoped finding carries.
pub(crate) fn scenario_context(scenario: &Scenario) -> ScenarioContext {
    ScenarioContext {
        id: scenario.id.clone(),
        title: scenario.heading.clone(),
        body_markdown: scenario.body.clone(),
    }
}

pub(crate) fn suggested_action_for(kind: &str) -> &'static str {
    match kind {
        "no-toml"
        | "orphan-toml"
        | "slug-collision"
        | "id-mismatch"
        | "invalid-status"
        | "invalid-falsifiability-class"
        | "missing-liveness-timeout"
        | "no-tests-declared"
        | "malformed-contract"
        | "missing-replacement"
        | "overlay-conflict" => "review_and_apply",
        "missing-runner" | "test-failing" | "no-tests-ran" => "edit_code_not_scenario",
        "contract-stale" => "run_ah_sync",
        _ => "human_review_required",
    }
}

pub(crate) fn spec_markdown_path(specs_root: &Path, spec: &str) -> String {
    specs_root
        .join(spec)
        .join("spec.md")
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
pub(crate) mod tests;
