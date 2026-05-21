use crate::adapters;
use crate::config;
use crate::contracts;
use crate::openspec::{self, Scenario};
use crate::runner::TestResult;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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
    pub counts_by_kind: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct ReportFinding {
    pub kind: String,
    pub category: String,
    pub spec: String,
    pub spec_path: String,
    pub scenario: ScenarioContext,
    pub suggested_action: String,
    pub playbook_command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_prose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<TestResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct ScenarioContext {
    pub id: String,
    pub title: String,
    pub body_markdown: String,
}

#[derive(Debug, Clone)]
struct ResolvedScenario {
    scenario: Scenario,
    contract_path: PathBuf,
}

#[derive(Debug)]
struct ResolvedScope {
    scenarios: Vec<ResolvedScenario>,
    contract_files: Vec<(String, String, PathBuf)>,
    changes: Vec<String>,
    findings: Vec<ReportFinding>,
}

pub fn run_check(repo_root: &Path, selected_changes: &[String]) -> anyhow::Result<CheckOutput> {
    let config_path = repo_root.join(".espectacular/config.toml");
    let cfg = config::load_config(config_path.to_str().unwrap())?;
    let specs_dir = repo_root.join(&cfg.paths.specs);
    let contracts_dir = repo_root.join(".espectacular");
    let changes_dir = repo_root.join(&cfg.paths.changes);

    let scope = resolve_scope(&specs_dir, &contracts_dir, &changes_dir, selected_changes)?;
    evaluate_scope(repo_root, &cfg, &specs_dir, scope)
}

pub fn structural_findings(specs_dir: &str, contracts_dir: &str) -> anyhow::Result<Vec<Finding>> {
    let scenarios = openspec::discover_scenarios(specs_dir)?;
    let resolved: Vec<_> = scenarios
        .into_iter()
        .map(|scenario| ResolvedScenario {
            contract_path: contract_path(
                Path::new(contracts_dir),
                &scenario.spec_path,
                &scenario.id,
            ),
            scenario,
        })
        .collect();
    let contract_files = collect_base_contract_files(Path::new(contracts_dir));
    Ok(collect_structural_findings(&resolved, &contract_files)
        .into_iter()
        .map(|finding| Finding::structural(&finding.spec, &finding.scenario.id, &finding.kind))
        .collect())
}

fn resolve_scope(
    specs_dir: &Path,
    contracts_dir: &Path,
    changes_dir: &Path,
    selected_changes: &[String],
) -> anyhow::Result<ResolvedScope> {
    let base_scenarios = openspec::discover_scenarios(specs_dir.to_str().unwrap())?;
    let mut scenarios: BTreeMap<(String, String), ResolvedScenario> = base_scenarios
        .into_iter()
        .map(|scenario| {
            let key = (scenario.spec_path.clone(), scenario.id.clone());
            let contract_path = contract_path(contracts_dir, &scenario.spec_path, &scenario.id);
            (
                key,
                ResolvedScenario {
                    scenario,
                    contract_path,
                },
            )
        })
        .collect();
    let mut contract_overrides: HashMap<(String, String), PathBuf> = HashMap::new();
    let mut contract_files = collect_base_contract_files(contracts_dir);
    let mut findings = Vec::new();

    let mut changes = selected_changes.to_vec();
    changes.sort();
    changes.dedup();

    for change in &changes {
        let change_specs = changes_dir.join(change).join("specs");
        if !change_specs.exists() {
            anyhow::bail!(
                "change '{change}' does not exist at {}",
                change_specs.display()
            );
        }

        let added_scenarios = openspec::discover_scenarios(change_specs.to_str().unwrap())?;
        for scenario in added_scenarios {
            let key = (scenario.spec_path.clone(), scenario.id.clone());
            if scenarios.contains_key(&key) {
                findings.push(report_finding(
                    "overlay-conflict",
                    "structural",
                    scenario.spec_path.clone(),
                    change_specs
                        .join(&scenario.spec_path)
                        .join("spec.md")
                        .to_string_lossy()
                        .into_owned(),
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
                ));
                continue;
            }
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

        let staged_root = contracts_dir.join("changes").join(change);
        for (spec, id, path) in collect_contract_files(&staged_root) {
            let key = (spec.clone(), id.clone());
            if let Some(previous) = contract_overrides.insert(key.clone(), path.clone()) {
                findings.push(report_finding(
                    "overlay-conflict",
                    "structural",
                    spec.clone(),
                    spec_markdown_path(specs_dir, &spec),
                    ScenarioContext {
                        id: id.clone(),
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
                ));
                continue;
            }
            contract_files.push((spec.clone(), id.clone(), path.clone()));
            if let Some(existing) = scenarios.get_mut(&key) {
                existing.contract_path = path;
            }
        }
    }

    Ok(ResolvedScope {
        scenarios: scenarios.into_values().collect(),
        contract_files,
        changes,
        findings,
    })
}

fn evaluate_scope(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scope: ResolvedScope,
) -> anyhow::Result<CheckOutput> {
    let mut findings = scope.findings;
    findings.extend(collect_structural_findings(
        &scope.scenarios,
        &scope.contract_files,
    ));

    let blocked = blocked_scenarios(&findings);
    let mut passed = 0usize;

    for resolved in sorted_resolved_scenarios(&scope.scenarios) {
        let scenario = &resolved.scenario;
        if blocked.contains(&(scenario.spec_path.clone(), scenario.id.clone())) {
            continue;
        }

        let contract = match contracts::load_contract(resolved.contract_path.to_str().unwrap()) {
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
                let result = match adapters::invoke(repo_root, cfg, &test_type, entry) {
                    Ok(result) => result,
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

    let mut counts_by_kind = BTreeMap::new();
    for finding in &findings {
        *counts_by_kind.entry(finding.kind.clone()).or_insert(0) += 1;
    }

    Ok(CheckOutput {
        scope: Scope {
            deployed: true,
            changes: scope.changes,
        },
        summary: Summary {
            structural,
            execution,
            passed,
            counts_by_kind,
        },
        findings,
    })
}

fn collect_structural_findings(
    scenarios: &[ResolvedScenario],
    contract_files: &[(String, String, PathBuf)],
) -> Vec<ReportFinding> {
    let mut findings = Vec::new();
    let bare_scenarios: Vec<_> = scenarios
        .iter()
        .map(|resolved| resolved.scenario.clone())
        .collect();
    let scenario_map: BTreeMap<(String, String), &ResolvedScenario> = scenarios
        .iter()
        .map(|resolved| {
            (
                (
                    resolved.scenario.spec_path.clone(),
                    resolved.scenario.id.clone(),
                ),
                resolved,
            )
        })
        .collect();

    for (spec, id, _) in openspec::detect_slug_collisions(&bare_scenarios) {
        if let Some(resolved) = scenario_map.get(&(spec.clone(), id.clone())) {
            findings.push(structural_report(
                &resolved.scenario,
                spec_path_root(&resolved.scenario),
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

    let known_ids: HashSet<(String, String)> = scenarios
        .iter()
        .map(|resolved| {
            (
                resolved.scenario.spec_path.clone(),
                resolved.scenario.id.clone(),
            )
        })
        .collect();

    let mut seen = HashSet::new();
    for resolved in sorted_resolved_scenarios(scenarios) {
        let scenario = &resolved.scenario;
        let key = (scenario.spec_path.clone(), scenario.id.clone());
        if collision_ids.contains(&key) || !seen.insert(key.clone()) {
            continue;
        }

        if !resolved.contract_path.exists() {
            findings.push(structural_report(
                scenario,
                spec_path_root(scenario),
                "no-toml",
                None,
            ));
            continue;
        }

        match contracts::load_contract(resolved.contract_path.to_str().unwrap()) {
            Ok(contract) => {
                if contract.id != scenario.id {
                    findings.push(structural_report(
                        scenario,
                        spec_path_root(scenario),
                        "id-mismatch",
                        None,
                    ));
                }
                if contract.tests.is_empty()
                    || contract.tests.values().all(|entries| entries.is_empty())
                {
                    findings.push(structural_report(
                        scenario,
                        spec_path_root(scenario),
                        "no-tests-declared",
                        None,
                    ));
                }
                if contract.status == "superseded"
                    && !known_ids
                        .contains(&(scenario.spec_path.clone(), contract.superseded_by.clone()))
                {
                    findings.push(structural_report(
                        scenario,
                        spec_path_root(scenario),
                        "missing-replacement",
                        Some(format!(
                            "replacement scenario '{}' is absent from scope",
                            contract.superseded_by
                        )),
                    ));
                }
            }
            Err(error) => {
                findings.push(structural_report(
                    scenario,
                    spec_path_root(scenario),
                    "malformed-contract",
                    Some(error.to_string()),
                ));
            }
        }
    }

    findings.extend(orphan_reports(contract_files, &scenario_map));
    findings.sort_by(report_finding_cmp);
    findings
}

fn orphan_reports(
    contract_files: &[(String, String, PathBuf)],
    scenarios: &BTreeMap<(String, String), &ResolvedScenario>,
) -> Vec<ReportFinding> {
    let mut findings = Vec::new();
    let mut seen = HashSet::new();
    for (spec, id, path) in contract_files {
        if !seen.insert((spec.clone(), id.clone(), path.clone())) {
            continue;
        }
        if scenarios.contains_key(&(spec.clone(), id.clone())) {
            continue;
        }
        findings.push(report_finding(
            "orphan-toml",
            "structural",
            spec.clone(),
            path.parent()
                .map(|_| format!("openspec/specs/{spec}/spec.md"))
                .unwrap_or_else(|| format!("openspec/specs/{spec}/spec.md")),
            ScenarioContext {
                id: id.clone(),
                title: String::new(),
                body_markdown: String::new(),
            },
            None,
            None,
            None,
        ));
    }
    findings
}

fn collect_base_contract_files(contracts_root: &Path) -> Vec<(String, String, PathBuf)> {
    collect_contract_files(contracts_root)
        .into_iter()
        .filter(|(spec, _, _)| spec != "changes")
        .collect()
}

fn collect_contract_files(root: &Path) -> Vec<(String, String, PathBuf)> {
    let mut files = Vec::new();
    let Ok(spec_dirs) = fs::read_dir(root) else {
        return files;
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
            files.push((spec.clone(), id, path));
        }
    }
    files
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
    report_finding(
        kind,
        "structural",
        scenario.spec_path.clone(),
        spec_markdown_path(specs_root, &scenario.spec_path),
        ScenarioContext {
            id: scenario.id.clone(),
            title: scenario.heading.clone(),
            body_markdown: scenario.body.clone(),
        },
        Some(scenario.body.clone()),
        None,
        message,
    )
}

fn execution_report(scenario: &Scenario, specs_root: &Path, test: TestResult) -> ReportFinding {
    report_finding(
        "test-failing",
        "execution",
        scenario.spec_path.clone(),
        spec_markdown_path(specs_root, &scenario.spec_path),
        ScenarioContext {
            id: scenario.id.clone(),
            title: scenario.heading.clone(),
            body_markdown: scenario.body.clone(),
        },
        Some(scenario.body.clone()),
        Some(test),
        None,
    )
}

fn report_finding(
    kind: &str,
    category: &str,
    spec: String,
    spec_path: String,
    scenario: ScenarioContext,
    scenario_prose: Option<String>,
    test: Option<TestResult>,
    message: Option<String>,
) -> ReportFinding {
    let suggested_action = suggested_action_for(kind).to_string();
    ReportFinding {
        kind: kind.to_string(),
        category: category.to_string(),
        spec,
        spec_path,
        scenario,
        suggested_action: suggested_action.clone(),
        playbook_command: format!("ah explain {suggested_action}"),
        scenario_prose,
        test,
        message,
    }
}

fn suggested_action_for(kind: &str) -> &'static str {
    match kind {
        "no-toml"
        | "orphan-toml"
        | "slug-collision"
        | "id-mismatch"
        | "invalid-status"
        | "no-tests-declared"
        | "malformed-contract"
        | "missing-replacement"
        | "overlay-conflict" => "review_and_apply",
        "missing-runner" | "test-failing" => "edit_code_not_scenario",
        _ => "human_review_required",
    }
}

fn sorted_resolved_scenarios(scenarios: &[ResolvedScenario]) -> Vec<&ResolvedScenario> {
    let mut sorted: Vec<_> = scenarios.iter().collect();
    sorted.sort_by(|left, right| {
        (&left.scenario.spec_path, &left.scenario.id)
            .cmp(&(&right.scenario.spec_path, &right.scenario.id))
    });
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

fn spec_path_root(scenario: &Scenario) -> &Path {
    if scenario.spec_path.contains('/') {
        Path::new("")
    } else {
        Path::new("openspec/specs")
    }
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
        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn findings_ordered_by_spec_scenario_kind() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        let mut sorted = findings.clone();
        sorted.sort();
        assert_eq!(findings, sorted);
    }

    #[test]
    fn missing_contract_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.kind == "no-toml" && f.scenario_id == "missing-contract"));
    }

    #[test]
    fn orphan_contract_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.kind == "orphan-toml" && f.scenario_id == "orphan-contract"));
    }

    #[test]
    fn no_tests_declared_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.kind == "no-tests-declared" && f.scenario_id == "no-tests-declared"));
    }

    #[test]
    fn duplicate_id_finding_present() {
        let findings = structural_findings(SPECS, CONTRACTS).unwrap();
        assert!(findings.iter().any(|f| f.kind == "slug-collision"));
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
        let output = run_check(dir.path(), &[]).unwrap();
        assert!(output.findings.is_empty());
        assert_eq!(output.summary.passed, 1);
        assert!(output.summary.counts_by_kind.is_empty());
    }

    #[test]
    fn run_check_with_change_adds_scenarios_to_scope() {
        let dir = success_repo();
        fs::create_dir_all(
            dir.path()
                .join("openspec/changes/add-parser/specs/compiler"),
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".espectacular/changes/add-parser/compiler")).unwrap();
        fs::write(
            dir.path().join("openspec/changes/add-parser/specs/compiler/spec.md"),
            "# Capability: compiler\n\n#### Scenario: Added path\n- **WHEN** change applies\n- **THEN** it passes\n",
        ).unwrap();
        fs::write(
            dir.path().join(".espectacular/changes/add-parser/compiler/added-path.toml"),
            "id = \"added-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
        ).unwrap();

        let output = run_check(dir.path(), &["add-parser".to_string()]).unwrap();
        assert_eq!(output.scope.changes, vec!["add-parser"]);
        assert_eq!(output.summary.passed, 2);
    }

    #[test]
    fn run_check_reports_overlay_conflict_for_duplicate_added_scenarios() {
        let dir = success_repo();
        for change in ["zeta", "alpha"] {
            fs::create_dir_all(
                dir.path()
                    .join(format!("openspec/changes/{change}/specs/compiler")),
            )
            .unwrap();
            fs::create_dir_all(
                dir.path()
                    .join(format!(".espectacular/changes/{change}/compiler")),
            )
            .unwrap();
            fs::write(
                dir.path().join(format!("openspec/changes/{change}/specs/compiler/spec.md")),
                "# Capability: compiler\n\n#### Scenario: Added path\n- **WHEN** change applies\n- **THEN** it passes\n",
            ).unwrap();
        }

        let output = run_check(dir.path(), &["zeta".to_string(), "alpha".to_string()]).unwrap();
        assert_eq!(output.scope.changes, vec!["alpha", "zeta"]);
        assert!(output.findings.iter().any(|f| f.kind == "overlay-conflict"));
        assert_eq!(
            output.summary.counts_by_kind.get("overlay-conflict"),
            Some(&1)
        );
    }

    #[test]
    fn findings_carry_action_fields_and_scenario_prose() {
        let dir = success_repo();
        write_executable(&dir.path().join("runner.sh"), "printf 'boom' >&2\nexit 7");

        let output = run_check(dir.path(), &[]).unwrap();
        let finding = output
            .findings
            .iter()
            .find(|f| f.kind == "test-failing")
            .unwrap();

        assert_eq!(finding.suggested_action, "edit_code_not_scenario");
        assert_eq!(
            finding.playbook_command,
            "ah explain edit_code_not_scenario"
        );
        assert_eq!(
            finding.scenario_prose.as_deref(),
            Some("- **WHEN** it runs\n- **THEN** it passes")
        );
    }

    #[test]
    fn staged_superseded_contract_requires_replacement_in_scope() {
        let dir = success_repo();
        fs::create_dir_all(dir.path().join(".espectacular/changes/add-parser/compiler")).unwrap();
        fs::write(
            dir.path().join(".espectacular/changes/add-parser/compiler/green-path.toml"),
            "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"superseded\"\nsuperseded_by = \"missing\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
        ).unwrap();
        fs::create_dir_all(dir.path().join("openspec/changes/add-parser/specs")).unwrap();

        let output = run_check(dir.path(), &["add-parser".to_string()]).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "missing-replacement"));
    }
}
