use crate::check;
use crate::contracts;
use crate::openspec;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct ReportOutput {
    pub matrix: Vec<MatrixRow>,
    pub summary: ReportSummary,
}

#[derive(Debug, Serialize)]
pub struct MatrixRow {
    pub spec: String,
    pub archetype: String,
    pub covered: usize,
    pub missing: usize,
    pub failing: usize,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct ReportSummary {
    pub total_scenarios: usize,
    pub total_contracts: usize,
    pub covered: usize,
    pub missing: usize,
    pub failing: usize,
}

pub fn run_report(repo_root: &Path) -> anyhow::Result<ReportOutput> {
    let config_path = repo_root.join(".espectacular/config.toml");
    let cfg = crate::config::load_config(config_path.to_str().unwrap())?;
    let specs_dir = repo_root.join(&cfg.paths.specs);
    let contracts_dir = repo_root.join(".espectacular");

    let scenarios = openspec::discover_scenarios(specs_dir.to_str().unwrap())?;
    let contract_files = check::collect_base_contract_files(&contracts_dir);

    // Build a lookup of spec+id -> contract info
    let mut contracts_map: BTreeMap<(String, String), String> = BTreeMap::new();
    for (spec, id, _path) in &contract_files {
        contracts_map.insert((spec.clone(), id.clone()), String::new());
    }

    // Group scenarios by (spec, archetype)
    let mut rows: BTreeMap<(String, String), MatrixRow> = BTreeMap::new();

    for scenario in &scenarios {
        let _key = (scenario.spec_path.clone(), String::new()); // archetype comes from contract
        let entry = rows
            .entry((scenario.spec_path.clone(), String::new()))
            .or_insert(MatrixRow {
                spec: scenario.spec_path.clone(),
                archetype: String::new(),
                covered: 0,
                missing: 0,
                failing: 0,
                total: 0,
            });
        entry.total += 1;

        let has_contract =
            contracts_map.contains_key(&(scenario.spec_path.clone(), scenario.id.clone()));
        if has_contract {
            entry.covered += 1;
        } else {
            entry.missing += 1;
        }
    }

    // Try to read archetype from each contract
    for (spec, _id, path) in &contract_files {
        if let Ok(contract) = contracts::load_contract(path.to_str().unwrap()) {
            if !contract.archetype.is_empty() {
                let key = (spec.clone(), contract.archetype.clone());
                // Update or add a row for this archetype
                let entry = rows.entry(key).or_insert(MatrixRow {
                    spec: spec.clone(),
                    archetype: contract.archetype.clone(),
                    covered: 0,
                    missing: 0,
                    failing: 0,
                    total: 0,
                });
                entry.archetype = contract.archetype.clone();
            }
        }
    }

    // Calculate aggregate
    let mut total_covered = 0usize;
    let mut total_missing = 0usize;
    let mut total_failing = 0usize;

    for row in rows.values() {
        total_covered += row.covered;
        total_missing += row.missing;
        total_failing += row.failing;
    }

    let matrix: Vec<MatrixRow> = rows.into_values().collect();
    let total_scenarios = matrix.iter().map(|r| r.total).sum();
    let total_contracts = total_covered + total_failing;

    Ok(ReportOutput {
        matrix,
        summary: ReportSummary {
            total_scenarios,
            total_contracts,
            covered: total_covered,
            missing: total_missing,
            failing: total_failing,
        },
    })
}
