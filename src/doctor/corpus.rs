//! Contract-corpus checks: orphaned contracts and unknown archetypes.

use crate::archetypes;
use crate::contracts;
use crate::openspec;
use genesis::doctor::DoctorCheck;
use genesis::suite_linter::{LintResult, Severity};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub(super) struct OrphanContractCheck {
    pub(super) repo_root: std::path::PathBuf,
    pub(super) specs_dir: std::path::PathBuf,
}
impl DoctorCheck for OrphanContractCheck {
    fn name(&self) -> &'static str {
        "orphan-contracts"
    }
    fn description(&self) -> &'static str {
        "Check for contracts with no matching scenario"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let Some(scenarios) = discover_scenarios(&self.specs_dir) else {
            return Ok(vec![]);
        };
        let known = known_spec_slugs(&scenarios);
        let linked = linked_contract_ids(&self.specs_dir, &scenarios);
        Ok(orphan_findings(
            &self.repo_root.join(".espectacular"),
            &known,
            &linked,
        ))
    }
}

fn discover_scenarios(specs_dir: &Path) -> Option<Vec<openspec::Scenario>> {
    if !specs_dir.exists() {
        return None;
    }
    let specs_str = specs_dir.to_string_lossy().to_string();
    openspec::discover_scenarios(&specs_str).ok()
}

fn known_spec_slugs(scenarios: &[openspec::Scenario]) -> HashSet<(String, String)> {
    scenarios
        .iter()
        .map(|s| (s.spec_path.clone(), s.id.clone()))
        .collect()
}

/// Derived contracts referenced by VERIFIES bullets are in use even though no
/// scenario shares their filename — same suppression as the ah check gate
/// (task 4.1 / full-corpus migration).
fn linked_contract_ids(
    specs_dir: &Path,
    scenarios: &[openspec::Scenario],
) -> HashSet<(String, String)> {
    let specs_str = specs_dir.to_string_lossy().to_string();
    let file_ids = openspec::frontmatter_ids(&specs_str).unwrap_or_default();
    let mut linked: HashSet<(String, String)> = HashSet::new();
    for s in scenarios {
        let file_id = file_ids.get(&s.spec_path).map(String::as_str);
        for property_id in crate::check::verifies_property_ids(&s.body, file_id) {
            linked.insert((
                s.spec_path.clone(),
                crate::sync::slugify_property_id(property_id.as_str()),
            ));
        }
    }
    linked
}

fn orphan_findings(
    espectacular_dir: &Path,
    known: &HashSet<(String, String)>,
    linked: &HashSet<(String, String)>,
) -> Vec<LintResult> {
    let mut results = Vec::new();
    for (spec_name, contract_dir) in contract_dirs(espectacular_dir) {
        for slug in contract_slugs(&contract_dir) {
            let key = (spec_name.clone(), slug.clone());
            if known.contains(&key) || linked.contains(&key) {
                continue;
            }
            results.push(LintResult::new(
                format!("contract {spec_name}/{slug}.toml has no matching scenario"),
                Severity::Error,
            ));
        }
    }
    results
}

/// Per-capability contract directories under `.espectacular` (changes excluded).
fn contract_dirs(espectacular_dir: &Path) -> Vec<(String, std::path::PathBuf)> {
    let Ok(entries) = fs::read_dir(espectacular_dir) else {
        return vec![];
    };
    let mut dirs: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if name != "changes" {
            dirs.push((name, path));
        }
    }
    dirs
}

fn contract_slugs(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("toml"))
        .filter_map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .collect()
}

pub(super) struct UnknownArchetypeCheck {
    pub(super) repo_root: std::path::PathBuf,
    pub(super) specs_dir: std::path::PathBuf,
}
impl DoctorCheck for UnknownArchetypeCheck {
    fn name(&self) -> &'static str {
        "unknown-archetypes"
    }
    fn description(&self) -> &'static str {
        "Check contracts reference known archetypes"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if !self.specs_dir.exists() {
            return Ok(vec![]);
        }
        let espectacular_dir = self.repo_root.join(".espectacular");
        Ok(unknown_archetype_findings(&espectacular_dir))
    }
}

fn unknown_archetype_findings(espectacular_dir: &Path) -> Vec<LintResult> {
    let mut results = Vec::new();
    for (spec_name, contract_dir) in contract_dirs(espectacular_dir) {
        for slug in contract_slugs(&contract_dir) {
            let path = contract_dir.join(format!("{slug}.toml"));
            if let Some(archetype) = unknown_archetype(&path) {
                results.push(LintResult::new(
                    format!("{spec_name}/{slug}.toml has unknown archetype: {archetype}"),
                    Severity::Error,
                ));
            }
        }
    }
    results
}

/// The contract's archetype when it is set but not a known one.
fn unknown_archetype(path: &Path) -> Option<String> {
    let contract = contracts::load_contract(path.to_str()?).ok()?;
    if contract.archetype.is_empty() || archetypes::is_known(&contract.archetype) {
        return None;
    }
    Some(contract.archetype)
}
