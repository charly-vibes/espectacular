use crate::adapters;
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
    let cfg = crate::config::load(repo_root)?;
    let specs_dir = repo_root.join(&cfg.paths.specs);
    let contracts_dir = repo_root.join(".espectacular");

    let scenarios = openspec::discover_scenarios(specs_dir.to_str().unwrap())?;
    let contract_files = check::collect_base_contract_files(&contracts_dir);

    // Build a lookup of spec+id -> (archetype, path)
    let mut contracts_map: BTreeMap<(String, String), (String, std::path::PathBuf)> =
        BTreeMap::new();
    for (spec, id, path) in &contract_files {
        let archetype = contracts::load_contract(path.to_str().unwrap())
            .map(|c| c.archetype)
            .unwrap_or_default();
        contracts_map.insert((spec.clone(), id.clone()), (archetype, path.clone()));
    }

    // Group scenarios by (spec, archetype) — attribute each scenario to its
    // contract's resolved archetype so the archetype row owns real counts.
    let mut rows: BTreeMap<(String, String), MatrixRow> = BTreeMap::new();

    for scenario in &scenarios {
        let key = (scenario.spec_path.clone(), scenario.id.clone());
        let archetype = contracts_map
            .get(&key)
            .map(|(a, _)| a.clone())
            .unwrap_or_default();
        let entry = rows
            .entry((scenario.spec_path.clone(), archetype.clone()))
            .or_insert(MatrixRow {
                spec: scenario.spec_path.clone(),
                archetype: archetype.clone(),
                covered: 0,
                missing: 0,
                failing: 0,
                total: 0,
            });
        entry.total += 1;

        if contracts_map.contains_key(&key) {
            entry.covered += 1;
        } else {
            entry.missing += 1;
        }
    }

    // Run declared tests and count failures per (spec, archetype)
    let mut failures: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (spec, _scenario_id, path) in &contract_files {
        let contract = match contracts::load_contract(path.to_str().unwrap()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if contract.tests.is_empty() || contract.tests.values().all(|entries| entries.is_empty()) {
            continue;
        }
        let mut test_types: Vec<_> = contract.tests.keys().cloned().collect();
        test_types.sort();
        let mut has_failure = false;
        for test_type in &test_types {
            let entries = &contract.tests[test_type];
            for entry in entries {
                let result = match adapters::invoke(repo_root, &cfg, test_type, entry) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                if result.timed_out || result.exit_code != Some(0) {
                    has_failure = true;
                }
            }
        }
        if has_failure {
            let archetype = if contract.archetype.is_empty() {
                String::new()
            } else {
                contract.archetype.clone()
            };
            *failures.entry((spec.clone(), archetype)).or_insert(0) += 1;
        }
    }

    // Reclassify: move failing contracts from covered to failing
    for ((spec, archetype), n) in &failures {
        let key = (spec.clone(), archetype.clone());
        if let Some(row) = rows.get_mut(&key) {
            row.covered = row.covered.saturating_sub(*n);
            row.failing += n;
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
