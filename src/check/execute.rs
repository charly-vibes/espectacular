//! Contract execution: run bound tests and record verdicts.
//!
//! Purpose: execute the in-scope contract bindings of the selected
//! capabilities and turn each outcome into a finding or a pass.
//!
//! Responsibilities: the per-scenario binding walk (custom runners, quality
//! types, batched vitest patterns, generic bindings), exit-code and
//! matched-zero verdicts, Layer 1 deselection and Layer 2 pruning, and the
//! batched invocation with its per-binding fallback.
//!
//! Rationale: every binding type funnels into the same pass/fail accounting
//! so a verdict can never be counted twice or silently dropped; the batched
//! path degrades to per-binding runs rather than to a silent green.

use super::batch::run_batched;
use super::ordering::sorted_resolved_scenarios;
use super::report::{
    execution_report, execution_report_message, no_tests_ran_report, structural_report,
    ReportFinding,
};
use super::scope::{scenario_key, ResolvedScenario};
use super::selection::{node_contains_filter, prunable_filter, SelectionState};
use crate::adapters;
use crate::batching;
use crate::config;
use crate::contracts::{self, TestEntry};
use crate::openspec::Scenario;
use crate::quality;
use crate::runner::{self, TestResult};
use std::collections::BTreeSet;
use std::path::Path;

/// Accumulated results of the execution phase.
pub(crate) struct ExecutionState {
    pub(crate) findings: Vec<ReportFinding>,
    pub(crate) passed: usize,
    /// Scenarios deselected by Layer 1 selection.
    pub(crate) skipped: usize,
    /// Bindings pruned by Layer 2 (testaruda) selection.
    pub(crate) testaruda_pruned: usize,
    pub(crate) extra_quality: Vec<quality::QualityFinding>,
    pub(crate) batchable: Vec<(usize, batching::BatchableBinding)>,
}

impl ExecutionState {
    pub(crate) fn new(findings: Vec<ReportFinding>) -> Self {
        ExecutionState {
            findings,
            passed: 0,
            skipped: 0,
            testaruda_pruned: 0,
            extra_quality: Vec::new(),
            batchable: Vec::new(),
        }
    }
}

/// How the current run narrows its bindings: structurally blocked scenarios,
/// the Layer 1 capability selection, and — when active — Layer 2 pruning.
pub(crate) struct RunPlan<'a> {
    blocked: &'a BTreeSet<(String, String)>,
    selection: &'a SelectionState,
    l2_selected: Option<&'a [String]>,
}

impl<'a> RunPlan<'a> {
    pub(crate) fn new(
        blocked: &'a BTreeSet<(String, String)>,
        selection: &'a SelectionState,
        l2_selected: Option<&'a [String]>,
    ) -> Self {
        RunPlan {
            blocked,
            selection,
            l2_selected,
        }
    }

    /// Structurally broken scenarios run nothing (their findings already
    /// explain why).
    fn blocked(&self, scenario: &Scenario) -> bool {
        self.blocked.contains(&scenario_key(scenario))
    }

    /// Layer 1 deselection: the changed-file set did not select this
    /// capability.
    fn skipped(&self, scenario: &Scenario) -> bool {
        match self.selection {
            SelectionState::Caps(caps) => !caps.contains(&scenario.spec_path),
            SelectionState::None | SelectionState::All { .. } => false,
        }
    }

    /// Layer 2 pruning: drop a binding whose runner filter matches nothing in
    /// testaruda's selected set.
    fn prunes(&self, test_type: &str, entry: &TestEntry) -> bool {
        let Some(selected) = self.l2_selected else {
            return false;
        };
        prunable_filter(test_type, entry)
            .is_some_and(|filter| !node_contains_filter(selected, filter))
    }
}

/// Execute every in-scope scenario binding, then any batched invocation.
pub(crate) fn execute_all(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scenarios: &[ResolvedScenario],
    plan: &RunPlan<'_>,
    run: &mut ExecutionState,
) {
    let sorted = sorted_resolved_scenarios(scenarios);
    for (scenario_index, resolved) in sorted.iter().enumerate() {
        run_scenario(
            repo_root,
            cfg,
            specs_root,
            scenario_index,
            resolved,
            plan,
            run,
        );
    }
    run_batched(repo_root, cfg, specs_root, &sorted, run);
}

#[allow(clippy::too_many_arguments)]
fn run_scenario(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scenario_index: usize,
    resolved: &ResolvedScenario,
    plan: &RunPlan<'_>,
    run: &mut ExecutionState,
) {
    let scenario = &resolved.scenario;
    if plan.blocked(scenario) {
        return;
    }
    if plan.skipped(scenario) {
        run.skipped += 1;
        return;
    }
    let Ok(contract) = contracts::load_contract(resolved.contract_path.to_str().unwrap()) else {
        return;
    };
    let mut test_types: Vec<_> = contract.tests.keys().cloned().collect();
    test_types.sort();
    for test_type in test_types {
        for entry in &contract.tests[&test_type] {
            run_binding(
                repo_root,
                cfg,
                specs_root,
                scenario_index,
                scenario,
                &test_type,
                entry,
                plan,
                run,
            );
        }
    }
}

/// Dispatch one binding: prune it, batch it, or execute it by test type.
#[allow(clippy::too_many_arguments)]
fn run_binding(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scenario_index: usize,
    scenario: &Scenario,
    test_type: &str,
    entry: &TestEntry,
    plan: &RunPlan<'_>,
    run: &mut ExecutionState,
) {
    if plan.prunes(test_type, entry) {
        run.testaruda_pruned += 1;
        return;
    }
    if test_type == "vitest" {
        if let Some(pattern) = batching::batchable_pattern(entry) {
            run.batchable.push((
                scenario_index,
                batching::BatchableBinding {
                    pattern,
                    entry: entry.clone(),
                },
            ));
            return;
        }
    }
    match test_type {
        "custom" => run_custom_binding(repo_root, cfg, specs_root, scenario, entry, run),
        "property" | "snapshot" => {
            run_quality_binding(repo_root, cfg, specs_root, scenario, test_type, entry, run)
        }
        _ => run_generic_binding(repo_root, cfg, specs_root, scenario, test_type, entry, run),
    }
}

fn run_custom_binding(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scenario: &Scenario,
    entry: &TestEntry,
    run: &mut ExecutionState,
) {
    match adapters::custom::invoke(repo_root, cfg, entry) {
        Ok(adapters::custom::CustomRunnerResult::Passed) => run.passed += 1,
        Ok(adapters::custom::CustomRunnerResult::TestFailing(result)) => {
            let finding = execution_report(scenario, specs_root, result);
            run.findings.push(finding);
        }
        Ok(adapters::custom::CustomRunnerResult::EnvelopeFindings(raw)) => {
            let findings = envelope_findings(scenario, specs_root, raw);
            run.findings.extend(findings);
        }
        Err(error) => {
            let finding = structural_report(
                scenario,
                specs_root,
                "missing-runner",
                Some(error.to_string()),
            );
            run.findings.push(finding);
        }
    }
}

/// Custom-runner envelope findings enter the report verbatim; an envelope
/// entry that does not match the schema becomes a schema-mismatch finding
/// instead of vanishing.
fn envelope_findings(
    scenario: &Scenario,
    specs_root: &Path,
    raw: Vec<serde_json::Value>,
) -> Vec<ReportFinding> {
    raw.into_iter()
        .map(
            |value| match serde_json::from_value::<ReportFinding>(value) {
                Ok(finding) => finding,
                Err(_) => execution_report_message(
                    scenario,
                    specs_root,
                    "custom runner emitted a finding that does not match the finding schema",
                ),
            },
        )
        .collect()
}

/// property/snapshot bindings are quality signals: a pass produces an
/// advisory quality finding, a failure a regular execution finding.
fn run_quality_binding(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scenario: &Scenario,
    test_type: &str,
    entry: &TestEntry,
    run: &mut ExecutionState,
) {
    let result = match adapters::invoke(repo_root, cfg, test_type, entry) {
        Ok(result) => result,
        Err(error) => {
            let finding = structural_report(
                scenario,
                specs_root,
                "missing-runner",
                Some(error.to_string()),
            );
            run.findings.push(finding);
            return;
        }
    };
    if result.timed_out || result.exit_code != Some(0) {
        let finding = execution_report(scenario, specs_root, result);
        run.findings.push(finding);
    } else {
        run.extra_quality
            .push(quality::QualityFinding::for_test_type(test_type));
    }
}

fn run_generic_binding(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scenario: &Scenario,
    test_type: &str,
    entry: &TestEntry,
    run: &mut ExecutionState,
) {
    let result = match adapters::invoke(repo_root, cfg, test_type, entry) {
        Ok(result) => result,
        Err(error) => {
            let finding = structural_report(
                scenario,
                specs_root,
                "missing-runner",
                Some(error.to_string()),
            );
            run.findings.push(finding);
            return;
        }
    };
    apply_execution_verdict(scenario, specs_root, result, run);
}

/// Exit-code verdict path shared by per-binding execution in the run loop
/// and per-binding fallback re-runs after a degenerate batched invocation
/// (batch-runner-spawns C-batch-fallback).
fn apply_execution_verdict(
    scenario: &Scenario,
    specs_root: &Path,
    result: TestResult,
    run: &mut ExecutionState,
) {
    if result.timed_out || result.exit_code != Some(0) {
        let finding = execution_report(scenario, specs_root, result);
        run.findings.push(finding);
    } else if runner::matched_zero_tests(&result) {
        let finding = no_tests_ran_report(scenario, specs_root, result);
        run.findings.push(finding);
    } else {
        run.passed += 1;
    }
}

/// Re-run one binding per-contract after batched attribution was impossible
/// (batch-runner-spawns C-batch-fallback): exit-code verdicts only.
pub(crate) fn run_binding_fallback(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scenario: &Scenario,
    binding: &batching::BatchableBinding,
    run: &mut ExecutionState,
) {
    let result = match adapters::invoke(repo_root, cfg, "vitest", &binding.entry) {
        Ok(result) => result,
        Err(error) => {
            let finding = structural_report(
                scenario,
                specs_root,
                "missing-runner",
                Some(error.to_string()),
            );
            run.findings.push(finding);
            return;
        }
    };
    apply_execution_verdict(scenario, specs_root, result, run);
}

#[cfg(test)]
pub(crate) mod tests;
