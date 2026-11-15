//! Derived-contract drift detection (`contract-stale`).
//!
//! Purpose: verify that a derived contract's `derived_from` hash still matches
//! the canonical serialization of its source property row (task 4.2,
//! C-derived-stale).
//!
//! Responsibilities: group derived contracts per spec, run spk's spec IR
//! parse once per spec, compare hashes, and attach each stale finding to the
//! covering scenario when one exists.
//!
//! Rationale: detection degrades silently when spk is missing or a spec fails
//! to parse (design D2 — validity stays with spk; a broken parse is caught by
//! `ah lint`), so every failure path here returns "no findings" rather than
//! failing the gate.

use super::report::{spec_markdown_path, structural_report, ReportFinding};
use super::scope::{ResolvedScenario, ResolvedScope};
use crate::derive;
use crate::openspec::Scenario;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// spk binary resolved from PATH (same convention as `ah sync`).
pub(crate) const SPK_PROGRAM: &str = "spk";

/// contract-stale drift detection (derive-contracts-from-specodelic task 4.2,
/// C-derived-stale): a derived contract's `derived_from` hash must match the
/// canonical serialization of its source property row. Findings are attached
/// to the covering scenario when one exists, else to the contract id itself.
pub(crate) fn contract_stale_findings(
    specs_root: &Path,
    scope: &ResolvedScope,
    spk: &str,
) -> Vec<ReportFinding> {
    contract_stale_for(
        specs_root,
        &scope.contract_files,
        &scope.covered,
        &scope.scenarios,
        spk,
    )
}

fn contract_stale_for(
    specs_root: &Path,
    contract_files: &[(String, String, PathBuf)],
    covered: &BTreeMap<(String, String), Vec<String>>,
    scenarios: &[ResolvedScenario],
    spk: &str,
) -> Vec<ReportFinding> {
    let by_spec = derived_contracts_by_spec(contract_files);
    let covering = covering_scenarios(covered, scenarios);
    by_spec
        .iter()
        .flat_map(|(spec, derived_contracts)| {
            stale_spec_findings(specs_root, spec, derived_contracts, &covering, spk)
        })
        .collect()
}

/// Derived contracts (`derived_from` non-empty) grouped by spec.
fn derived_contracts_by_spec(
    contract_files: &[(String, String, PathBuf)],
) -> BTreeMap<String, Vec<(String, String)>> {
    let mut by_spec: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (spec, id, path) in contract_files {
        let Some(derived) = derived_from(path) else {
            continue;
        };
        by_spec
            .entry(spec.clone())
            .or_default()
            .push((id.clone(), derived));
    }
    by_spec
}

fn derived_from(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let table = text.parse::<toml::Table>().ok()?;
    let derived = table.get("derived_from").and_then(toml::Value::as_str)?;
    if derived.is_empty() {
        return None;
    }
    Some(derived.to_string())
}

/// (spec, contract id) → covering scenario, for finding attachment.
fn covering_scenarios<'a>(
    covered: &BTreeMap<(String, String), Vec<String>>,
    scenarios: &'a [ResolvedScenario],
) -> BTreeMap<(String, String), &'a Scenario> {
    let mut covering = BTreeMap::new();
    for ((spec, scenario_id), contract_ids) in covered {
        let Some(resolved) = scenarios.iter().find(|resolved| {
            resolved.scenario.spec_path == *spec && resolved.scenario.id == *scenario_id
        }) else {
            continue;
        };
        for contract_id in contract_ids {
            covering.insert((spec.clone(), contract_id.clone()), &resolved.scenario);
        }
    }
    covering
}

fn stale_spec_findings(
    specs_root: &Path,
    spec: &str,
    derived_contracts: &[(String, String)],
    covering: &BTreeMap<(String, String), &Scenario>,
    spk: &str,
) -> Vec<ReportFinding> {
    let markdown = spec_markdown_path(specs_root, spec);
    let Ok(ir) = derive::parse_spec_ir(spk, Path::new(&markdown)) else {
        return Vec::new();
    };
    derived_contracts
        .iter()
        .filter_map(|(contract_id, derived)| {
            stale_finding(specs_root, spec, contract_id, derived, &ir, covering)
        })
        .collect()
}

fn stale_finding(
    specs_root: &Path,
    spec: &str,
    contract_id: &str,
    derived: &str,
    ir: &derive::ParseData,
    covering: &BTreeMap<(String, String), &Scenario>,
) -> Option<ReportFinding> {
    let (property_id, stored) = derived.split_once('@')?;
    let message = stale_message(ir, property_id, derived, stored)?;
    let synthetic = Scenario {
        id: contract_id.to_string(),
        heading: String::new(),
        spec_path: spec.to_string(),
        source_line: 0,
        body: String::new(),
    };
    let scenario = covering
        .get(&(spec.to_string(), contract_id.to_string()))
        .copied()
        .unwrap_or(&synthetic);
    Some(structural_report(
        scenario,
        specs_root,
        "contract-stale",
        Some(message),
    ))
}

fn stale_message(
    ir: &derive::ParseData,
    property_id: &str,
    derived: &str,
    stored: &str,
) -> Option<String> {
    match ir.properties.iter().find(|row| row.id == property_id) {
        None => Some(format!(
            "derived_from references property {property_id}, which is absent from the specodelic Properties table"
        )),
        Some(row) => {
            let fresh = derive::canonical_hash(row);
            (fresh != derived).then(|| {
                format!(
                    "derived_from hash @{stored} does not match specodelic row {property_id} (@{}); run ah sync to refresh derived fields",
                    fresh.split_once('@').map(|(_, hash)| hash).unwrap_or("?")
                )
            })
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
