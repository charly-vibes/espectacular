use crate::config;
use crate::contracts;
use crate::openspec::{self, Scenario};
use crate::runner::{self, TestResult};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CheckOutput {
    pub scope: Scope,
    pub summary: Summary,
    pub findings: Vec<ReportFinding>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Scope {
    pub deployed: bool,
    pub changes: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub structural: usize,
    pub execution: usize,
    pub passed: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ReportFinding {
    pub kind: String,
    pub category: String,
    pub spec: String,
    pub spec_path: String,
    pub scenario: ScenarioContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<TestResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ScenarioContext {
    pub id: String,
    pub title: String,
    pub body_markdown: String,
}

pub fn run_check(repo_root: &Path) -> anyhow::Result<CheckOutput> {
    let config_path = repo_root.join(".espectacular/config.toml");
    let cfg = config::load_config(config_path.to_str().unwrap())?;
    let specs_dir = repo_root.join(&cfg.paths.specs);
    let contracts_dir = repo_root.join(".espectacular");

    let scenarios = openspec::discover_scenarios(specs_dir.to_str().unwrap())?;
    let report = evaluate_deployed(repo_root, &cfg, &scenarios, &specs_dir, &contracts_dir)?;
    Ok(report)
}

pub fn structural_findings(specs_dir: &str, contracts_dir: &str) -> anyhow::Result<Vec<Finding>> {
    let scenarios = openspec::discover_scenarios(specs_dir)?;
    let specs_root = Path::new(specs_dir);
    let contracts_root = Path::new(contracts_dir);
    Ok(
        collect_structural_findings(&scenarios, specs_root, contracts_root)
            .into_iter()
            .map(|finding| Finding::structural(&finding.spec, &finding.scenario.id, &finding.kind))
            .collect(),
    )
}

fn evaluate_deployed(
    repo_root: &Path,
    cfg: &config::Config,
    scenarios: &[Scenario],
    specs_root: &Path,
    contracts_root: &Path,
) -> anyhow::Result<CheckOutput> {
    let mut findings = collect_structural_findings(scenarios, specs_root, contracts_root);
    let blocked = blocked_scenarios(&findings);
    let mut passed = 0usize;

    for scenario in sorted_scenarios(scenarios) {
        if blocked.contains(&(scenario.spec_path.clone(), scenario.id.clone())) {
            continue;
        }

        let contract_path = contract_path(contracts_root, &scenario.spec_path, &scenario.id);
        let contract = match contracts::load_contract(contract_path.to_str().unwrap()) {
            Ok(contract) => contract,
            Err(_) => continue,
        };

        if contract.tests.is_empty() || contract.tests.values().all(|entries| entries.is_empty()) {
            continue;
        }

        let mut test_types: Vec<_> = contract.tests.keys().cloned().collect();
        test_types.sort();
        for test_type in test_types {
            let entries = &contract.tests[&test_type];
            for entry in entries {
                let planned = match runner::compose_command(cfg, &test_type, entry) {
                    Ok(planned) => planned,
                    Err(error) => {
                        findings.push(structural_report(
                            scenario,
                            specs_root,
                            "missing-runner",
                            Some(error.to_string()),
                        ));
                        continue;
                    }
                };

                let result = runner::execute_command(repo_root, &planned)?;
                if result.timed_out || result.exit_code != Some(0) {
                    findings.push(execution_report(scenario, specs_root, result));
                } else {
                    passed += 1;
                }
            }
        }
    }

    findings.sort_by(report_finding_cmp);

    let structural = findings
        .iter()
        .filter(|finding| finding.category == "structural")
        .count();
    let execution = findings
        .iter()
        .filter(|finding| finding.category == "execution")
        .count();

    Ok(CheckOutput {
        scope: Scope {
            deployed: true,
            changes: Vec::new(),
        },
        summary: Summary {
            structural,
            execution,
            passed,
        },
        findings,
    })
}

fn collect_structural_findings(
    scenarios: &[Scenario],
    specs_root: &Path,
    contracts_root: &Path,
) -> Vec<ReportFinding> {
    let mut findings = Vec::new();
    let scenario_map: BTreeMap<(String, String), &Scenario> = scenarios
        .iter()
        .map(|scenario| ((scenario.spec_path.clone(), scenario.id.clone()), scenario))
        .collect();

    for (spec, id, _) in openspec::detect_slug_collisions(scenarios) {
        if let Some(scenario) = scenario_map.get(&(spec.clone(), id.clone())) {
            findings.push(structural_report(
                scenario,
                specs_root,
                "slug-collision",
                None,
            ));
        }
    }

    let collision_ids: HashSet<(String, String)> = findings
        .iter()
        .filter(|finding| finding.kind == "slug-collision")
        .map(|finding| (finding.spec.clone(), finding.scenario.id.clone()))
        .collect();

    let mut seen = HashSet::new();
    for scenario in sorted_scenarios(scenarios) {
        let key = (scenario.spec_path.clone(), scenario.id.clone());
        if collision_ids.contains(&key) || !seen.insert(key.clone()) {
            continue;
        }

        let path = contract_path(contracts_root, &scenario.spec_path, &scenario.id);
        if !path.exists() {
            findings.push(structural_report(scenario, specs_root, "no-toml", None));
            continue;
        }

        match contracts::load_contract(path.to_str().unwrap()) {
            Ok(contract) => {
                if contract.id != scenario.id {
                    findings.push(structural_report(scenario, specs_root, "id-mismatch", None));
                }
                if contract.tests.is_empty()
                    || contract.tests.values().all(|entries| entries.is_empty())
                {
                    findings.push(structural_report(
                        scenario,
                        specs_root,
                        "no-tests-declared",
                        None,
                    ));
                }
            }
            Err(error) => {
                findings.push(structural_report(
                    scenario,
                    specs_root,
                    "malformed-contract",
                    Some(error.to_string()),
                ));
            }
        }
    }

    findings.extend(orphan_reports(contracts_root, specs_root, &scenario_map));
    findings.sort_by(report_finding_cmp);
    findings
}

fn orphan_reports(
    contracts_root: &Path,
    specs_root: &Path,
    scenarios: &BTreeMap<(String, String), &Scenario>,
) -> Vec<ReportFinding> {
    let mut findings = Vec::new();
    let Ok(spec_dirs) = fs::read_dir(contracts_root) else {
        return findings;
    };

    for spec_dir in spec_dirs.flatten() {
        if !spec_dir
            .file_type()
            .map(|kind| kind.is_dir())
            .unwrap_or(false)
        {
            continue;
        }
        let spec = spec_dir.file_name().to_string_lossy().into_owned();
        if spec == "changes" {
            continue;
        }
        let Ok(contract_files) = fs::read_dir(spec_dir.path()) else {
            continue;
        };
        for entry in contract_files.flatten() {
            if !entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
            {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("toml") {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            if scenarios.contains_key(&(spec.clone(), id.clone())) {
                continue;
            }
            findings.push(ReportFinding {
                kind: "orphan-toml".to_string(),
                category: "structural".to_string(),
                spec: spec.clone(),
                spec_path: spec_markdown_path(specs_root, &spec),
                scenario: ScenarioContext {
                    id,
                    title: String::new(),
                    body_markdown: String::new(),
                },
                test: None,
                message: None,
            });
        }
    }

    findings
}

fn blocked_scenarios(findings: &[ReportFinding]) -> BTreeSet<(String, String)> {
    findings
        .iter()
        .filter(|finding| finding.category == "structural")
        .map(|finding| (finding.spec.clone(), finding.scenario.id.clone()))
        .collect()
}

fn structural_report(
    scenario: &Scenario,
    specs_root: &Path,
    kind: &str,
    message: Option<String>,
) -> ReportFinding {
    ReportFinding {
        kind: kind.to_string(),
        category: "structural".to_string(),
        spec: scenario.spec_path.clone(),
        spec_path: spec_markdown_path(specs_root, &scenario.spec_path),
        scenario: ScenarioContext {
            id: scenario.id.clone(),
            title: scenario.heading.clone(),
            body_markdown: scenario.body.clone(),
        },
        test: None,
        message,
    }
}

fn execution_report(scenario: &Scenario, specs_root: &Path, test: TestResult) -> ReportFinding {
    ReportFinding {
        kind: "test-failing".to_string(),
        category: "execution".to_string(),
        spec: scenario.spec_path.clone(),
        spec_path: spec_markdown_path(specs_root, &scenario.spec_path),
        scenario: ScenarioContext {
            id: scenario.id.clone(),
            title: scenario.heading.clone(),
            body_markdown: scenario.body.clone(),
        },
        test: Some(test),
        message: None,
    }
}

fn sorted_scenarios(scenarios: &[Scenario]) -> Vec<&Scenario> {
    let mut sorted: Vec<_> = scenarios.iter().collect();
    sorted.sort_by(|left, right| (&left.spec_path, &left.id).cmp(&(&right.spec_path, &right.id)));
    sorted
}

fn report_finding_cmp(left: &ReportFinding, right: &ReportFinding) -> std::cmp::Ordering {
    (
        &left.spec_path,
        &left.scenario.id,
        &left.kind,
        left.test
            .as_ref()
            .map(|test| (&test.test_type, &test.command)),
    )
        .cmp(&(
            &right.spec_path,
            &right.scenario.id,
            &right.kind,
            right
                .test
                .as_ref()
                .map(|test| (&test.test_type, &test.command)),
        ))
}

fn contract_path(contracts_root: &Path, spec: &str, id: &str) -> PathBuf {
    contracts_root.join(spec).join(format!("{id}.toml"))
}

fn spec_markdown_path(specs_root: &Path, spec: &str) -> String {
    specs_root
        .join(spec)
        .join("spec.md")
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

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
        assert_eq!(
            findings, sorted,
            "findings must be deterministically ordered"
        );
    }

    #[test]
    fn missing_contract_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            findings
                .iter()
                .any(|f| f.kind == "no-toml" && f.scenario_id == "missing-contract"),
            "expected no-toml finding"
        );
    }

    #[test]
    fn orphan_contract_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            findings
                .iter()
                .any(|f| f.kind == "orphan-toml" && f.scenario_id == "orphan-contract"),
            "expected orphan-toml finding"
        );
    }

    #[test]
    fn no_tests_declared_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            findings
                .iter()
                .any(|f| f.kind == "no-tests-declared" && f.scenario_id == "no-tests-declared"),
            "expected no-tests-declared finding"
        );
    }

    #[test]
    fn duplicate_id_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(
            findings.iter().any(|f| f.kind == "slug-collision"),
            "expected slug-collision finding"
        );
    }

    #[test]
    fn all_findings_have_structural_category() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        for f in &findings {
            assert_eq!(
                f.category, "structural",
                "finding {:?} must be structural",
                f.kind
            );
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

    fn write_executable(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    fn success_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
        fs::write(
            repo.join("openspec/specs/compiler/spec.md"),
            "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n",
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n",
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/compiler/green-path.toml"),
            "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
        )
        .unwrap();
        write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");
        dir
    }

    #[test]
    fn run_check_reports_success_with_empty_findings() {
        let dir = success_repo();
        let output = run_check(dir.path()).unwrap();

        assert!(output.findings.is_empty());
        assert_eq!(output.summary.passed, 1);
        assert_eq!(
            output.scope,
            Scope {
                deployed: true,
                changes: vec![]
            }
        );
    }

    #[test]
    fn run_check_maps_non_zero_exit_to_execution_finding() {
        let dir = success_repo();
        write_executable(&dir.path().join("runner.sh"), "printf 'oops' >&2\nexit 5");

        let output = run_check(dir.path()).unwrap();
        let finding = output
            .findings
            .iter()
            .find(|finding| finding.kind == "test-failing")
            .unwrap();

        assert_eq!(output.summary.execution, 1);
        assert_eq!(finding.category, "execution");
        assert_eq!(finding.test.as_ref().unwrap().exit_code, Some(5));
        assert_eq!(finding.test.as_ref().unwrap().stderr_tail, "oops");
    }

    #[test]
    fn run_check_emits_missing_runner_structural_finding() {
        let dir = success_repo();
        fs::write(
            dir.path().join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npbt = [\"echo\"]\n",
        )
        .unwrap();

        let output = run_check(dir.path()).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|finding| finding.kind == "missing-runner"));
    }
}
