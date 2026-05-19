use crate::contracts;
use crate::openspec;
use std::collections::{HashMap, HashSet};
use std::fs;
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
    fn structural(spec_path: &str, scenario_id: &str, kind: &str) -> Self {
        Finding {
            spec_path: spec_path.to_string(),
            scenario_id: scenario_id.to_string(),
            kind: kind.to_string(),
            category: "structural".to_string(),
            source_location: None,
        }
    }
}

fn contract_findings_for_scenario(
    spec_path: &str,
    scenario_id: &str,
    contracts_dir: &str,
) -> anyhow::Result<Option<Finding>> {
    let contract_path = Path::new(contracts_dir)
        .join(spec_path)
        .join(format!("{scenario_id}.toml"));
    if !contract_path.exists() {
        return Ok(Some(Finding::structural(spec_path, scenario_id, "missing-contract")));
    }
    match contracts::load_contract(contract_path.to_str().unwrap()) {
        Err(_) => Ok(Some(Finding::structural(spec_path, scenario_id, "invalid-contract"))),
        Ok(contract) => {
            if contract.tests.is_empty() || contract.tests.values().all(|v| v.is_empty()) {
                Ok(Some(Finding::structural(spec_path, scenario_id, "no-tests-declared")))
            } else {
                Ok(None)
            }
        }
    }
}

fn orphan_findings_for_contracts_dir(
    contracts_dir: &str,
    known: &HashMap<&str, HashSet<&str>>,
) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for spec_entry in fs::read_dir(contracts_dir)? {
        let spec_entry = spec_entry?;
        if !spec_entry.file_type()?.is_dir() {
            continue;
        }
        let spec_name = spec_entry.file_name().to_string_lossy().into_owned();
        let known_ids = known.get(spec_name.as_str()).cloned().unwrap_or_default();
        for toml_entry in fs::read_dir(spec_entry.path())? {
            let toml_entry = toml_entry?;
            let fname = toml_entry.file_name().to_string_lossy().into_owned();
            if !fname.ends_with(".toml") {
                continue;
            }
            let stem = fname.trim_end_matches(".toml").to_string();
            if !known_ids.contains(stem.as_str()) {
                findings.push(Finding::structural(&spec_name, &stem, "orphan-contract"));
            }
        }
    }
    Ok(findings)
}

pub fn structural_findings(specs_dir: &str, contracts_dir: &str) -> anyhow::Result<Vec<Finding>> {
    let scenarios = openspec::discover_scenarios(specs_dir)?;
    let collision_list = openspec::detect_slug_collisions(&scenarios);
    let collision_ids: HashSet<(&str, &str)> = collision_list
        .iter()
        .map(|(spec, id, _)| (spec.as_str(), id.as_str()))
        .collect();

    let mut findings: Vec<Finding> = collision_list
        .iter()
        .map(|(spec, id, _)| Finding::structural(spec, id, "duplicate-id"))
        .collect();

    let mut scenario_ids: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut checked: HashSet<(&str, &str)> = HashSet::new();
    for s in &scenarios {
        scenario_ids.entry(&s.spec_path).or_default().insert(&s.id);
        let key = (s.spec_path.as_str(), s.id.as_str());
        if collision_ids.contains(&key) || !checked.insert(key) {
            continue;
        }
        if let Some(f) = contract_findings_for_scenario(&s.spec_path, &s.id, contracts_dir)? {
            findings.push(f);
        }
    }

    findings.extend(orphan_findings_for_contracts_dir(contracts_dir, &scenario_ids)?);
    findings.sort();
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPECS: &str = "tests/fixtures/four-findings/openspec/specs";
    const CONTRACTS: &str = "tests/fixtures/four-findings/.espectacular";

    #[test]
    fn four_findings_fixture_emits_exactly_four() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert_eq!(
            findings.len(),
            4,
            "expected 4 structural findings, got: {:#?}",
            findings.iter().map(|f| &f.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn findings_ordered_by_spec_scenario_kind() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        let mut sorted = findings.clone();
        sorted.sort();
        assert_eq!(findings, sorted, "findings must be deterministically ordered");
    }

    #[test]
    fn missing_contract_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            findings.iter().any(|f| f.kind == "missing-contract" && f.scenario_id == "missing-contract"),
            "expected missing-contract finding"
        );
    }

    #[test]
    fn orphan_contract_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            findings.iter().any(|f| f.kind == "orphan-contract" && f.scenario_id == "orphan-contract"),
            "expected orphan-contract finding"
        );
    }

    #[test]
    fn no_tests_declared_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            findings.iter().any(|f| f.kind == "no-tests-declared" && f.scenario_id == "no-tests-declared"),
            "expected no-tests-declared finding"
        );
    }

    #[test]
    fn duplicate_id_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            findings.iter().any(|f| f.kind == "duplicate-id"),
            "expected duplicate-id finding"
        );
    }

    #[test]
    fn all_findings_have_structural_category() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        for f in &findings {
            assert_eq!(f.category, "structural", "finding {:?} must be structural", f.kind);
        }
    }

    #[test]
    fn valid_scenario_produces_no_finding() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            !findings.iter().any(|f| f.scenario_id == "has-contract"),
            "valid scenario should not produce a finding"
        );
    }
}
