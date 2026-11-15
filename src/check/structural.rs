//! Structural correspondence checks: does every scenario own a well-formed
//! contract, and does every contract belong to a scenario?
//!
//! Purpose: run the id/schema/test-declaration/supersede/liveness rules over
//! each resolved contract and report orphan contracts, slug collisions, and
//! unparseable contracts.
//!
//! Responsibilities: the per-contract rule set (one function per rule, so
//! each rule's condition reads on its own), orphan detection with the
//! VERIFIES exemption, and the library-shaped `structural_findings` entry.
//!
//! Rationale: every finding this module emits blocks execution for its
//! scenario, so each rule is a single named predicate — a rule that fires
//! spuriously turns a green gate red, and a rule silently dropped hides a
//! missing contract.

use super::contract_files::collect_base_contract_files;
use super::ordering::{report_finding_cmp, sorted_resolved_scenarios};
use super::report::{
    report_finding, spec_markdown_path, structural_report, Finding, ReportFinding, ScenarioContext,
};
use super::scope::{scenario_key, ResolvedScenario};
use super::verifies::{contract_path, covered_contract_keys};
use crate::contracts;
use crate::openspec::{self, Scenario};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Library-shaped entry point over `collect_structural_findings`; used by
/// contract tests (the binary goes through `run_check` → the same collector).
#[allow(dead_code)]
pub fn structural_findings(specs_dir: &str, contracts_dir: &str) -> anyhow::Result<Vec<Finding>> {
    let scenarios = openspec::discover_scenarios(specs_dir)?;
    let resolved: Vec<_> = scenarios
        .into_iter()
        .map(|scenario| ResolvedScenario {
            contract_path: contract_path(
                Path::new(contracts_dir),
                &scenario.spec_path,
                &scenario.id,
            ),
            scenario,
        })
        .collect();
    let contract_files = collect_base_contract_files(Path::new(contracts_dir));
    Ok(collect_structural_findings(
        &resolved,
        &contract_files,
        Path::new(specs_dir),
        &BTreeMap::new(),
    )
    .into_iter()
    .map(|finding| Finding::structural(&finding.spec, &finding.scenario.id, &finding.kind))
    .collect())
}

pub(crate) fn collect_structural_findings(
    scenarios: &[ResolvedScenario],
    contract_files: &[(String, String, PathBuf)],
    specs_root: &Path,
    covered: &BTreeMap<(String, String), Vec<String>>,
) -> Vec<ReportFinding> {
    let scenario_map = scenario_map(scenarios);
    let mut findings = collision_findings(scenarios, &scenario_map, specs_root);
    let collision_ids = collision_keys(&findings);
    let known_ids = scenario_keys(scenarios);
    let mut seen = HashSet::new();
    for resolved in sorted_resolved_scenarios(scenarios) {
        let key = scenario_key(&resolved.scenario);
        if collision_ids.contains(&key) || !seen.insert(key) {
            continue;
        }
        findings.extend(contract_findings(resolved, &known_ids, covered, specs_root));
    }
    findings.extend(orphan_reports(
        contract_files,
        &scenario_map,
        specs_root,
        covered,
    ));
    findings.sort_by(report_finding_cmp);
    findings
}

fn scenario_map(scenarios: &[ResolvedScenario]) -> BTreeMap<(String, String), &ResolvedScenario> {
    scenarios
        .iter()
        .map(|resolved| (scenario_key(&resolved.scenario), resolved))
        .collect()
}

/// Slug collisions are detected on the raw discovery list, then reported for
/// every scenario that carries the colliding (spec, id).
fn collision_findings(
    scenarios: &[ResolvedScenario],
    scenario_map: &BTreeMap<(String, String), &ResolvedScenario>,
    specs_root: &Path,
) -> Vec<ReportFinding> {
    let bare_scenarios: Vec<_> = scenarios
        .iter()
        .map(|resolved| resolved.scenario.clone())
        .collect();
    openspec::detect_slug_collisions(&bare_scenarios)
        .into_iter()
        .filter_map(|(spec, id, _)| {
            scenario_map.get(&(spec, id)).map(|resolved| {
                structural_report(&resolved.scenario, specs_root, "slug-collision", None)
            })
        })
        .collect()
}

fn collision_keys(findings: &[ReportFinding]) -> HashSet<(String, String)> {
    findings
        .iter()
        .filter(|finding| finding.kind == "slug-collision")
        .map(|finding| (finding.spec.clone(), finding.scenario.id.clone()))
        .collect()
}

fn scenario_keys(scenarios: &[ResolvedScenario]) -> HashSet<(String, String)> {
    scenarios
        .iter()
        .map(|resolved| scenario_key(&resolved.scenario))
        .collect()
}

/// All structural findings for one scenario's contract: a missing or
/// unparseable contract replaces the rule set, an existing one is checked.
fn contract_findings(
    resolved: &ResolvedScenario,
    known_ids: &HashSet<(String, String)>,
    covered: &BTreeMap<(String, String), Vec<String>>,
    specs_root: &Path,
) -> Vec<ReportFinding> {
    let scenario = &resolved.scenario;
    if !resolved.contract_path.exists() {
        return vec![structural_report(scenario, specs_root, "no-toml", None)];
    }
    match contracts::load_contract(resolved.contract_path.to_str().unwrap()) {
        Ok(contract) => contract_rule_findings(scenario, &contract, known_ids, covered, specs_root),
        Err(error) => vec![structural_report(
            scenario,
            specs_root,
            "malformed-contract",
            Some(error.to_string()),
        )],
    }
}

/// The contract rule set, each rule appending only when its condition holds.
fn contract_rule_findings(
    scenario: &Scenario,
    contract: &contracts::Contract,
    known_ids: &HashSet<(String, String)>,
    covered: &BTreeMap<(String, String), Vec<String>>,
    specs_root: &Path,
) -> Vec<ReportFinding> {
    let mut findings = id_mismatch_findings(scenario, contract, covered, specs_root);
    findings.extend(invalid_class_findings(scenario, contract, specs_root));
    findings.extend(no_tests_findings(scenario, contract, specs_root));
    findings.extend(replacement_findings(
        scenario, contract, known_ids, specs_root,
    ));
    findings.extend(liveness_timeout_findings(scenario, contract, specs_root));
    findings
}

/// A VERIFIES-covered scenario resolves to its property's derived contract,
/// whose id is the slugified property id — not the scenario id — so the
/// mismatch check does not apply.
fn id_mismatch_findings(
    scenario: &Scenario,
    contract: &contracts::Contract,
    covered: &BTreeMap<(String, String), Vec<String>>,
    specs_root: &Path,
) -> Vec<ReportFinding> {
    if covered.contains_key(&scenario_key(scenario)) || contract.id == scenario.id {
        return Vec::new();
    }
    vec![structural_report(scenario, specs_root, "id-mismatch", None)]
}

fn invalid_class_findings(
    scenario: &Scenario,
    contract: &contracts::Contract,
    specs_root: &Path,
) -> Vec<ReportFinding> {
    if contract.falsifiability_class.is_empty()
        || contracts::FALSIFIABILITY_CLASSES.contains(&contract.falsifiability_class.as_str())
    {
        return Vec::new();
    }
    vec![structural_report(
        scenario,
        specs_root,
        "invalid-falsifiability-class",
        Some(format!(
            "unknown falsifiability_class: {} (valid values: safety, liveness)",
            contract.falsifiability_class
        )),
    )]
}

fn contract_declares_tests(contract: &contracts::Contract) -> bool {
    !contract.tests.is_empty() && contract.tests.values().any(|entries| !entries.is_empty())
}

fn no_tests_findings(
    scenario: &Scenario,
    contract: &contracts::Contract,
    specs_root: &Path,
) -> Vec<ReportFinding> {
    if contract_declares_tests(contract) {
        return Vec::new();
    }
    vec![structural_report(
        scenario,
        specs_root,
        "no-tests-declared",
        None,
    )]
}

fn replacement_findings(
    scenario: &Scenario,
    contract: &contracts::Contract,
    known_ids: &HashSet<(String, String)>,
    specs_root: &Path,
) -> Vec<ReportFinding> {
    if contract.status != "superseded" {
        return Vec::new();
    }
    let replacement = (scenario.spec_path.clone(), contract.superseded_by.clone());
    if known_ids.contains(&replacement) {
        return Vec::new();
    }
    vec![structural_report(
        scenario,
        specs_root,
        "missing-replacement",
        Some(format!(
            "replacement scenario '{}' is absent from scope",
            contract.superseded_by
        )),
    )]
}

/// Liveness assertions are only falsifiable under bounded execution.
const LIVENESS_TIMEOUT_MESSAGE: &str = concat!(
    "liveness claim has no timeout_seconds on any test entry; liveness ",
    "assertions are only falsifiable under bounded execution — declare ",
    "timeout_seconds at least once"
);

/// Liveness timeout warning (gate delta C-liveness-timeout-warning): fires
/// only when ≥1 test entry exists (zero entries are governed by
/// no-tests-declared) and every entry lacks timeout_seconds.
fn liveness_timeout_findings(
    scenario: &Scenario,
    contract: &contracts::Contract,
    specs_root: &Path,
) -> Vec<ReportFinding> {
    if contract.falsifiability_class != "liveness" {
        return Vec::new();
    }
    let entries: Vec<&contracts::TestEntry> = contract.tests.values().flatten().collect();
    if entries.is_empty() || entries.iter().any(|entry| entry.timeout_seconds.is_some()) {
        return Vec::new();
    }
    vec![structural_report(
        scenario,
        specs_root,
        "missing-liveness-timeout",
        Some(LIVENESS_TIMEOUT_MESSAGE.to_string()),
    )]
}

fn orphan_reports(
    contract_files: &[(String, String, PathBuf)],
    scenarios: &BTreeMap<(String, String), &ResolvedScenario>,
    specs_root: &Path,
    covered: &BTreeMap<(String, String), Vec<String>>,
) -> Vec<ReportFinding> {
    // Derived contracts referenced by a VERIFIES link are in use even though
    // no scenario shares their id — never orphans (task 4.1).
    let covered_keys = covered_contract_keys(covered);
    let mut seen = HashSet::new();
    contract_files
        .iter()
        .filter(|(spec, id, path)| seen.insert((spec.clone(), id.clone(), path.clone())))
        .filter(|(spec, id, _)| !scenarios.contains_key(&(spec.clone(), id.clone())))
        .filter(|(spec, id, _)| !covered_keys.contains(&(spec.clone(), id.clone())))
        .map(|(spec, id, _)| orphan_report(spec, id, specs_root))
        .collect()
}

fn orphan_report(spec: &str, id: &str, specs_root: &Path) -> ReportFinding {
    report_finding(
        "orphan-toml",
        "structural",
        spec.to_string(),
        spec_markdown_path(specs_root, spec),
        ScenarioContext {
            id: id.to_string(),
            title: String::new(),
            body_markdown: String::new(),
        },
        None,
        None,
        None,
    )
}

#[cfg(test)]
pub(crate) mod tests;
