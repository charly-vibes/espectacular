//! Scope resolution: which scenarios and contracts this run covers.
//!
//! Purpose: build the `ResolvedScope` — deployed scenarios plus the selected
//! change overlays — and the findings that only resolution can see (slug
//! collisions, staged-contract conflicts, conflicting redefinitions).
//!
//! Responsibilities: discovery of the deployed specs tree, indexing of the
//! deployed scenarios, VERIFIES coverage redirection, and the deploy-time
//! tolerance for a repo with no specs dir yet. Overlay merging lives in the
//! [`overlay`] submodule.
//!
//! Rationale: overlay merging decides what the gate actually evaluates, so
//! each rule lives in its own named step instead of one nested walk; the
//! byte-identical exemption keeps openspec's MODIFIED-wholesale deltas from
//! turning verbatim carry-along into conflicts.

mod overlay;

use super::report::ReportFinding;
use super::verifies::{contract_path, resolve_verifies_coverage, slug_collision_findings};
use crate::openspec::{self, Scenario};
use overlay::{apply_changes, OverlayDirs};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct ResolvedScenario {
    pub(crate) scenario: Scenario,
    pub(crate) contract_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct ResolvedScope {
    pub(crate) scenarios: Vec<ResolvedScenario>,
    pub(crate) contract_files: Vec<(String, String, PathBuf)>,
    pub(crate) changes: Vec<String>,
    pub(crate) findings: Vec<ReportFinding>,
    /// VERIFIES coverage (derive-contracts-from-specodelic task 4.1):
    /// (spec_path, scenario_id) → derived contract id. A scenario whose body
    /// carries `- **VERIFIES** [[<file-id>.P-x]]` is covered by the property's
    /// derived contract (`.espectacular/<spec>/<slug>.toml`) when it exists
    /// (design D1); the entry is absent when no derived contract backs the
    /// link, leaving the scenario's own contract requirement unchanged
    /// (C-unlinked-unchanged).
    pub(crate) covered: BTreeMap<(String, String), Vec<String>>,
}

pub(crate) fn resolve_scope(
    specs_dir: &Path,
    contracts_dir: &Path,
    changes_dir: &Path,
    selected_changes: &[String],
) -> anyhow::Result<ResolvedScope> {
    let base_scenarios = discover_base_scenarios(specs_dir)?;
    // Deployed-spec slug collisions must be detected on the raw discovery
    // list, BEFORE the (spec, id) map below collapses same-key duplicates
    // (last wins) — otherwise the slug-collision finding is dead code in
    // the ah check binary and only structural_findings()/doctor see it
    // (espectacular-ybs).
    let mut findings = slug_collision_findings(&base_scenarios, specs_dir);
    let mut scenarios = index_scenarios(base_scenarios, contracts_dir);
    // Pre-overlay keys: the byte-identical carry-along exemption (4u6) is
    // deployed-only; two changes adding the same scenario stay conflicting.
    let deployed_keys: HashSet<(String, String)> = scenarios.keys().cloned().collect();
    let changes = sorted_changes(selected_changes);
    let dirs = OverlayDirs::new(specs_dir, contracts_dir, changes_dir);
    let contract_files = apply_changes(
        &changes,
        dirs,
        &deployed_keys,
        &mut scenarios,
        &mut findings,
    )?;

    // Frontmatter id per deployed spec — VERIFIES links are file-local
    // (`<own-file-id>.<property-id>` under the rev-18 naming law).
    let file_ids = frontmatter_ids(specs_dir);
    let covered = resolve_verifies_coverage(&mut scenarios, contracts_dir, &file_ids);

    Ok(ResolvedScope {
        scenarios: scenarios.into_values().collect(),
        contract_files,
        changes,
        findings,
        covered,
    })
}

/// GH#28.2 / espectacular-iq4: a repo with no deployed specs yet (changes
/// only) is a legitimate state — ah doctor classifies a missing specs dir
/// as a missing-specs-dir finding, so check degrades consistently to an
/// empty base scope instead of failing closed with a bare ENOENT (0.3.0
/// parity). A selected change with missing staged specs still bails.
fn discover_base_scenarios(specs_dir: &Path) -> anyhow::Result<Vec<Scenario>> {
    if !specs_dir.exists() {
        return Ok(Vec::new());
    }
    openspec::discover_scenarios(specs_dir.to_str().unwrap())
}

/// Index the deployed scenarios by (spec, id) at their deployed contract path.
fn index_scenarios(
    base_scenarios: Vec<Scenario>,
    contracts_dir: &Path,
) -> BTreeMap<(String, String), ResolvedScenario> {
    base_scenarios
        .into_iter()
        .map(|scenario| {
            let key = scenario_key(&scenario);
            let contract_path = contract_path(contracts_dir, &scenario.spec_path, &scenario.id);
            (
                key,
                ResolvedScenario {
                    scenario,
                    contract_path,
                },
            )
        })
        .collect()
}

fn frontmatter_ids(specs_dir: &Path) -> BTreeMap<String, String> {
    if !specs_dir.exists() {
        return BTreeMap::new();
    }
    openspec::frontmatter_ids(specs_dir.to_str().unwrap()).unwrap_or_default()
}

fn sorted_changes(selected_changes: &[String]) -> Vec<String> {
    let mut changes = selected_changes.to_vec();
    changes.sort();
    changes.dedup();
    changes
}

pub(crate) fn scenario_key(scenario: &Scenario) -> (String, String) {
    (scenario.spec_path.clone(), scenario.id.clone())
}

#[cfg(test)]
pub(crate) mod tests;
