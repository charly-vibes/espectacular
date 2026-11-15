//! VERIFIES coverage: links from scenarios to property-derived contracts.
//!
//! Purpose: parse `- **VERIFIES** [[<file-id>.P-x]]` bullets and redirect a
//! covered scenario at its property's derived contract, recording which
//! derived contracts are in use (they are never orphans).
//!
//! Responsibilities: link parsing under the rev-18 file-local naming law, the
//! coverage redirect, the coverage map, the deployed contract path helper,
//! and deployed-spec slug-collision findings.
//!
//! Rationale: a link's derived contract, not the scenario id, owns the tests
//! for a property; keeping parsing and redirection together means the naming
//! law has exactly one implementation shared by check, doctor, report, and
//! lint.

use super::report::{structural_report, ReportFinding};
use super::scope::ResolvedScenario;
use crate::openspec;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Flag every scenario whose (spec, id) appears more than once in the raw
/// list. Detection runs over the raw discovery output, not the collapsed
/// map, so colliding duplicates are seen before they merge (last wins).
pub(crate) fn slug_collision_findings(
    scenarios: &[openspec::Scenario],
    specs_root: &Path,
) -> Vec<ReportFinding> {
    openspec::detect_slug_collisions(scenarios)
        .into_iter()
        .filter_map(|(spec, id, _)| {
            scenarios
                .iter()
                .find(|scenario| scenario.spec_path == spec && scenario.id == id)
                .map(|scenario| structural_report(scenario, specs_root, "slug-collision", None))
        })
        .collect()
}

/// Extract property ids from `- **VERIFIES** [[P-x]]` bullets in a scenario
/// body (design D1 coverage convention). The link is file-local: under the
/// specodelic Revision 18 naming law it reads `<own-file-id>.<property-id>`
/// (the legacy rev-17 spelling was `spec.<property-id>`), so the file's own
/// frontmatter id is stripped when present; a dotted link whose prefix does
/// not match falls back to everything after the last dot. Bare property ids
/// pass through unchanged.
pub(crate) fn verifies_property_ids(body: &str, file_id: Option<&str>) -> Vec<String> {
    let mut ids = Vec::new();
    for line in body.lines() {
        let Some(rest) = line.trim().strip_prefix("- **VERIFIES** [[") else {
            continue;
        };
        let Some(link) = rest.strip_suffix("]]") else {
            continue;
        };
        if link.is_empty() {
            continue;
        }
        let property_id = match file_id {
            Some(fid) if link.starts_with(&format!("{fid}.")) => &link[fid.len() + 1..],
            _ => link.rsplit('.').next().unwrap_or(link),
        };
        if !property_id.is_empty() {
            ids.push(property_id.to_string());
        }
    }
    ids
}

/// Redirect VERIFIES-linked scenarios at their property's derived contract
/// and record the coverage map. A link whose derived contract file is absent
/// leaves the scenario untouched (falls back to its own contract path).
/// Multi-link scenarios (two VERIFIES bullets) redirect at the first link's
/// contract but record every linked contract as in use — the second derived
/// contract must never be orphaned.
pub(crate) fn resolve_verifies_coverage(
    scenarios: &mut BTreeMap<(String, String), ResolvedScenario>,
    contracts_dir: &Path,
    file_ids: &BTreeMap<String, String>,
) -> BTreeMap<(String, String), Vec<String>> {
    let mut covered = BTreeMap::new();
    let mut redirected = HashSet::new();
    for ((spec, id), resolved) in scenarios.iter_mut() {
        let file_id = file_ids.get(spec).map(String::as_str);
        for property_id in verifies_property_ids(&resolved.scenario.body, file_id) {
            record_coverage(
                &mut covered,
                &mut redirected,
                resolved,
                spec,
                id,
                &property_id,
                contracts_dir,
            );
        }
    }
    covered
}

fn record_coverage(
    covered: &mut BTreeMap<(String, String), Vec<String>>,
    redirected: &mut HashSet<(String, String)>,
    resolved: &mut ResolvedScenario,
    spec: &str,
    id: &str,
    property_id: &str,
    contracts_dir: &Path,
) {
    let contract_id = crate::sync::slugify_property_id(property_id);
    let path = contracts_dir.join(spec).join(format!("{contract_id}.toml"));
    if !path.exists() {
        return;
    }
    if redirected.insert((spec.to_string(), id.to_string())) {
        resolved.contract_path = path;
    }
    covered
        .entry((spec.to_string(), id.to_string()))
        .or_default()
        .push(contract_id);
}

pub(crate) fn contract_path(contracts_root: &Path, spec: &str, id: &str) -> PathBuf {
    contracts_root.join(spec).join(format!("{id}.toml"))
}

/// Coverage keys of a scenario, used by the orphan check: derived contracts
/// referenced by a VERIFIES link are in use even without a scenario id.
pub(crate) fn covered_contract_keys(
    covered: &BTreeMap<(String, String), Vec<String>>,
) -> HashSet<(String, String)> {
    covered
        .iter()
        .flat_map(|((spec, _), contract_ids)| {
            contract_ids
                .iter()
                .map(move |contract_id| (spec.clone(), contract_id.clone()))
        })
        .collect()
}

#[cfg(test)]
mod tests;
