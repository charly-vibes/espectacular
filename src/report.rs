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
    let file_ids = openspec::frontmatter_ids(specs_dir.to_str().unwrap()).unwrap_or_default();

    let contracts_map = build_contracts_map(&contract_files);
    let mut rows = classify_scenarios(&scenarios, &contracts_map, &file_ids);
    reclassify_failures(
        &mut rows,
        count_contract_failures(repo_root, &cfg, &contract_files),
    );

    Ok(aggregate(rows))
}

/// Lookup of spec+contract-id -> (archetype, path).
fn build_contracts_map(
    contract_files: &[(String, String, std::path::PathBuf)],
) -> BTreeMap<(String, String), (String, std::path::PathBuf)> {
    let mut contracts_map: BTreeMap<(String, String), (String, std::path::PathBuf)> =
        BTreeMap::new();
    for (spec, id, path) in contract_files {
        let archetype = contracts::load_contract(path.to_str().unwrap())
            .map(|c| c.archetype)
            .unwrap_or_default();
        contracts_map.insert((spec.clone(), id.clone()), (archetype, path.clone()));
    }
    contracts_map
}

/// Group scenarios by (spec, archetype) — attribute each scenario to its
/// contract's resolved archetype so the archetype row owns real counts.
/// GH#34 (espectacular-84k): attribute through the VERIFIES redirect the same
/// way ah check does — a scenario whose body carries
/// `- **VERIFIES** [[spec.P-x]]` bullets is covered by the derived contract
/// (first link whose contract file exists wins), not missing. Direct key
/// lookup stays the fast path and the fallback.
fn resolve_contract<'map>(
    scenario: &openspec::Scenario,
    contracts_map: &'map BTreeMap<(String, String), (String, std::path::PathBuf)>,
    file_ids: &BTreeMap<String, String>,
) -> Option<&'map (String, std::path::PathBuf)> {
    let key = (scenario.spec_path.clone(), scenario.id.clone());
    if let Some(found) = contracts_map.get(&key) {
        return Some(found);
    }
    let file_id = file_ids.get(&scenario.spec_path).map(String::as_str);
    check::verifies_property_ids(&scenario.body, file_id)
        .iter()
        .map(|property_id| {
            (
                scenario.spec_path.clone(),
                crate::sync::slugify_property_id(property_id),
            )
        })
        .find_map(|derived_key| contracts_map.get(&derived_key))
}

fn classify_scenarios(
    scenarios: &[openspec::Scenario],
    contracts_map: &BTreeMap<(String, String), (String, std::path::PathBuf)>,
    file_ids: &BTreeMap<String, String>,
) -> BTreeMap<(String, String), MatrixRow> {
    let mut rows: BTreeMap<(String, String), MatrixRow> = BTreeMap::new();
    for scenario in scenarios {
        let contract = resolve_contract(scenario, contracts_map, file_ids);
        let archetype = contract.map(|(a, _)| a.clone()).unwrap_or_default();
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
        if contract.is_some() {
            entry.covered += 1;
        } else {
            entry.missing += 1;
        }
    }
    rows
}

/// Run declared tests and count failing contracts per (spec, archetype).
fn count_contract_failures(
    repo_root: &Path,
    cfg: &crate::config::Config,
    contract_files: &[(String, String, std::path::PathBuf)],
) -> BTreeMap<(String, String), usize> {
    let mut failures: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (spec, _scenario_id, path) in contract_files {
        let contract = match contracts::load_contract(path.to_str().unwrap()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if contract.tests.is_empty() || contract.tests.values().all(|entries| entries.is_empty()) {
            continue;
        }
        if contract_has_failure(repo_root, cfg, &contract) {
            let archetype = if contract.archetype.is_empty() {
                String::new()
            } else {
                contract.archetype.clone()
            };
            *failures.entry((spec.clone(), archetype)).or_insert(0) += 1;
        }
    }
    failures
}

fn contract_has_failure(
    repo_root: &Path,
    cfg: &crate::config::Config,
    contract: &contracts::Contract,
) -> bool {
    let mut test_types: Vec<_> = contract.tests.keys().cloned().collect();
    test_types.sort();
    for test_type in &test_types {
        for entry in &contract.tests[test_type] {
            let result = match adapters::invoke(repo_root, cfg, test_type, entry) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if result.timed_out || result.exit_code != Some(0) {
                return true;
            }
        }
    }
    false
}

/// Reclassify: move failing contracts from covered to failing.
fn reclassify_failures(
    rows: &mut BTreeMap<(String, String), MatrixRow>,
    failures: BTreeMap<(String, String), usize>,
) {
    for ((spec, archetype), n) in &failures {
        let key = (spec.clone(), archetype.clone());
        if let Some(row) = rows.get_mut(&key) {
            row.covered = row.covered.saturating_sub(*n);
            row.failing += n;
        }
    }
}

fn aggregate(rows: BTreeMap<(String, String), MatrixRow>) -> ReportOutput {
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

    ReportOutput {
        matrix,
        summary: ReportSummary {
            total_scenarios,
            total_contracts,
            covered: total_covered,
            missing: total_missing,
            failing: total_failing,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Tempdir corpus: dual-format spec with a VERIFIES-linked scenario and
    /// an unlinked one, plus the derived contract stub for `p-thing`.
    fn verifies_report_repo() -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        fs::create_dir_all(repo.path().join(".espectacular")).unwrap();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
        )
        .unwrap();
        let specs = repo.path().join("openspec/specs/minspec");
        fs::create_dir_all(&specs).unwrap();
        fs::write(
            specs.join("spec.md"),
            "---\nid: minspec\nkind: intent\n---\n\n## Requirements\n\n#### Scenario: derived coverage\n\n- **GIVEN** something\n- **WHEN** it happens\n- **THEN** it SHALL hold\n- **VERIFIES** [[minspec.P-thing]]\n\n#### Scenario: unlinked coverage\n\n- **GIVEN** something else\n- **WHEN** it happens\n- **THEN** it SHALL hold\n",
        )
        .unwrap();
        let contracts_dir = repo.path().join(".espectacular/minspec");
        fs::create_dir_all(&contracts_dir).unwrap();
        fs::write(
            contracts_dir.join("p-thing.toml"),
            "id = \"p-thing\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.0.0\"\n",
        )
        .unwrap();
        repo
    }

    // GH#34 (espectacular-84k): a scenario reaching its contract through a
    // VERIFIES bullet must count covered, exactly like ah check attributes
    // it — not missing.
    #[test]
    fn report_redirects_verifies_scenarios_at_derived_contract() {
        let repo = verifies_report_repo();
        let output = run_report(repo.path()).unwrap();

        assert_eq!(output.summary.total_scenarios, 2);
        assert_eq!(
            output.summary.covered, 1,
            "VERIFIES-linked scenario counts covered; got {output:?}"
        );
        assert_eq!(output.summary.missing, 1, "unlinked scenario stays missing");
        let covered_row = output
            .matrix
            .iter()
            .find(|r| r.covered > 0)
            .expect("one row owns the covered scenario");
        assert_eq!(
            covered_row.archetype, "PF",
            "attribution flows from the derived contract's archetype"
        );
    }
}
