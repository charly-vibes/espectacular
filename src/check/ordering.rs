//! Deterministic ordering and blocking helpers over check findings.
//!
//! Purpose: keep finding order stable across runs (gate output is diffed by
//! contract tests) and derive the blocked-scenario set from structural
//! findings.
//!
//! Responsibilities: scenario sort order, the finding comparator, and the
//! (spec, scenario) block set that suppresses execution for structurally
//! broken scenarios.
//!
//! Rationale: sorting and blocking are cross-phase concerns; owning them in
//! one place keeps the structural and execution phases from inventing
//! competing orders.

use super::report::ReportFinding;
use super::scope::ResolvedScenario;
use std::collections::BTreeSet;

pub(crate) fn sorted_resolved_scenarios(scenarios: &[ResolvedScenario]) -> Vec<&ResolvedScenario> {
    let mut sorted: Vec<_> = scenarios.iter().collect();
    sorted.sort_by(|left, right| {
        (&left.scenario.spec_path, &left.scenario.id)
            .cmp(&(&right.scenario.spec_path, &right.scenario.id))
    });
    sorted
}

pub(crate) fn report_finding_cmp(
    left: &ReportFinding,
    right: &ReportFinding,
) -> std::cmp::Ordering {
    (
        &left.spec_path,
        &left.scenario.id,
        &left.kind,
        left.test
            .as_ref()
            .map(|test| (&test.test_type, &test.command)),
    )
        .cmp(&(
            &right.spec_path,
            &right.scenario.id,
            &right.kind,
            right
                .test
                .as_ref()
                .map(|test| (&test.test_type, &test.command)),
        ))
}

pub(crate) fn blocked_scenarios(findings: &[ReportFinding]) -> BTreeSet<(String, String)> {
    findings
        .iter()
        .filter(|finding| finding.category == "structural")
        .map(|finding| (finding.spec.clone(), finding.scenario.id.clone()))
        .collect()
}
