//! `ah check` orchestration: scope → structural → execution → report.
//!
//! Purpose: wire scope resolution, the structural pass, selection, contract
//! execution, and the quality phase into one `CheckOutput`.
//!
//! Responsibilities: the public entry points (`run_check`,
//! `run_check_with_selection`), the Layer 2 consultation guards, the
//! structural phase ordering, and the final summary/selection assembly.
//!
//! Rationale: this module is the only place that decides *what runs*; every
//! phase it calls is a pure-ish step whose findings it merges, so the
//! conservative-run-all guarantees are readable in one function.

use super::execute::{execute_all, ExecutionState, RunPlan};
use super::ordering::{blocked_scenarios, report_finding_cmp, sorted_resolved_scenarios};
use super::report::{CheckOutput, ReportFinding, ScenarioContext, Scope, Summary};
use super::scope::{resolve_scope, scenario_key, ResolvedScenario, ResolvedScope};
use super::selection::{
    node_contains_filter, prunable_filter, resolve_selection_state, selection_report,
    testaruda_select, testaruda_store_present, SelectionState, TestarudaSelection,
};
use super::stale::{contract_stale_findings, SPK_PROGRAM};
use super::structural::collect_structural_findings;
use crate::config;
use crate::contracts;
use crate::openspec::Scenario;
use crate::quality;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Back-compat entry point (structural callers and tests); CLI uses
/// `run_check_with_selection` directly.
#[allow(dead_code)]
pub fn run_check(
    repo_root: &Path,
    selected_changes: &[String],
    run_tests: bool,
) -> anyhow::Result<CheckOutput> {
    run_check_with_selection(repo_root, selected_changes, run_tests, &[], false)
}

/// Like `run_check`, but with an explicit changed-file set (repo-relative).
/// When `changed_files` is non-empty and `all_tests` is false, only the
/// contract tests of capabilities whose spec or contract files changed run
/// (Layer 1 selection, espectacular-0k8). A changed set that maps to no
/// capability falls back conservatively to running everything.
#[allow(clippy::too_many_arguments)]
pub fn run_check_with_selection(
    repo_root: &Path,
    selected_changes: &[String],
    run_tests: bool,
    changed_files: &[String],
    all_tests: bool,
) -> anyhow::Result<CheckOutput> {
    let cfg = config::load(repo_root)?;
    let specs_dir = repo_root.join(&cfg.paths.specs);
    let contracts_dir = repo_root.join(".espectacular");
    let changes_dir = repo_root.join(&cfg.paths.changes);
    let ah_scope = std::env::var("AH_SCOPE").unwrap_or_default();

    let scope = resolve_scope(&specs_dir, &contracts_dir, &changes_dir, selected_changes)?;
    evaluate_scope(
        repo_root,
        &cfg,
        &specs_dir,
        scope,
        &ah_scope,
        run_tests,
        changed_files,
        all_tests,
    )
}

// Layer 1 selection added changed_files/all_tests; the parameter count is
// deliberate — each arg is a distinct axis of the run, not a hidden bundle.
#[allow(clippy::too_many_arguments)]
fn evaluate_scope(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    mut scope: ResolvedScope,
    ah_scope: &str,
    run_tests: bool,
    changed_files: &[String],
    all_tests: bool,
) -> anyhow::Result<CheckOutput> {
    let findings = structural_phase(specs_root, &mut scope);
    let blocked = blocked_scenarios(&findings);
    let selection_state =
        resolve_selection_state(cfg, &scope.scenarios, changed_files, run_tests, all_tests);
    let testaruda = consult_testaruda(repo_root, changed_files, run_tests, all_tests);
    let l2_selected =
        l2_pruning_selection(&testaruda, &blocked, &selection_state, &scope.scenarios);
    let plan = RunPlan::new(&blocked, &selection_state, l2_selected);
    let mut run = ExecutionState::new(findings);
    if run_tests {
        execute_all(
            repo_root,
            cfg,
            specs_root,
            &scope.scenarios,
            &plan,
            &mut run,
        );
    }
    Ok(assemble_output(
        repo_root,
        cfg,
        ah_scope,
        scope.changes,
        run,
        &selection_state,
        testaruda.is_some(),
    ))
}

/// Structural findings of the whole scope, in report order: resolution
/// findings, then correspondence findings, then derived-contract drift.
fn structural_phase(specs_root: &Path, scope: &mut ResolvedScope) -> Vec<ReportFinding> {
    let mut findings = std::mem::take(&mut scope.findings);
    findings.extend(collect_structural_findings(
        &scope.scenarios,
        &scope.contract_files,
        specs_root,
        &scope.covered,
    ));
    findings.extend(contract_stale_findings(specs_root, scope, SPK_PROGRAM));
    findings
}

/// Layer 2 refinement (espectacular-0k8): when a testaruda store exists and
/// a changed-file set is supplied, consult it to prune contract bindings
/// whose underlying tests are unaffected. `--all-tests` and an empty changed
/// set skip the consultation entirely.
fn consult_testaruda(
    repo_root: &Path,
    changed_files: &[String],
    run_tests: bool,
    all_tests: bool,
) -> Option<TestarudaSelection> {
    if !run_tests || all_tests || changed_files.is_empty() {
        return None;
    }
    if !testaruda_store_present(repo_root) {
        return None;
    }
    testaruda_select(repo_root, changed_files)
}

/// The active Layer 2 selection set, when pruning is both permitted and
/// anchored. Guards, all conservative (never a hidden green): an EMPTY
/// selection (exit 20, no code affected) skips refinement only, unresolved
/// changed files disable pruning, and pruning activates only when at least
/// one binding in scope matches the selected set — an adapter that discovers
/// nothing must not deselect anything.
fn l2_pruning_selection<'a>(
    consulted: &'a Option<TestarudaSelection>,
    blocked: &BTreeSet<(String, String)>,
    selection_state: &SelectionState,
    scenarios: &[ResolvedScenario],
) -> Option<&'a [String]> {
    let selection = consulted.as_ref()?;
    if selection.empty || selection.any_unresolved || selection.selected.is_empty() {
        return None;
    }
    if !l2_anchor_matched(&selection.selected, blocked, selection_state, scenarios) {
        return None;
    }
    Some(selection.selected.as_slice())
}

fn l2_anchor_matched(
    selected: &[String],
    blocked: &BTreeSet<(String, String)>,
    selection_state: &SelectionState,
    scenarios: &[ResolvedScenario],
) -> bool {
    sorted_resolved_scenarios(scenarios)
        .into_iter()
        .filter(|resolved| !blocked.contains(&scenario_key(&resolved.scenario)))
        .filter(|resolved| capability_selected(selection_state, &resolved.scenario))
        .any(|resolved| contract_matches_selection_path(resolved, selected))
}

fn capability_selected(selection_state: &SelectionState, scenario: &Scenario) -> bool {
    match selection_state {
        SelectionState::Caps(caps) => caps.contains(&scenario.spec_path),
        SelectionState::None | SelectionState::All { .. } => true,
    }
}

fn contract_matches_selection_path(resolved: &ResolvedScenario, selected: &[String]) -> bool {
    let Ok(contract) = contracts::load_contract(resolved.contract_path.to_str().unwrap()) else {
        return false;
    };
    contract.tests.iter().any(|(test_type, entries)| {
        entries
            .iter()
            .any(|entry| entry_matches(prunable_filter(test_type, entry), selected))
    })
}

fn entry_matches(filter: Option<&str>, selected: &[String]) -> bool {
    filter.is_some_and(|filter| node_contains_filter(selected, filter))
}

#[allow(clippy::too_many_arguments)]
fn assemble_output(
    repo_root: &Path,
    cfg: &config::Config,
    ah_scope: &str,
    changes: Vec<String>,
    run: ExecutionState,
    selection_state: &SelectionState,
    testaruda_consulted: bool,
) -> CheckOutput {
    let mut findings = run.findings;
    findings.sort_by(report_finding_cmp);
    let structural = count_category(&findings, "structural");
    let execution = count_category(&findings, "execution");
    let counts_by_kind = counts_by_kind(&findings);

    let (quality_findings, tool_findings) =
        quality_phase(repo_root, cfg, ah_scope, run.extra_quality);
    findings.extend(tool_findings);

    CheckOutput {
        scope: Scope {
            deployed: true,
            changes,
        },
        summary: Summary {
            structural,
            execution,
            passed: run.passed,
            counts_by_kind,
        },
        findings,
        quality_findings,
        selection: selection_report(
            selection_state,
            run.skipped,
            run.testaruda_pruned,
            testaruda_consulted,
        ),
    }
}

/// Advisory quality findings plus the gate-failing finding for a tool that
/// could not run at all.
fn quality_phase(
    repo_root: &Path,
    cfg: &config::Config,
    ah_scope: &str,
    extra: Vec<quality::QualityFinding>,
) -> (Vec<quality::QualityFinding>, Vec<ReportFinding>) {
    let (mut quality_findings, tool_errors) =
        quality::collect_quality_findings(repo_root, &cfg.quality, ah_scope);
    quality_findings.extend(extra);
    let tool_findings = tool_errors.into_iter().map(mutation_tool_finding).collect();
    (quality_findings, tool_findings)
}

fn count_category(findings: &[ReportFinding], category: &str) -> usize {
    findings
        .iter()
        .filter(|finding| finding.category == category)
        .count()
}

fn counts_by_kind(findings: &[ReportFinding]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts.entry(finding.kind.clone()).or_insert(0) += 1;
    }
    counts
}

/// Quality-tool invocation failures are gate-failing execution findings
/// attributed to the mutation tool, not to any scenario.
fn mutation_tool_finding(message: String) -> ReportFinding {
    ReportFinding {
        kind: "test-failing".to_string(),
        category: "execution".to_string(),
        spec: "quality".to_string(),
        spec_path: "(quality/mutation)".to_string(),
        scenario: ScenarioContext {
            id: "mutation-tool".to_string(),
            title: "Mutation testing tool".to_string(),
            body_markdown: String::new(),
        },
        suggested_action: "fix_tool_invocation".to_string(),
        playbook_command: "ah explain fix_tool_invocation".to_string(),
        severity: "error".to_string(),
        scenario_prose: None,
        test: None,
        message: Some(message),
    }
}
