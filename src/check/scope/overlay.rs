//! Change-overlay merging: staged contracts and overlay scenarios.
//!
//! Purpose: fold each selected change onto the deployed scope, deciding per
//! scenario whether the overlay inserts, deliberately replaces, verbatim
//! carries along, or conflicts.
//!
//! Responsibilities: staged-contract collection (with conflicting staged
//! updates reported), the overlay scenario merge rules, and the overlay
//! conflict findings.
//!
//! Rationale: these rules decide what the gate evaluates, so each outcome is
//! a named `OverlayAction` instead of nested conditionals — a mis-merge
//! either hides a scenario or invents one.

use super::{scenario_key, ResolvedScenario};
use crate::check::contract_files::{collect_base_contract_files, collect_contract_files};
use crate::check::report::{report_finding, spec_markdown_path, ReportFinding, ScenarioContext};
use crate::openspec::{self, Scenario};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// The three roots an overlay merge reads from.
#[derive(Clone, Copy)]
pub(crate) struct OverlayDirs<'a> {
    specs_dir: &'a Path,
    contracts_dir: &'a Path,
    changes_dir: &'a Path,
}

impl<'a> OverlayDirs<'a> {
    pub(crate) fn new(specs_dir: &'a Path, contracts_dir: &'a Path, changes_dir: &'a Path) -> Self {
        OverlayDirs {
            specs_dir,
            contracts_dir,
            changes_dir,
        }
    }
}

/// Merge every selected change onto the scope, returning the base contract
/// files plus the staged ones.
pub(crate) fn apply_changes(
    changes: &[String],
    dirs: OverlayDirs<'_>,
    deployed_keys: &HashSet<(String, String)>,
    scenarios: &mut BTreeMap<(String, String), ResolvedScenario>,
    findings: &mut Vec<ReportFinding>,
) -> anyhow::Result<Vec<(String, String, PathBuf)>> {
    let mut contract_files = collect_base_contract_files(dirs.contracts_dir);
    let mut contract_overrides: HashMap<(String, String), PathBuf> = HashMap::new();
    for change in changes {
        apply_change_overlay(
            change,
            dirs,
            deployed_keys,
            scenarios,
            &mut contract_files,
            &mut contract_overrides,
            findings,
        )?;
    }
    Ok(contract_files)
}

/// Merge one selected change: its staged contracts first (they signal
/// deliberate modifications), then its overlay scenarios.
fn apply_change_overlay(
    change: &str,
    dirs: OverlayDirs<'_>,
    deployed_keys: &HashSet<(String, String)>,
    scenarios: &mut BTreeMap<(String, String), ResolvedScenario>,
    contract_files: &mut Vec<(String, String, PathBuf)>,
    contract_overrides: &mut HashMap<(String, String), PathBuf>,
    findings: &mut Vec<ReportFinding>,
) -> anyhow::Result<()> {
    let change_specs = dirs.changes_dir.join(change).join("specs");
    if !change_specs.exists() {
        anyhow::bail!(
            "change '{change}' does not exist at {}",
            change_specs.display()
        );
    }
    // Staged contracts are collected BEFORE the overlay scenarios so a staged
    // contract for a (spec, id) can act as the explicit signal that the change
    // deliberately MODIFIES a deployed scenario's text
    // (allow-overlay-scenario-modification).
    let staged = collect_staged_contracts(
        change,
        dirs,
        scenarios,
        contract_files,
        contract_overrides,
        findings,
    );
    merge_overlay_scenarios(
        change,
        &change_specs,
        dirs.contracts_dir,
        deployed_keys,
        &staged,
        scenarios,
        findings,
    )
}

/// Collect `<contracts>/changes/<change>/<cap>/<id>.toml` into the scope and
/// return the (spec, id) keys this change staged. A second staged update for
/// the same key — within one change or across changes — is an overlay
/// conflict.
fn collect_staged_contracts(
    change: &str,
    dirs: OverlayDirs<'_>,
    scenarios: &mut BTreeMap<(String, String), ResolvedScenario>,
    contract_files: &mut Vec<(String, String, PathBuf)>,
    contract_overrides: &mut HashMap<(String, String), PathBuf>,
    findings: &mut Vec<ReportFinding>,
) -> HashSet<(String, String)> {
    let mut staged = HashSet::new();
    let staged_root = dirs.contracts_dir.join("changes").join(change);
    for (spec, id, path) in collect_contract_files(&staged_root) {
        let key = (spec.clone(), id.clone());
        if let Some(previous) = contract_overrides.insert(key.clone(), path.clone()) {
            let finding = override_conflict_finding(dirs.specs_dir, &spec, &id, &previous, &path);
            findings.push(finding);
            continue;
        }
        contract_files.push((spec, id, path.clone()));
        staged.insert(key.clone());
        if let Some(existing) = scenarios.get_mut(&key) {
            existing.contract_path = path;
        }
    }
    staged
}

/// Merge this change's overlay scenarios one by one.
#[allow(clippy::too_many_arguments)]
fn merge_overlay_scenarios(
    change: &str,
    change_specs: &Path,
    contracts_dir: &Path,
    deployed_keys: &HashSet<(String, String)>,
    staged: &HashSet<(String, String)>,
    scenarios: &mut BTreeMap<(String, String), ResolvedScenario>,
    findings: &mut Vec<ReportFinding>,
) -> anyhow::Result<()> {
    for scenario in openspec::discover_scenarios(change_specs.to_str().unwrap())? {
        merge_overlay_scenario(
            change,
            change_specs,
            contracts_dir,
            deployed_keys,
            staged,
            scenario,
            scenarios,
            findings,
        );
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OverlayAction {
    /// The overlay adds a scenario the scope does not know yet.
    Insert,
    /// A staged contract signals a deliberate modification of the text.
    Replace,
    /// Byte-identical carry-along of an unchanged deployed scenario.
    Keep,
    /// An unsignaled redefinition of a deployed (or already overlaid) scenario.
    Conflict,
}

#[allow(clippy::too_many_arguments)]
fn merge_overlay_scenario(
    change: &str,
    change_specs: &Path,
    contracts_dir: &Path,
    deployed_keys: &HashSet<(String, String)>,
    staged: &HashSet<(String, String)>,
    scenario: Scenario,
    scenarios: &mut BTreeMap<(String, String), ResolvedScenario>,
    findings: &mut Vec<ReportFinding>,
) {
    let key = scenario_key(&scenario);
    let action = overlay_action(deployed_keys, staged, scenarios.get(&key), &scenario);
    match action {
        OverlayAction::Insert => {
            insert_overlay_scenario(contracts_dir, change, key, scenario, scenarios)
        }
        OverlayAction::Replace => {
            if let Some(existing) = scenarios.get_mut(&key) {
                existing.scenario = scenario;
            }
        }
        OverlayAction::Keep => {}
        OverlayAction::Conflict => {
            let finding = overlay_conflict_finding(change, change_specs, scenario);
            findings.push(finding);
        }
    }
}

fn overlay_action(
    deployed_keys: &HashSet<(String, String)>,
    staged: &HashSet<(String, String)>,
    existing: Option<&ResolvedScenario>,
    scenario: &Scenario,
) -> OverlayAction {
    let Some(existing) = existing else {
        return OverlayAction::Insert;
    };
    let key = scenario_key(scenario);
    if staged.contains(&key) {
        return OverlayAction::Replace;
    }
    // Byte-identical carry-along (espectacular-4u6): openspec's
    // MODIFIED-wholesale rule forces unchanged scenarios to be re-declared
    // verbatim in every MODIFIED delta. The gate spec pins overlay-conflict
    // to redefinitions "with different body text", so an identical body
    // without a staged contract is a no-op — deployed text stands.
    if deployed_keys.contains(&key) && existing.scenario.body.trim() == scenario.body.trim() {
        return OverlayAction::Keep;
    }
    OverlayAction::Conflict
}

fn insert_overlay_scenario(
    contracts_dir: &Path,
    change: &str,
    key: (String, String),
    scenario: Scenario,
    scenarios: &mut BTreeMap<(String, String), ResolvedScenario>,
) {
    let contract_path = contracts_dir
        .join("changes")
        .join(change)
        .join(&scenario.spec_path)
        .join(format!("{}.toml", scenario.id));
    scenarios.insert(
        key,
        ResolvedScenario {
            scenario,
            contract_path,
        },
    );
}

fn overlay_conflict_finding(
    change: &str,
    change_specs: &Path,
    scenario: Scenario,
) -> ReportFinding {
    let spec_path = change_specs
        .join(&scenario.spec_path)
        .join("spec.md")
        .to_string_lossy()
        .into_owned();
    report_finding(
        "overlay-conflict",
        "structural",
        scenario.spec_path.clone(),
        spec_path,
        ScenarioContext {
            id: scenario.id.clone(),
            title: scenario.heading.clone(),
            body_markdown: scenario.body.clone(),
        },
        Some(scenario.body.clone()),
        None,
        Some(format!(
            "change '{change}' defines scenario '{}', which already exists in scope",
            scenario.id
        )),
    )
}

fn override_conflict_finding(
    specs_dir: &Path,
    spec: &str,
    id: &str,
    previous: &Path,
    path: &Path,
) -> ReportFinding {
    report_finding(
        "overlay-conflict",
        "structural",
        spec.to_string(),
        spec_markdown_path(specs_dir, spec),
        ScenarioContext {
            id: id.to_string(),
            title: String::new(),
            body_markdown: String::new(),
        },
        None,
        None,
        Some(format!(
            "multiple staged contract updates for {}:{} ({} and {})",
            spec,
            id,
            previous.display(),
            path.display()
        )),
    )
}
