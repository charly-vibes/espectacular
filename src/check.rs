use crate::adapters;
use crate::config;
use crate::contracts;
use crate::openspec::{self, Scenario};
use crate::quality;
use crate::runner::{self, TestResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub spec_path: String,
    pub scenario_id: String,
    pub kind: String,
    pub category: String,
    pub source_location: Option<String>,
}

impl Finding {
    #[allow(dead_code)]
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub quality_findings: Vec<quality::QualityFinding>,
    /// Test-selection report (espectacular-0k8 Layer 1): present only when
    /// a changed-file set narrowed the executed contract tests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<Selection>,
}

/// Layer 1 selection (espectacular-0k8): capability-granular, spec-provenance
/// selection. `selected` lists capability names whose contract tests ran;
/// `skipped` counts scenarios skipped by selection.
/// Layer 1 selection state (espectacular-0k8): which capability contracts
/// the run narrows to, or a conservative run-all whose bypass is reported.
enum SelectionState {
    /// No selection: run everything, report nothing (no changed files).
    None,
    /// Conservative run-all with reported provenance: the --all-tests
    /// escape hatch (review F3) or a zero-overlap/unmapped bypass
    /// (review F1/F9). `selected` is the caps that ran (all-tests) or
    /// empty (bypassed — selection did not narrow anything).
    All {
        source: &'static str,
        unmapped_files: usize,
        selected: Vec<String>,
    },
    /// Selected capability contracts only (review F0).
    Caps(std::collections::HashSet<String>),
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Selection {
    pub selected: Vec<String>,
    pub skipped: usize,
    pub source: String,
    pub unmapped_files: usize,
    /// Layer 2 refinement (espectacular-0k8): contract bindings pruned
    /// because testaruda's selection deemed their underlying tests
    /// unaffected. Present whenever a testaruda store was consulted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub testaruda: Option<TestarudaRefinement>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TestarudaRefinement {
    pub pruned: usize,
}

/// Layer 2 selection facts parsed from `testaruda select --agent`.
struct TestarudaSelection {
    /// Selected test node_ids (includes always_run fallbacks — testaruda's
    /// own conservative over-approximation flows through).
    selected: Vec<String>,
    /// EMPTY selection (exit 20): no code affected. Refinement is skipped,
    /// the Layer 1 set still runs (spec change must verify own contracts).
    empty: bool,
    /// A changed file testaruda could not resolve to a content unit —
    /// its dependency view is incomplete, so pruning is unsafe.
    any_unresolved: bool,
}

fn testaruda_store_present(repo_root: &Path) -> bool {
    repo_root.join(".testaruda/store.db").is_file()
}

/// Run `testaruda select --agent --files <changed>` in the repo. Returns
/// None when the store is absent, the binary is missing, or the output
/// cannot be parsed — every failure path is conservative (no pruning).
fn testaruda_select(repo_root: &Path, changed_files: &[String]) -> Option<TestarudaSelection> {
    use std::process::Stdio;
    let output = std::process::Command::new("testaruda")
        .args([
            "select",
            "--agent",
            "--files",
            &changed_files.join(","),
            "-q",
        ])
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let selected = value
        .get("selected")?
        .as_array()?
        .iter()
        .filter_map(|t| {
            t.get("node_id")
                .and_then(|n| n.as_str())
                .map(str::to_string)
        })
        .collect();
    let selected_count = value
        .get("summary")
        .and_then(|s| s.get("selected_count"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0);
    let any_unresolved = value
        .get("changed_units")
        .and_then(|u| u.as_array())
        .map(|units| {
            units.iter().any(|u| {
                u.get("unresolved")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    Some(TestarudaSelection {
        selected,
        empty: selected_count == 0,
        any_unresolved,
    })
}

/// The runner filter a contract binding injects, if it is prunable at all.
/// Shell commands are opaque (no filter into a test runner) and never
/// pruned; bindings without a non-empty flags filter are never pruned.
fn prunable_filter<'a>(test_type: &str, entry: &'a crate::contracts::TestEntry) -> Option<&'a str> {
    if test_type == "shell" {
        return None;
    }
    entry.flags.as_deref().filter(|f| !f.is_empty())
}

fn node_contains_filter(selected: &[String], filter: &str) -> bool {
    selected.iter().any(|node| node.contains(filter))
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ReportFinding {
    pub kind: String,
    pub category: String,
    pub spec: String,
    pub spec_path: String,
    pub scenario: ScenarioContext,
    pub suggested_action: String,
    pub playbook_command: String,
    /// Gate severity: "error" for structural/execution findings (gate-failing),
    /// "warning" for non-gating findings (exit zero on warnings only).
    /// External custom-runner findings that omit it default to "error".
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_prose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<TestResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn default_severity() -> String {
    "error".to_string()
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
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
    /// VERIFIES coverage (derive-contracts-from-specodelic task 4.1):
    /// (spec_path, scenario_id) → derived contract id. A scenario whose body
    /// carries `- **VERIFIES** [[<file-id>.P-x]]` is covered by the property's
    /// derived contract (`.espectacular/<spec>/<slug>.toml`) when it exists
    /// (design D1); the entry is absent when no derived contract backs the
    /// link, leaving the scenario's own contract requirement unchanged
    /// (C-unlinked-unchanged).
    covered: BTreeMap<(String, String), Vec<String>>,
}

/// Back-compat entry point (structural callers and tests); CLI uses
/// `run_check_with_selection` directly.
#[allow(dead_code)]
pub fn run_check(
    repo_root: &Path,
    selected_changes: &[String],
    run_tests: bool,
) -> anyhow::Result<CheckOutput> {
    run_check_with_selection(repo_root, selected_changes, run_tests, &[], false)
}

/// Like `run_check`, but with an explicit changed-file set (repo-relative).
/// When `changed_files` is non-empty and `all_tests` is false, only the
/// contract tests of capabilities whose spec or contract files changed run
/// (Layer 1 selection, espectacular-0k8). A changed set that maps to no
/// capability falls back conservatively to running everything.
#[allow(clippy::too_many_arguments)]
pub fn run_check_with_selection(
    repo_root: &Path,
    selected_changes: &[String],
    run_tests: bool,
    changed_files: &[String],
    all_tests: bool,
) -> anyhow::Result<CheckOutput> {
    let cfg = config::load(repo_root)?;
    let specs_dir = repo_root.join(&cfg.paths.specs);
    let contracts_dir = repo_root.join(".espectacular");
    let changes_dir = repo_root.join(&cfg.paths.changes);
    let ah_scope = std::env::var("AH_SCOPE").unwrap_or_default();

    let scope = resolve_scope(&specs_dir, &contracts_dir, &changes_dir, selected_changes)?;
    evaluate_scope(
        repo_root,
        &cfg,
        &specs_dir,
        scope,
        &ah_scope,
        run_tests,
        changed_files,
        all_tests,
    )
}

/// First non-empty path segment, mapped to a capability name.
fn first_segment(rest: &str) -> Option<String> {
    rest.split('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Map a changed file (repo-relative) to the capability it belongs to,
/// via either the deployed specs tree (`<specs>/<capability>/...`), the
/// change-overlay specs tree (`<changes>/<change>/specs/<capability>/...`,
/// review F4: overlay spec edits previously selected nothing), or the
/// contracts tree (`.espectacular/<capability>/...`, including change
/// overlays `.espectacular/changes/<change>/<capability>/...`).
/// Everything else (agent code, change proposal/tasks files) maps to no
/// capability.
fn affected_capability(file: &str, specs_dir: &str, changes_dir: &str) -> Option<String> {
    let specs_prefix = format!("{}/", specs_dir.trim_end_matches('/'));
    let changes_prefix = format!("{}/", changes_dir.trim_end_matches('/'));
    const CONTRACTS_PREFIX: &str = ".espectacular/";

    // deployed specs tree: <specs_dir>/<capability>/...
    if let Some(rest) = file.strip_prefix(specs_prefix.as_str()) {
        return first_segment(rest);
    }

    // change-overlay specs tree: <changes_dir>/<change>/specs/<cap>/...
    // Only files under specs/ select a capability; change proposal, tasks
    // and design files select nothing.
    if let Some(rest) = file.strip_prefix(changes_prefix.as_str()) {
        let mut segments = rest.split('/');
        let _change = segments.next();
        if segments.next() == Some("specs") {
            if let Some(cap) = segments.next() {
                if !cap.is_empty() {
                    return Some(cap.to_string());
                }
            }
        }
        return None;
    }

    // contracts tree: .espectacular/<capability>/...
    // staged contracts: .espectacular/changes/<change>/<capability>/...
    if let Some(rest) = file.strip_prefix(CONTRACTS_PREFIX) {
        if rest.starts_with("changes/") {
            let mut segments = rest.split('/');
            segments.next(); // "changes"
            segments.next(); // change name
            if let Some(cap) = segments.next() {
                if !cap.is_empty() {
                    return Some(cap.to_string());
                }
            }
            return None;
        }
        return first_segment(rest);
    }
    None
}

#[allow(dead_code)]
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
    Ok(collect_structural_findings(
        &resolved,
        &contract_files,
        Path::new(specs_dir),
        &BTreeMap::new(),
    )
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
    // GH#28.2 / espectacular-iq4: a repo with no deployed specs yet (changes
    // only) is a legitimate state — ah doctor classifies a missing specs dir
    // as a missing-specs-dir finding, so check degrades consistently to an
    // empty base scope instead of failing closed with a bare ENOENT (0.3.0
    // parity). A selected change with missing staged specs still bails below.
    let base_scenarios = if specs_dir.exists() {
        openspec::discover_scenarios(specs_dir.to_str().unwrap())?
    } else {
        Vec::new()
    };
    // Deployed-spec slug collisions must be detected on the raw discovery
    // list, BEFORE the (spec, id) map below collapses same-key duplicates
    // (last wins) — otherwise the slug-collision finding is dead code in
    // the ah check binary and only structural_findings()/doctor see it
    // (espectacular-ybs).
    let mut findings = slug_collision_findings(&base_scenarios, specs_dir);
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
        // Staged contracts are collected BEFORE the overlay scenarios so a
        // staged contract for a (spec, id) can act as the explicit signal that
        // the change deliberately MODIFIES a deployed scenario's text
        // (allow-overlay-scenario-modification).
        let mut change_staged: HashSet<(String, String)> = HashSet::new();
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
            change_staged.insert(key.clone());
            if let Some(existing) = scenarios.get_mut(&key) {
                existing.contract_path = path;
            }
        }
        for scenario in added_scenarios {
            let key = (scenario.spec_path.clone(), scenario.id.clone());
            if scenarios.contains_key(&key) {
                if change_staged.contains(&key) {
                    // Deliberate modification: the overlay prose replaces the
                    // deployed scenario text in scope. The staged loop above
                    // already pointed the contract path at the staged file.
                    if let Some(existing) = scenarios.get_mut(&key) {
                        existing.scenario = scenario;
                    }
                    continue;
                }
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
    }

    // Frontmatter id per deployed spec — VERIFIES links are file-local
    // (`<own-file-id>.<property-id>` under the rev-18 naming law).
    let file_ids = if specs_dir.exists() {
        openspec::frontmatter_ids(specs_dir.to_str().unwrap()).unwrap_or_default()
    } else {
        BTreeMap::new()
    };

    let covered = resolve_verifies_coverage(&mut scenarios, contracts_dir, &file_ids);

    Ok(ResolvedScope {
        scenarios: scenarios.into_values().collect(),
        contract_files,
        changes,
        findings,
        covered,
    })
}

/// Flag every scenario whose (spec, id) appears more than once in the raw
/// list. Detection runs over the raw discovery output, not the collapsed
/// map, so colliding duplicates are seen before they merge (last wins).
fn slug_collision_findings(
    scenarios: &[openspec::Scenario],
    specs_root: &Path,
) -> Vec<ReportFinding> {
    openspec::detect_slug_collisions(scenarios)
        .into_iter()
        .filter_map(|(spec, id, _)| {
            scenarios
                .iter()
                .find(|s| s.spec_path == spec && s.id == id)
                .map(|s| structural_report(s, specs_root, "slug-collision", None))
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
fn resolve_verifies_coverage(
    scenarios: &mut BTreeMap<(String, String), ResolvedScenario>,
    contracts_dir: &Path,
    file_ids: &BTreeMap<String, String>,
) -> BTreeMap<(String, String), Vec<String>> {
    let mut covered = BTreeMap::new();
    let mut redirected = std::collections::HashSet::new();
    for ((spec, id), resolved) in scenarios.iter_mut() {
        let file_id = file_ids.get(spec).map(String::as_str);
        for property_id in verifies_property_ids(&resolved.scenario.body, file_id) {
            let contract_id = crate::sync::slugify_property_id(&property_id);
            let path = contracts_dir.join(spec).join(format!("{contract_id}.toml"));
            if path.exists() {
                if redirected.insert((spec.clone(), id.clone())) {
                    resolved.contract_path = path;
                }
                covered
                    .entry((spec.clone(), id.clone()))
                    .or_insert_with(Vec::new)
                    .push(contract_id);
            }
        }
    }
    covered
}

fn evaluate_scope(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    scope: ResolvedScope,
    ah_scope: &str,
    run_tests: bool,
    changed_files: &[String],
    all_tests: bool,
) -> anyhow::Result<CheckOutput> {
    let stale_findings = contract_stale_findings(specs_root, &scope, SPK_PROGRAM);
    let mut findings = scope.findings;
    findings.extend(collect_structural_findings(
        &scope.scenarios,
        &scope.contract_files,
        specs_root,
        &scope.covered,
    ));
    findings.extend(stale_findings);

    let blocked = blocked_scenarios(&findings);
    // Layer 1 selection (espectacular-0k8): only when a changed-file set is
    // supplied. Capabilities with changed spec or contract files are
    // selected; the rest are skipped. --all-tests reports the escape hatch
    // (review F3). Zero-overlap guard (review F1/F9): a changed set that
    // maps to no capability, maps only to capabilities owning no in-scope
    // scenario, or contains files mapping to no capability at all must NOT
    // deselect anything — conservative run-all, bypass reported in the
    // JSON, never a hidden green.
    let mut selection_state = SelectionState::None;
    if run_tests && !changed_files.is_empty() {
        if all_tests {
            let mut all_caps: Vec<String> = scope
                .scenarios
                .iter()
                .map(|s| s.scenario.spec_path.clone())
                .collect();
            all_caps.sort();
            all_caps.dedup();
            selection_state = SelectionState::All {
                source: "all-tests",
                unmapped_files: 0,
                selected: all_caps,
            };
        } else {
            let mut caps = std::collections::HashSet::new();
            let mut unmapped = 0usize;
            for f in changed_files {
                match affected_capability(f, &cfg.paths.specs, &cfg.paths.changes) {
                    Some(cap) => {
                        caps.insert(cap);
                    }
                    None => {
                        unmapped += 1;
                    }
                }
            }
            let zero_overlap = caps.is_empty()
                || !scope
                    .scenarios
                    .iter()
                    .any(|s| caps.contains(&s.scenario.spec_path));
            if unmapped > 0 || zero_overlap {
                selection_state = SelectionState::All {
                    source: "bypassed",
                    unmapped_files: unmapped,
                    selected: Vec::new(),
                };
            } else {
                selection_state = SelectionState::Caps(caps);
            }
        }
    }
    let mut skipped = 0usize;
    let mut passed = 0usize;
    let mut extra_quality_findings: Vec<quality::QualityFinding> = Vec::new();

    // Layer 2 refinement (espectacular-0k8): when a testaruda store exists
    // and a changed-file set is supplied, consult it to prune contract
    // bindings whose underlying tests are unaffected. Guards, all
    // conservative (never a hidden green):
    // - --all-tests or empty changed set -> no consultation at all;
    // - select failure or unparseable output -> no pruning;
    // - EMPTY selection (exit 20, no code affected) -> skip refinement only,
    //   the Layer 1 set still runs;
    // - unresolved changed files -> the dependency view is incomplete, no
    //   pruning;
    // - anchor guard: pruning activates only when at least one binding in
    //   scope matches the selected set — an adapter that discovers nothing
    //   must not deselect anything.
    let l2_selection = if run_tests
        && !all_tests
        && !changed_files.is_empty()
        && testaruda_store_present(repo_root)
    {
        testaruda_select(repo_root, changed_files)
    } else {
        None
    };
    let l2_active = l2_selection
        .as_ref()
        .is_some_and(|sel| !sel.empty && !sel.any_unresolved && !sel.selected.is_empty());
    let mut l2_anchor_matched = false;
    if l2_active {
        let selected = &l2_selection.as_ref().unwrap().selected;
        for resolved in sorted_resolved_scenarios(&scope.scenarios) {
            let scenario = &resolved.scenario;
            if blocked.contains(&(scenario.spec_path.clone(), scenario.id.clone())) {
                continue;
            }
            if let SelectionState::Caps(caps) = &selection_state {
                if !caps.contains(&scenario.spec_path) {
                    continue;
                }
            }
            let contract = match contracts::load_contract(resolved.contract_path.to_str().unwrap())
            {
                Ok(contract) => contract,
                Err(_) => continue,
            };
            for (test_type, entries) in &contract.tests {
                for entry in entries {
                    if let Some(filter) = prunable_filter(test_type, entry) {
                        if node_contains_filter(selected, filter) {
                            l2_anchor_matched = true;
                        }
                    }
                }
            }
        }
    }
    let l2_prunes = l2_active && l2_anchor_matched;
    let mut testaruda_pruned = 0usize;

    if run_tests {
        for resolved in sorted_resolved_scenarios(&scope.scenarios) {
            let scenario = &resolved.scenario;
            if blocked.contains(&(scenario.spec_path.clone(), scenario.id.clone())) {
                continue;
            }

            if let SelectionState::Caps(caps) = &selection_state {
                if !caps.contains(&scenario.spec_path) {
                    skipped += 1;
                    continue;
                }
            }

            let contract = match contracts::load_contract(resolved.contract_path.to_str().unwrap())
            {
                Ok(contract) => contract,
                Err(_) => continue,
            };

            if contract.tests.is_empty()
                || contract.tests.values().all(|entries| entries.is_empty())
            {
                continue;
            }

            let mut test_types: Vec<_> = contract.tests.keys().cloned().collect();
            test_types.sort();
            for test_type in test_types {
                let entries = &contract.tests[&test_type];
                for entry in entries {
                    // Layer 2 pruning: drop bindings whose runner filter
                    // matches nothing in testaruda's selected set.
                    if l2_prunes {
                        if let Some(filter) = prunable_filter(&test_type, entry) {
                            if !node_contains_filter(
                                &l2_selection.as_ref().unwrap().selected,
                                filter,
                            ) {
                                testaruda_pruned += 1;
                                continue;
                            }
                        }
                    }

                    if test_type == "custom" {
                        match adapters::custom::invoke(repo_root, cfg, entry) {
                            Ok(adapters::custom::CustomRunnerResult::Passed) => {
                                passed += 1;
                            }
                            Ok(adapters::custom::CustomRunnerResult::TestFailing(result)) => {
                                findings.push(execution_report(scenario, specs_root, result));
                            }
                            Ok(adapters::custom::CustomRunnerResult::EnvelopeFindings(raw)) => {
                                for value in raw {
                                    match serde_json::from_value::<ReportFinding>(value) {
                                        Ok(finding) => findings.push(finding),
                                        Err(_) => findings.push(execution_report_message(
                                            scenario,
                                            specs_root,
                                            "custom runner emitted a finding that does not match the finding schema",
                                        )),
                                    }
                                }
                            }
                            Err(error) => {
                                findings.push(structural_report(
                                    scenario,
                                    specs_root,
                                    "missing-runner",
                                    Some(error.to_string()),
                                ));
                            }
                        }
                        continue;
                    }

                    if test_type == "property" || test_type == "snapshot" {
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
                            extra_quality_findings
                                .push(quality::QualityFinding::for_test_type(&test_type));
                        }
                        continue;
                    }

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
                    } else if runner::matched_zero_tests(&result) {
                        findings.push(no_tests_ran_report(scenario, specs_root, result));
                    } else {
                        passed += 1;
                    }
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

    let (mut quality_findings, tool_errors) =
        quality::collect_quality_findings(repo_root, &cfg.quality, ah_scope);
    quality_findings.extend(extra_quality_findings);
    for msg in tool_errors {
        findings.push(ReportFinding {
            kind: "test-failing".to_string(),
            category: "execution".to_string(),
            spec: "quality".to_string(),
            spec_path: "(quality/mutation)".to_string(),
            scenario: ScenarioContext {
                id: "mutation-tool".to_string(),
                title: "Mutation testing tool".to_string(),
                body_markdown: String::new(),
            },
            suggested_action: "fix_tool_invocation".to_string(),
            playbook_command: "ah explain fix_tool_invocation".to_string(),
            severity: "error".to_string(),
            scenario_prose: None,
            test: None,
            message: Some(msg),
        });
    }

    let selection_report = match &selection_state {
        SelectionState::None => None,
        SelectionState::All {
            source,
            unmapped_files,
            selected,
        } => Some(Selection {
            selected: selected.clone(),
            skipped,
            source: source.to_string(),
            unmapped_files: *unmapped_files,
            testaruda: l2_selection.as_ref().map(|_| TestarudaRefinement {
                pruned: testaruda_pruned,
            }),
        }),
        SelectionState::Caps(caps) => {
            let mut v: Vec<String> = caps.iter().cloned().collect();
            v.sort();
            Some(Selection {
                selected: v,
                skipped,
                source: "spec-provenance".to_string(),
                unmapped_files: 0,
                testaruda: l2_selection.as_ref().map(|_| TestarudaRefinement {
                    pruned: testaruda_pruned,
                }),
            })
        }
    };

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
        quality_findings,
        selection: selection_report,
    })
}

fn collect_structural_findings(
    scenarios: &[ResolvedScenario],
    contract_files: &[(String, String, PathBuf)],
    specs_root: &Path,
    covered: &BTreeMap<(String, String), Vec<String>>,
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
            findings.push(structural_report(scenario, specs_root, "no-toml", None));
            continue;
        }

        match contracts::load_contract(resolved.contract_path.to_str().unwrap()) {
            Ok(contract) => {
                // A VERIFIES-covered scenario resolves to its property's
                // derived contract, whose id is the slugified property id —
                // not the scenario id — so the mismatch check does not apply.
                let covered_via_link =
                    covered.contains_key(&(scenario.spec_path.clone(), scenario.id.clone()));
                if !covered_via_link && contract.id != scenario.id {
                    findings.push(structural_report(scenario, specs_root, "id-mismatch", None));
                }
                if !contract.falsifiability_class.is_empty()
                    && !contracts::FALSIFIABILITY_CLASSES
                        .contains(&contract.falsifiability_class.as_str())
                {
                    findings.push(structural_report(
                        scenario,
                        specs_root,
                        "invalid-falsifiability-class",
                        Some(format!(
                            "unknown falsifiability_class: {} (valid values: safety, liveness)",
                            contract.falsifiability_class
                        )),
                    ));
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
                if contract.status == "superseded"
                    && !known_ids
                        .contains(&(scenario.spec_path.clone(), contract.superseded_by.clone()))
                {
                    findings.push(structural_report(
                        scenario,
                        specs_root,
                        "missing-replacement",
                        Some(format!(
                            "replacement scenario '{}' is absent from scope",
                            contract.superseded_by
                        )),
                    ));
                }
                // Liveness timeout warning (gate delta C-liveness-timeout-warning):
                // fires only when ≥1 test entry exists (zero entries are governed
                // by no-tests-declared) and every entry lacks timeout_seconds.
                let all_entries: Vec<&contracts::TestEntry> =
                    contract.tests.values().flatten().collect();
                if contract.falsifiability_class == "liveness"
                    && !all_entries.is_empty()
                    && all_entries
                        .iter()
                        .all(|entry| entry.timeout_seconds.is_none())
                {
                    findings.push(structural_report(
                        scenario,
                        specs_root,
                        "missing-liveness-timeout",
                        Some(
                            concat!(
                            "liveness claim has no timeout_seconds on any test entry; liveness ",
                            "assertions are only falsifiable under bounded execution — declare ",
                            "timeout_seconds at least once"
                        )
                            .to_string(),
                        ),
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

    findings.extend(orphan_reports(
        contract_files,
        &scenario_map,
        specs_root,
        covered,
    ));
    findings.sort_by(report_finding_cmp);
    findings
}

fn orphan_reports(
    contract_files: &[(String, String, PathBuf)],
    scenarios: &BTreeMap<(String, String), &ResolvedScenario>,
    specs_root: &Path,
    covered: &BTreeMap<(String, String), Vec<String>>,
) -> Vec<ReportFinding> {
    // Derived contracts referenced by a VERIFIES link are in use even though
    // no scenario shares their id — never orphans (task 4.1).
    let covered_contract_keys: HashSet<(String, String)> = covered
        .iter()
        .flat_map(|((spec, _), contract_ids)| {
            contract_ids
                .iter()
                .map(move |cid| (spec.clone(), cid.clone()))
        })
        .collect();
    let mut findings = Vec::new();
    let mut seen = HashSet::new();
    for (spec, id, path) in contract_files {
        if !seen.insert((spec.clone(), id.clone(), path.clone())) {
            continue;
        }
        if scenarios.contains_key(&(spec.clone(), id.clone())) {
            continue;
        }
        if covered_contract_keys.contains(&(spec.clone(), id.clone())) {
            continue;
        }
        findings.push(report_finding(
            "orphan-toml",
            "structural",
            spec.clone(),
            path.parent()
                .map(|_| spec_markdown_path(specs_root, spec))
                .unwrap_or_else(|| spec_markdown_path(specs_root, spec)),
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

pub fn collect_base_contract_files(contracts_root: &Path) -> Vec<(String, String, PathBuf)> {
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

/// spk binary resolved from PATH (same convention as `ah sync`).
const SPK_PROGRAM: &str = "spk";

/// contract-stale drift detection (derive-contracts-from-specodelic task 4.2,
/// C-derived-stale): a derived contract's `derived_from` hash must match the
/// canonical serialization of its source property row. Findings are attached
/// to the covering scenario when one exists, else to the contract id itself.
/// Degrades silently when spk is missing or a spec fails to parse (design D2:
/// validity stays with spk; a broken parse is caught by ah lint instead).
fn contract_stale_findings(
    specs_root: &Path,
    scope: &ResolvedScope,
    spk: &str,
) -> Vec<ReportFinding> {
    // Collect the derived contracts (derived_from non-empty) per spec.
    let mut by_spec: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (spec, id, path) in &scope.contract_files {
        let derived = fs::read_to_string(path)
            .ok()
            .and_then(|text| text.parse::<toml::Table>().ok())
            .and_then(|table| {
                table
                    .get("derived_from")
                    .and_then(toml::Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_default();
        if derived.is_empty() {
            continue;
        }
        by_spec
            .entry(spec.clone())
            .or_default()
            .push((id.clone(), derived));
    }
    if by_spec.is_empty() {
        return Vec::new();
    }

    // (spec, contract id) → covering scenario, for finding attachment.
    let covering: BTreeMap<(String, String), &Scenario> = scope
        .covered
        .iter()
        .flat_map(|((spec, scenario_id), contract_ids)| {
            contract_ids.iter().filter_map(move |contract_id| {
                scope
                    .scenarios
                    .iter()
                    .find(|r| r.scenario.spec_path == *spec && r.scenario.id == *scenario_id)
                    .map(|r| ((spec.clone(), contract_id.clone()), &r.scenario))
            })
        })
        .collect();

    let mut findings = Vec::new();
    for (spec, derived_contracts) in by_spec {
        let markdown = spec_markdown_path(specs_root, &spec);
        let Ok(ir) = crate::derive::parse_spec_ir(spk, Path::new(&markdown)) else {
            continue;
        };
        for (contract_id, derived) in derived_contracts {
            let Some((property_id, stored)) = derived.split_once('@') else {
                continue;
            };
            let message = match ir.properties.iter().find(|row| row.id == property_id) {
                None => Some(format!(
                    "derived_from references property {property_id}, which is absent from the specodelic Properties table"
                )),
                Some(row) => {
                    let fresh = crate::derive::canonical_hash(row);
                    (fresh != derived).then(|| {
                        format!(
                            "derived_from hash @{stored} does not match specodelic row {property_id} (@{}); run ah sync to refresh derived fields",
                            fresh.split_once('@').map(|(_, h)| h).unwrap_or("?")
                        )
                    })
                }
            };
            let Some(message) = message else {
                continue;
            };
            let synthetic = Scenario {
                id: contract_id.clone(),
                heading: String::new(),
                spec_path: spec.clone(),
                source_line: 0,
                body: String::new(),
            };
            let scenario = covering
                .get(&(spec.clone(), contract_id.clone()))
                .copied()
                .unwrap_or(&synthetic);
            findings.push(structural_report(
                scenario,
                specs_root,
                "contract-stale",
                Some(message),
            ));
        }
    }
    findings
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

fn execution_report_message(
    scenario: &Scenario,
    specs_root: &Path,
    message: &str,
) -> ReportFinding {
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
        None,
        Some(message.to_string()),
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

#[allow(clippy::too_many_arguments)]
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
    let severity = if kind == "missing-liveness-timeout" {
        "warning"
    } else {
        "error"
    };
    let category = if severity == "warning" {
        "warning"
    } else {
        category
    };
    ReportFinding {
        kind: kind.to_string(),
        category: category.to_string(),
        spec,
        spec_path,
        scenario,
        suggested_action: suggested_action.clone(),
        playbook_command: format!("ah explain {suggested_action}"),
        severity: severity.to_string(),
        scenario_prose,
        test,
        message,
    }
}

fn no_tests_ran_report(scenario: &Scenario, specs_root: &Path, test: TestResult) -> ReportFinding {
    report_finding(
        "no-tests-ran",
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

fn suggested_action_for(kind: &str) -> &'static str {
    match kind {
        "no-toml"
        | "orphan-toml"
        | "slug-collision"
        | "id-mismatch"
        | "invalid-status"
        | "invalid-falsifiability-class"
        | "missing-liveness-timeout"
        | "no-tests-declared"
        | "malformed-contract"
        | "missing-replacement"
        | "overlay-conflict" => "review_and_apply",
        "missing-runner" | "test-failing" | "no-tests-ran" => "edit_code_not_scenario",
        "contract-stale" => "run_ah_sync",
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

pub(crate) fn contract_path(contracts_root: &Path, spec: &str, id: &str) -> PathBuf {
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
        // 5 findings: missing-contract, orphan-toml, no-tests-declared, slug-collision x2
        // (both scenarios sharing a colliding slug are now flagged)
        assert_eq!(findings.len(), 5);
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

    #[test]
    fn invalid_falsifiability_class_emits_structural_finding() {
        let dir = success_repo();
        let contract = dir.path().join(".espectacular/compiler/green-path.toml");
        let text = fs::read_to_string(&contract).unwrap();
        fs::write(
            &contract,
            text.replace(
                "status = \"active\"",
                "status = \"active\"\nfalsifiability_class = \"eventual\"",
            ),
        )
        .unwrap();

        let specs = dir.path().join("openspec/specs");
        let contracts = dir.path().join(".espectacular");
        let findings =
            structural_findings(specs.to_str().unwrap(), contracts.to_str().unwrap()).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.kind == "invalid-falsifiability-class"));
    }

    // ---- VERIFIES coverage (derive-contracts-from-specodelic task 4.1) ----

    const LINKED_BODY: &str = "- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[spec.P-token]]\n";
    const UNLINKED_BODY: &str =
        "- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n";

    const DERIVED_CONTRACT: &str = "id = \"p-token\"\ndescription = \"invalid tokens rejected\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.9.0\"\nfalsifiability_class = \"safety\"\nderived_from = \"P-token@0123456789ab\"\n\n[[tests.cargo]]\nflags = \"p_token::case1\"\n";

    const OTHER_DERIVED_CONTRACT: &str = "id = \"p-other\"\ndescription = \"second property covered by the same scenario\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.9.0\"\nfalsifiability_class = \"safety\"\nderived_from = \"P-other@0123456789ab\"\n\n[[tests.cargo]]\nflags = \"p_other::case1\"\n";

    fn verifies_repo(scenario_bodies: &[&str], contracts: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/auth")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/auth")).unwrap();
        fs::write(
            repo.join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
        ).unwrap();
        let mut spec = String::from("# Capability: auth\n");
        for (i, body) in scenario_bodies.iter().enumerate() {
            spec.push_str(&format!("\n#### Scenario: Token check {i}\n{body}\n"));
        }
        fs::write(repo.join("openspec/specs/auth/spec.md"), spec).unwrap();
        for (name, content) in contracts {
            fs::write(repo.join(".espectacular/auth").join(name), content).unwrap();
        }
        dir
    }

    #[test]
    fn verifies_linked_scenario_is_covered_by_derived_contract() {
        let dir = verifies_repo(&[LINKED_BODY], &[("p-token.toml", DERIVED_CONTRACT)]);
        let output = run_check(dir.path(), &[], false).unwrap();
        for kind in ["no-toml", "no-tests-declared", "orphan-toml", "id-mismatch"] {
            assert!(
                !output.findings.iter().any(|f| f.kind == kind),
                "VERIFIES-linked scenario must not emit {kind}; got: {:?}",
                output.findings
            );
        }
    }

    #[test]
    fn verifies_real_id_frontmatter_link_is_covered_by_derived_contract() {
        // Revision 18: the link is file-local under the real parent-dir id
        // (`auth.tokens.P-token`), not the legacy `spec.P-token` spelling.
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/auth-tokens")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/auth-tokens")).unwrap();
        fs::write(
            repo.join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
        )
        .unwrap();
        fs::write(
            repo.join("openspec/specs/auth-tokens/spec.md"),
            "---\nid: auth.tokens\nkind: intent\nstatement: \"WHEN checked THE system SHALL reject invalid tokens\"\n---\n\n# Capability: auth tokens\n\n#### Scenario: Token check 0\n- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[auth.tokens.P-token]]\n",
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/auth-tokens/p-token.toml"),
            DERIVED_CONTRACT,
        )
        .unwrap();
        let output = run_check(repo, &[], false).unwrap();
        for kind in ["no-toml", "orphan-toml"] {
            assert!(
                !output.findings.iter().any(|f| f.kind == kind),
                "real-id VERIFIES link must resolve; got: {:?}",
                output.findings
            );
        }
    }

    #[test]
    fn scenario_with_two_links_covers_both_derived_contracts() {
        // A scenario carrying two VERIFIES bullets asserts both properties;
        // both derived contracts are in use — neither may be orphan-toml.
        // (Full-corpus migration: P-handlers/P-selfheal share one scenario.)
        const TWO_LINKS: &str = "- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[spec.P-token]]\n- **VERIFIES** [[spec.P-other]]\n";
        let dir = verifies_repo(
            &[TWO_LINKS],
            &[
                ("p-token.toml", DERIVED_CONTRACT),
                ("p-other.toml", OTHER_DERIVED_CONTRACT),
            ],
        );
        let output = run_check(dir.path(), &[], false).unwrap();
        assert!(
            !output.findings.iter().any(|f| f.kind == "orphan-toml"),
            "both linked contracts are in use; got: {:?}",
            output.findings
        );
    }

    #[test]
    fn covered_scenario_executes_derived_contract_tests() {
        let dir = verifies_repo(&[LINKED_BODY], &[("p-token.toml", DERIVED_CONTRACT)]);
        // Swap the derived contract's tests for a passing shell entry to prove
        // the redirect feeds the execution path, not just the structural pass.
        let contract = dir.path().join(".espectacular/auth/p-token.toml");
        let text = fs::read_to_string(&contract).unwrap();
        fs::write(
            &contract,
            text.replace(
                "[[tests.cargo]]\nflags = \"p_token::case1\"",
                "[[tests.shell]]\ncommand = \"exit 0\"",
            ),
        )
        .unwrap();
        let output = run_check(dir.path(), &[], true).unwrap();
        assert_eq!(
            output.summary.passed, 1,
            "derived contract tests must run; got: {:?}",
            output.findings
        );
    }

    #[test]
    fn unlinked_scenario_still_requires_own_contract() {
        let dir = verifies_repo(&[UNLINKED_BODY], &[("p-token.toml", DERIVED_CONTRACT)]);
        let output = run_check(dir.path(), &[], false).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "no-toml"),
            "unlinked scenario must still emit no-toml; got: {:?}",
            output.findings
        );
    }

    #[test]
    fn link_to_testless_derived_contract_still_flags_no_tests_declared() {
        // The link alone is not enough: the covering derived contract must
        // actually declare tests (C-verifies-covers covers the scenario; the
        // contract's own no-tests-declared check still applies).
        let empty = DERIVED_CONTRACT.replace("\n[[tests.cargo]]\nflags = \"p_token::case1\"", "");
        let dir = verifies_repo(&[LINKED_BODY], &[("p-token.toml", empty.as_str())]);
        let output = run_check(dir.path(), &[], false).unwrap();
        assert!(
            output
                .findings
                .iter()
                .any(|f| f.kind == "no-tests-declared"),
            "testless derived contract must still emit no-tests-declared; got: {:?}",
            output.findings
        );
    }

    #[test]
    fn verifies_link_to_absent_property_keeps_own_contract_requirement() {
        let dir = verifies_repo(&[LINKED_BODY], &[]);
        let output = run_check(dir.path(), &[], false).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "no-toml"),
            "link without a derived contract must still emit no-toml; got: {:?}",
            output.findings
        );
    }

    // ---- contract-stale (derive-contracts-from-specodelic task 4.2) ----

    const P_TOKEN_CELLS: &[(&str, &str)] = &[
        ("id", "P-token"),
        ("kind", "unit"),
        ("derives_from", "[[spec.C-token]]"),
        ("generator", "valid vs invalid tokens"),
        ("predicate", "invalid tokens rejected with 401"),
    ];

    fn p_token_row() -> crate::derive::Row {
        crate::derive::Row {
            id: "P-token".to_string(),
            kind: Some("unit".to_string()),
            cells: P_TOKEN_CELLS
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    /// Fake spk: `parse <path> --json` emits an IR carrying exactly the
    /// P-token row from `p_token_row`.
    fn parse_shim(dir: &Path, name: &str) -> String {
        // Payload goes to a file and the shim `cat`s it. The earlier form fed
        // the JSON through printf's FORMAT string with \" escapes, which only
        // bash collapses — dash (CI's /bin/sh) prints them literally and the
        // parse result becomes invalid JSON (espectacular-pli CI failure).
        let mut cells = String::new();
        for (k, v) in P_TOKEN_CELLS {
            cells.push_str(&format!("\"{k}\":\"{v}\","));
        }
        cells.pop();
        let ir = format!(
            "{{\"ok\":true,\"data\":{{\"properties\":[{{\"id\":\"P-token\",\"kind\":\"unit\",\"cells\":{{{cells}}}}}],\"constraints\":[],\"states\":[],\"transitions\":[]}}}}"
        );
        let payload = dir.join(format!("{name}.json"));
        fs::write(&payload, &ir).unwrap();
        write_executable(
            &dir.join(name),
            &format!(
                "if [ \"$1\" = parse ]; then\n  cat '{}'\nelse\n  printf '%s' '{{\"ok\":true,\"data\":{{\"issues\":[]}}}}'\nfi",
                payload.display()
            ),
        );
        dir.join(name).to_string_lossy().into_owned()
    }

    fn stale_fixture(derived_from: &str) -> (tempfile::TempDir, ResolvedScope) {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/auth")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/auth")).unwrap();
        fs::write(
            repo.join("openspec/specs/auth/spec.md"),
            "# Capability: auth\n",
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/auth/p-token.toml"),
            format!(
                "id = \"p-token\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.9.0\"\nfalsifiability_class = \"safety\"\nderived_from = \"{derived_from}\"\n\n[[tests.cargo]]\nflags = \"p_token::case1\"\n"
            ),
        )
        .unwrap();
        let scope = ResolvedScope {
            scenarios: Vec::new(),
            contract_files: vec![(
                "auth".to_string(),
                "p-token".to_string(),
                repo.join(".espectacular/auth/p-token.toml"),
            )],
            changes: Vec::new(),
            findings: Vec::new(),
            covered: BTreeMap::new(),
        };
        (dir, scope)
    }

    #[test]
    fn contract_stale_emitted_when_row_hash_differs() {
        let (dir, scope) = stale_fixture("P-token@deadbeefcafe");
        let spk = parse_shim(dir.path(), "spk-stale");
        let findings = contract_stale_findings(&dir.path().join("openspec/specs"), &scope, &spk);
        assert!(
            findings
                .iter()
                .any(|f| f.kind == "contract-stale" && f.category == "structural"),
            "expected contract-stale; got: {:?}",
            findings
        );
    }

    #[test]
    fn fresh_derived_contract_emits_no_contract_stale() {
        let fresh = crate::derive::canonical_hash(&p_token_row());
        let (dir, scope) = stale_fixture(&fresh);
        let spk = parse_shim(dir.path(), "spk-fresh");
        let findings = contract_stale_findings(&dir.path().join("openspec/specs"), &scope, &spk);
        assert!(
            !findings.iter().any(|f| f.kind == "contract-stale"),
            "unexpected contract-stale; got: {:?}",
            findings
        );
    }

    #[test]
    fn derived_from_property_absent_from_ir_is_stale() {
        let (dir, scope) = stale_fixture("P-gone@deadbeefcafe");
        let spk = parse_shim(dir.path(), "spk-gone");
        let findings = contract_stale_findings(&dir.path().join("openspec/specs"), &scope, &spk);
        let stale = findings
            .iter()
            .find(|f| f.kind == "contract-stale")
            .expect("absent property must be stale");
        assert!(
            stale
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("P-gone"),
            "message must name the absent property; got: {:?}",
            stale.message
        );
    }

    #[test]
    fn contract_stale_finding_attaches_to_covering_scenario() {
        let (dir, mut scope) = stale_fixture("P-token@deadbeefcafe");
        scope.covered.insert(
            ("auth".to_string(), "token-check".to_string()),
            vec!["p-token".to_string()],
        );
        scope.scenarios.push(ResolvedScenario {
            scenario: crate::openspec::Scenario {
                id: "token-check".to_string(),
                heading: "Token check".to_string(),
                spec_path: "auth".to_string(),
                source_line: 3,
                body: "- **VERIFIES** [[spec.P-token]]".to_string(),
            },
            contract_path: dir.path().join(".espectacular/auth/p-token.toml"),
        });
        let spk = parse_shim(dir.path(), "spk-covered");
        let findings = contract_stale_findings(&dir.path().join("openspec/specs"), &scope, &spk);
        let stale = findings
            .iter()
            .find(|f| f.kind == "contract-stale")
            .expect("stale finding");
        assert_eq!(stale.scenario.id, "token-check");
    }

    #[test]
    fn contract_stale_degrades_without_spk() {
        let (dir, scope) = stale_fixture("P-token@deadbeefcafe");
        let findings = contract_stale_findings(
            &dir.path().join("openspec/specs"),
            &scope,
            "spk-definitely-not-on-path-xyz",
        );
        assert!(
            findings.is_empty(),
            "missing spk must degrade to no findings; got: {:?}",
            findings
        );
    }

    #[test]
    fn run_check_does_not_fail_on_derived_contracts_without_spk_changes() {
        // Regression: a repo with derived contracts and no spk on PATH still
        // checks normally (stale detection silently degrades, design D2).
        let (dir, _scope) = stale_fixture("P-token@deadbeefcafe");
        fs::write(
            dir.path().join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
        )
        .unwrap();
        let output = run_check(dir.path(), &[], false).unwrap();
        assert!(
            !output.findings.iter().any(|f| f.kind == "contract-stale"),
            "stale must degrade without spk; got: {:?}",
            output.findings
        );
        // The structural pass still sees the contract as healthy.
        assert!(
            !output
                .findings
                .iter()
                .any(|f| f.kind == "malformed-contract"),
            "derived contract must load; got: {:?}",
            output.findings
        );
    }

    // ---- Regression: plain + Properties-less specs unaffected (task 4.5) ----
    // C-plain-unaffected: plain openspec specs and dual-format specs without
    // Properties rows discover and check exactly as before this change.

    #[test]
    fn plain_spec_with_derived_named_contract_is_unaffected() {
        // Pre-change corpus shape: no VERIFIES bullet anywhere, but a
        // hand-written contract whose id matches no scenario → no-toml for
        // the scenario plus orphan-toml for the stray contract, exactly as
        // before coverage existed.
        let dir = verifies_repo(&[UNLINKED_BODY], &[("p-token.toml", DERIVED_CONTRACT)]);
        let output = run_check(dir.path(), &[], false).unwrap();
        assert!(output.findings.iter().any(|f| f.kind == "no-toml"));
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "orphan-toml" && f.scenario.id == "p-token"));
    }

    #[test]
    fn plain_spec_verifies_bullet_without_derived_contract_changes_nothing() {
        // A bullet in a plain (frontmatter-less) spec is inert when no
        // derived contract backs it: same single no-toml finding as without
        // the bullet.
        let with_link = verifies_repo(&[LINKED_BODY], &[]);
        let without_link = verifies_repo(&[UNLINKED_BODY], &[]);
        let extract = |output: &crate::check::CheckOutput| {
            let mut kinds: Vec<(String, String)> = output
                .findings
                .iter()
                .map(|f| (f.kind.clone(), f.scenario.id.clone()))
                .collect();
            kinds.sort();
            format!("{kinds:?}")
        };
        let a = run_check(with_link.path(), &[], false).unwrap();
        let b = run_check(without_link.path(), &[], false).unwrap();
        assert_eq!(
            extract(&a),
            extract(&b),
            "bullet must be inert without a derived contract"
        );
        assert!(a.findings.iter().any(|f| f.kind == "no-toml"));
    }

    #[test]
    fn propertiesless_dual_format_spec_is_unaffected() {
        // Dual-format frontmatter (id: spec) but no Properties rows: the
        // bullet must not suppress the scenario's own contract requirement.
        let dir = verifies_repo(&[LINKED_BODY], &[]);
        fs::write(
            dir.path().join("openspec/specs/auth/spec.md"),
            "---\nid: spec\nkind: intent\nstatement: \"WHEN checked THE system SHALL reject invalid tokens\"\n---\n\n# Capability: auth\n\n#### Scenario: Token check 0\n- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[spec.P-token]]\n",
        )
        .unwrap();
        let output = run_check(dir.path(), &[], false).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "no-toml"),
            "Properties-less dual-format spec must keep the contract requirement; got: {:?}",
            output.findings
        );
    }

    #[test]
    fn discovery_output_is_identical_for_bullet_bearing_scenarios() {
        // Discovery is text-faithful: a VERIFIES bullet is part of the body
        // and must not change scenario ids, headings, or line numbers.
        let dir = verifies_repo(&[LINKED_BODY], &[]);
        let scenarios = crate::openspec::discover_scenarios(
            dir.path().join("openspec/specs").to_str().unwrap(),
        )
        .unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].id, "token-check-0");
        assert_eq!(scenarios[0].heading, "Token check 0");
        assert!(scenarios[0].body.contains("**VERIFIES** [[spec.P-token]]"));
    }

    #[test]
    fn invalid_falsifiability_class_blocks_scenario_execution() {
        let dir = success_repo();
        let contract = dir.path().join(".espectacular/compiler/green-path.toml");
        let text = fs::read_to_string(&contract).unwrap();
        fs::write(
            &contract,
            text.replace(
                "status = \"active\"",
                "status = \"active\"\nfalsifiability_class = \"eventual\"",
            ),
        )
        .unwrap();

        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "invalid-falsifiability-class"));
        assert_eq!(output.summary.passed, 0, "declared tests must not run");
    }

    #[test]
    fn valid_falsifiability_class_values_pass_and_run_tests() {
        // safety: no findings at all
        {
            let dir = success_repo();
            let contract = dir.path().join(".espectacular/compiler/green-path.toml");
            let text = fs::read_to_string(&contract).unwrap();
            fs::write(
                &contract,
                text.replace(
                    "status = \"active\"",
                    "status = \"active\"\nfalsifiability_class = \"safety\"",
                ),
            )
            .unwrap();
            let output = run_check(dir.path(), &[], true).unwrap();
            assert!(
                output.findings.is_empty(),
                "falsifiability_class = safety must not produce findings"
            );
            assert_eq!(output.summary.passed, 1);
        }
        // liveness: only the timeout warning (entry in success_repo has no timeout)
        {
            let dir = success_repo();
            let contract = dir.path().join(".espectacular/compiler/green-path.toml");
            let text = fs::read_to_string(&contract).unwrap();
            fs::write(
                &contract,
                text.replace(
                    "status = \"active\"",
                    "status = \"active\"\nfalsifiability_class = \"liveness\"",
                ),
            )
            .unwrap();
            let output = run_check(dir.path(), &[], true).unwrap();
            assert_eq!(output.findings.len(), 1);
            assert_eq!(output.findings[0].kind, "missing-liveness-timeout");
            assert_eq!(output.findings[0].severity, "warning");
            assert_eq!(output.summary.passed, 1);
        }
    }

    #[test]
    fn liveness_without_timeout_emits_warning_severity() {
        let dir = success_repo();
        let contract = dir.path().join(".espectacular/compiler/green-path.toml");
        let text = fs::read_to_string(&contract).unwrap();
        fs::write(
            &contract,
            text.replace(
                "status = \"active\"",
                "status = \"active\"\nfalsifiability_class = \"liveness\"",
            ),
        )
        .unwrap();

        let output = run_check(dir.path(), &[], true).unwrap();
        let warnings: Vec<_> = output
            .findings
            .iter()
            .filter(|f| f.kind == "missing-liveness-timeout")
            .collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].severity, "warning");
        assert_eq!(warnings[0].category, "warning");
        assert_eq!(
            output.summary.structural, 0,
            "warning must not be structural"
        );
        assert_eq!(output.summary.passed, 1, "tests still run and pass");
    }

    #[test]
    fn liveness_with_timeout_emits_no_warning() {
        let dir = success_repo();
        let contract = dir.path().join(".espectacular/compiler/green-path.toml");
        let text = fs::read_to_string(&contract).unwrap();
        fs::write(
            &contract,
            text.replace(
                "[[tests.unit]]\nflags = \"ok\"",
                "falsifiability_class = \"liveness\"\n\n[[tests.unit]]\nflags = \"ok\"\ntimeout_seconds = 30",
            ),
        )
        .unwrap();

        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "missing-liveness-timeout"));
        assert_eq!(output.summary.passed, 1);
    }

    #[test]
    fn structural_findings_carry_error_severity() {
        let dir = success_repo();
        let contract = dir.path().join(".espectacular/compiler/green-path.toml");
        fs::write(&contract, "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\nfalsifiability_class = \"liveness\"\n[tests]\n").unwrap();

        let output = run_check(dir.path(), &[], true).unwrap();
        let declared: Vec<_> = output
            .findings
            .iter()
            .filter(|f| f.kind == "no-tests-declared")
            .collect();
        assert_eq!(declared.len(), 1);
        assert_eq!(declared[0].severity, "error");
        assert!(
            !output
                .findings
                .iter()
                .any(|f| f.kind == "missing-liveness-timeout"),
            "zero test entries must not emit the liveness warning"
        );
    }

    #[test]
    fn non_liveness_findings_serialize_error_severity() {
        let dir = success_repo();
        let contract = dir.path().join(".espectacular/compiler/green-path.toml");
        let text = fs::read_to_string(&contract).unwrap();
        fs::write(
            &contract,
            text.replace(
                "status = \"active\"",
                "status = \"active\"\nfalsifiability_class = \"eventual\"",
            ),
        )
        .unwrap();

        let output = run_check(dir.path(), &[], true).unwrap();
        let findings: Vec<_> = output
            .findings
            .iter()
            .filter(|f| f.kind == "invalid-falsifiability-class")
            .collect();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, "error");
        // byte-compatibility: serialized JSON keeps every pre-existing field
        let json = serde_json::to_value(findings[0].clone()).unwrap();
        for key in [
            "kind",
            "category",
            "spec",
            "spec_path",
            "scenario",
            "suggested_action",
            "playbook_command",
            "message",
        ] {
            assert!(json.get(key).is_some(), "missing pre-existing field {key}");
        }
        // external (custom-runner) findings without severity deserialize as error
        let external: ReportFinding =
            serde_json::from_value(serde_json::json!({"kind": "test-failing", "category": "execution", "spec": "s", "spec_path": "p", "scenario": {"id": "i", "title": "t", "body_markdown": "b"}, "suggested_action": "edit_code_not_scenario", "playbook_command": "ah explain edit_code_not_scenario"})).unwrap();
        assert_eq!(external.severity, "error");
    }

    #[test]
    fn every_preexisting_finding_kind_serializes_error_severity() {
        // All kinds that existed before the severity field (schema enum minus
        // the two falsifiability kinds introduced alongside it).
        let kinds = [
            "no-toml",
            "orphan-toml",
            "slug-collision",
            "id-mismatch",
            "invalid-status",
            "no-tests-declared",
            "missing-runner",
            "malformed-contract",
            "missing-replacement",
            "overlay-conflict",
            "test-failing",
            "no-tests-ran",
        ];
        for kind in kinds {
            let finding = report_finding(
                kind,
                "structural",
                "s".to_string(),
                "p".to_string(),
                ScenarioContext {
                    id: "i".to_string(),
                    title: "t".to_string(),
                    body_markdown: "b".to_string(),
                },
                None,
                None,
                None,
            );
            assert_eq!(finding.severity, "error", "kind {kind} must be error");
            let json = serde_json::to_value(&finding).unwrap();
            for key in [
                "kind",
                "category",
                "spec",
                "spec_path",
                "scenario",
                "suggested_action",
                "playbook_command",
            ] {
                assert!(
                    json.get(key).is_some(),
                    "kind {kind}: missing pre-existing field {key}"
                );
            }
        }
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
    fn run_check_missing_specs_dir_degrades_to_clean_zero_findings() {
        // GH#28.2 / espectacular-iq4: a repo with no deployed specs yet (changes
        // only) must not hard-fail with a bare ENOENT — ah doctor classifies a
        // missing specs dir as a missing-specs-dir finding, so check degrades
        // consistently (0.3.0 parity: clean, zero structural).
        let dir = success_repo();
        fs::remove_dir_all(dir.path().join("openspec/specs")).unwrap();
        fs::remove_dir_all(dir.path().join(".espectacular/compiler")).unwrap();

        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(output.findings.is_empty());
        assert_eq!(output.summary.passed, 0);
    }

    #[test]
    fn run_check_reports_success_with_empty_findings() {
        let dir = success_repo();
        let output = run_check(dir.path(), &[], true).unwrap();
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

        let output = run_check(dir.path(), &["add-parser".to_string()], true).unwrap();
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

        let output =
            run_check(dir.path(), &["zeta".to_string(), "alpha".to_string()], true).unwrap();
        assert_eq!(output.scope.changes, vec!["alpha", "zeta"]);
        assert!(output.findings.iter().any(|f| f.kind == "overlay-conflict"));
        assert_eq!(
            output.summary.counts_by_kind.get("overlay-conflict"),
            Some(&1)
        );
    }

    #[test]
    fn run_check_overlay_modification_with_staged_contract_passes() {
        // RED (espectacular-hct / allow-overlay-scenario-modification):
        // a change delta that redefines a deployed scenario id WITH a staged
        // contract is a deliberate modification — the overlay text replaces
        // the base text, no overlay-conflict.
        let dir = success_repo();
        fs::create_dir_all(
            dir.path()
                .join("openspec/changes/mod-parser/specs/compiler"),
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".espectacular/changes/mod-parser/compiler")).unwrap();
        fs::write(
            dir.path().join("openspec/changes/mod-parser/specs/compiler/spec.md"),
            "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs modified\n- **THEN** it passes differently\n",
        ).unwrap();
        fs::write(
            dir.path().join(".espectacular/changes/mod-parser/compiler/green-path.toml"),
            "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
        ).unwrap();

        let output = run_check(dir.path(), &["mod-parser".to_string()], true).unwrap();
        assert!(
            !output.findings.iter().any(|f| f.kind == "overlay-conflict"),
            "staged-contract modification must not conflict; got: {:?}",
            output.findings
        );
    }

    #[test]
    fn run_check_unsignaled_scenario_redefinition_still_conflicts() {
        let dir = success_repo();
        fs::create_dir_all(
            dir.path()
                .join("openspec/changes/mod-parser/specs/compiler"),
        )
        .unwrap();
        fs::write(
            dir.path().join("openspec/changes/mod-parser/specs/compiler/spec.md"),
            "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs modified\n- **THEN** it passes differently\n",
        ).unwrap();

        let output = run_check(dir.path(), &["mod-parser".to_string()], true).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "overlay-conflict"),
            "redefinition without a staged contract must still conflict; got: {:?}",
            output.findings
        );
    }

    #[test]
    fn run_check_conflicting_scenario_modifications_across_changes() {
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
                "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs modified\n- **THEN** it passes differently\n",
            ).unwrap();
            fs::write(
                dir.path().join(format!(".espectacular/changes/{change}/compiler/green-path.toml")),
                "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
            ).unwrap();
        }

        let output =
            run_check(dir.path(), &["zeta".to_string(), "alpha".to_string()], true).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "overlay-conflict"),
            "two changes modifying the same deployed scenario must conflict; got: {:?}",
            output.findings
        );
    }

    #[test]
    fn findings_carry_action_fields_and_scenario_prose() {
        let dir = success_repo();
        write_executable(&dir.path().join("runner.sh"), "printf 'boom' >&2\nexit 7");

        let output = run_check(dir.path(), &[], true).unwrap();
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

    fn custom_repo(runner_body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
        fs::write(
            repo.join("openspec/specs/compiler/spec.md"),
            "# Capability: compiler\n\n#### Scenario: Custom check\n- **WHEN** custom runner invoked\n- **THEN** it reports results\n",
        )
        .unwrap();
        let runner_path = repo.join("custom-runner.sh");
        write_executable(&runner_path, runner_body);
        fs::write(
            repo.join(".espectacular/config.toml"),
            format!(
                "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncustom = [\"{runner}\"]\n",
                runner = runner_path.display()
            ),
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/compiler/custom-check.toml"),
            "id = \"custom-check\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.custom]]\nflags = \"custom-check\"\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn custom_runner_passed_envelope_counts_as_passed() {
        let dir = custom_repo("printf '{\"exit_code\":0,\"passed\":true,\"findings\":[]}'");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(output.findings.is_empty());
        assert_eq!(output.summary.passed, 1);
    }

    #[test]
    fn custom_runner_non_zero_exit_emits_test_failing() {
        let dir = custom_repo("printf 'error' >&2; exit 1");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(output.findings.iter().any(|f| f.kind == "test-failing"));
        assert_eq!(output.summary.passed, 0);
    }

    #[test]
    fn custom_runner_envelope_findings_flow_into_check_output() {
        let finding = serde_json::json!({
            "kind": "test-failing",
            "category": "execution",
            "spec": "compiler",
            "spec_path": "openspec/specs/compiler/spec.md",
            "scenario": {"id": "custom-check", "title": "Custom check", "body_markdown": "B"},
            "suggested_action": "edit_code_not_scenario",
            "playbook_command": "ah explain edit_code_not_scenario"
        });
        let envelope = serde_json::json!({
            "exit_code": 0,
            "passed": false,
            "findings": [finding]
        });
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
        fs::write(
            repo.join("openspec/specs/compiler/spec.md"),
            "# Capability: compiler\n\n#### Scenario: Custom check\n- **WHEN** custom runner invoked\n- **THEN** it reports results\n",
        )
        .unwrap();
        let data = repo.join("envelope.json");
        fs::write(&data, envelope.to_string()).unwrap();
        let runner_path = repo.join("custom-runner.sh");
        write_executable(&runner_path, &format!("cat {}", data.display()));
        fs::write(
            repo.join(".espectacular/config.toml"),
            format!(
                "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncustom = [\"{runner}\"]\n",
                runner = runner_path.display()
            ),
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/compiler/custom-check.toml"),
            "id = \"custom-check\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.custom]]\nflags = \"custom-check\"\n",
        )
        .unwrap();

        let output = run_check(repo, &[], true).unwrap();
        assert_eq!(output.findings.len(), 1);
        assert_eq!(output.findings[0].kind, "test-failing");
        assert_eq!(output.summary.passed, 0);
    }

    fn quality_test_repo(test_type: &str, runner_body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/suite")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/suite")).unwrap();
        fs::write(
            repo.join("openspec/specs/suite/spec.md"),
            "# Capability: suite\n\n#### Scenario: Quality check\n- **WHEN** it runs\n- **THEN** it reports quality\n",
        )
        .unwrap();
        let runner_path = repo.join(format!("{test_type}-runner.sh"));
        write_executable(&runner_path, runner_body);
        fs::write(
            repo.join(".espectacular/config.toml"),
            format!(
                "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n{test_type} = [\"{runner}\"]\n",
                runner = runner_path.display()
            ),
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/suite/quality-check.toml"),
            format!(
                "id = \"quality-check\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.{test_type}]]\nflags = \"all\"\n"
            ),
        )
        .unwrap();
        dir
    }

    // 8.5 Red: property/snapshot test entries emit quality findings
    //
    // Quality tests are combined into one function to avoid parallel execution
    // interference — these tests create temp dirs with runner scripts that can
    // collide when run concurrently (see espectacular-9q4).

    fn mutation_failure_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs")).unwrap();
        fs::create_dir_all(repo.join(".espectacular")).unwrap();
        let script = repo.join("mutation-tool.sh");
        write_executable(&script, "exit 1");
        fs::write(
            repo.join(".espectacular/config.toml"),
            format!(
                "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n\n[quality.mutation]\nenabled = true\nthreshold = 0.80\ncommand = [\"/bin/sh\", \"{}\"]\n",
                script.display()
            ),
        )
        .unwrap();
        dir
    }

    #[test]
    fn quality_tests_run_sequentially() {
        // property passing
        let dir = quality_test_repo("property", "exit 0");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            output
                .quality_findings
                .iter()
                .any(|f| f.kind == "quality-property"),
            "property passing: expected quality-property finding; got: {:?}",
            output.quality_findings
        );
        assert!(
            output.findings.is_empty(),
            "property passing: expected no regular findings"
        );

        // property failing
        let dir = quality_test_repo("property", "exit 1");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "test-failing"),
            "property failing: expected test-failing finding"
        );
        assert!(
            output.quality_findings.is_empty(),
            "property failing: expected no quality findings on failure"
        );

        // snapshot passing
        let dir = quality_test_repo("snapshot", "exit 0");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            output
                .quality_findings
                .iter()
                .any(|f| f.kind == "quality-snapshot"),
            "snapshot passing: expected quality-snapshot finding; got: {:?}",
            output.quality_findings
        );
        assert!(
            output.findings.is_empty(),
            "snapshot passing: expected no regular findings"
        );

        // snapshot failing
        let dir = quality_test_repo("snapshot", "exit 1");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "test-failing"),
            "snapshot failing: expected test-failing finding"
        );
        assert!(
            output.quality_findings.is_empty(),
            "snapshot failing: expected no quality findings on failure"
        );

        // mutation tool failure emits test-failing
        let dir = mutation_failure_repo();
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "test-failing"),
            "mutation tool failure: expected test-failing finding; got findings: {:?}",
            output.findings
        );

        // mutation failure does not produce quality findings
        let dir = mutation_failure_repo();
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            output.quality_findings.is_empty(),
            "mutation failure: must not appear as advisory quality finding"
        );
    }

    fn shell_check_repo(runner_body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/app")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/app")).unwrap();
        fs::write(
            repo.join("openspec/specs/app/spec.md"),
            "# Capability: app\n\n#### Scenario: Shell check\n- **WHEN** the command runs\n- **THEN** tests pass\n",
        ).unwrap();
        fs::write(
            repo.join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
        ).unwrap();
        let script = repo.join("check.sh");
        write_executable(&script, runner_body);
        fs::write(
            repo.join(".espectacular/app/shell-check.toml"),
            format!(
                "id = \"shell-check\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.shell]]\ncommand = \"/bin/sh {}\"\n",
                script.display()
            ),
        ).unwrap();
        dir
    }

    #[test]
    fn shell_zero_tests_ran_emits_no_tests_ran_finding() {
        let dir = shell_check_repo(
            "printf 'test result: ok. 0 passed; 0 failed; 0 ignored; 5 filtered out\\n'",
        );
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "no-tests-ran"),
            "expected no-tests-ran finding; got: {:?}",
            output.findings
        );
        assert_eq!(output.summary.passed, 0);
    }

    #[test]
    fn shell_with_passing_tests_does_not_emit_no_tests_ran() {
        let dir = shell_check_repo("printf 'test result: ok. 3 passed; 0 failed\\n'");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            !output.findings.iter().any(|f| f.kind == "no-tests-ran"),
            "unexpected no-tests-ran finding"
        );
        assert_eq!(output.summary.passed, 1);
    }

    #[test]
    fn shell_multiple_binaries_no_false_positive_no_tests_ran() {
        let dir = shell_check_repo(
            "printf 'test result: ok. 3 passed; 0 failed\\ntest result: ok. 0 passed; 0 failed\\n'",
        );
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            !output.findings.iter().any(|f| f.kind == "no-tests-ran"),
            "multi-binary output with 3 passed then 0 passed must not emit no-tests-ran; got: {:?}",
            output.findings
        );
        assert_eq!(output.summary.passed, 1);
    }

    #[test]
    fn shell_non_cargo_output_exit_zero_still_passes() {
        let dir = shell_check_repo("exit 0");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            !output.findings.iter().any(|f| f.kind == "no-tests-ran"),
            "plain exit 0 must not emit no-tests-ran"
        );
        assert_eq!(output.summary.passed, 1);
    }

    fn cargo_check_repo(cargo_shim_body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/app")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/app")).unwrap();
        fs::write(
            repo.join("openspec/specs/app/spec.md"),
            "# Capability: app\n\n#### Scenario: Cargo token\n- **WHEN** the token is checked\n- **THEN** tests pass\n",
        ).unwrap();
        fs::write(
            repo.join(".espectacular/config.toml"),
            format!("tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncargo = [\"{}\"]\n", repo.join("cargo").display()),
        ).unwrap();
        write_executable(&repo.join("cargo"), cargo_shim_body);
        fs::write(
            repo.join(".espectacular/app/cargo-token.toml"),
            "id = \"cargo-token\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.cargo]]\nflags = \"p_token::case1\"\n",
        ).unwrap();
        dir
    }

    // Task 4.3 (derive-contracts-from-specodelic): a `flags` binding whose
    // runner matched zero tests must fail the gate instead of passing —
    // `cargo test -- nonexistent-filter` exits 0 with "0 passed".
    #[test]
    fn cargo_flags_binding_matching_zero_tests_emits_no_tests_ran() {
        let dir =
            cargo_check_repo("printf 'test result: ok. 0 passed; 1 filtered out; 0 finished\\n'");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            output.findings.iter().any(|f| f.kind == "no-tests-ran"),
            "expected no-tests-ran for zero-matched cargo binding; got: {:?}",
            output.findings
        );
        assert_eq!(output.summary.passed, 0);
    }

    #[test]
    fn cargo_flags_binding_with_passes_still_counts_passed() {
        let dir = cargo_check_repo("printf 'test result: ok. 2 passed; 0 failed\\n'");
        let output = run_check(dir.path(), &[], true).unwrap();
        assert!(
            !output.findings.iter().any(|f| f.kind == "no-tests-ran"),
            "unexpected no-tests-ran for positive cargo output"
        );
        assert_eq!(output.summary.passed, 1);
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

        let output = run_check(dir.path(), &["add-parser".to_string()], true).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "missing-replacement"));
    }

    #[test]
    fn configured_specs_path_used_in_structural_findings() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        // Use a non-default specs root
        fs::create_dir_all(repo.join("myspecs/compiler")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
        fs::write(
            repo.join("myspecs/compiler/spec.md"),
            "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n",
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"myspecs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n",
        )
        .unwrap();
        // No contract for this scenario — triggers a no-toml structural finding
        write_executable(&repo.join("runner.sh"), "exit 0");

        let output = run_check(dir.path(), &[], false).unwrap();
        let no_toml = output
            .findings
            .iter()
            .find(|f| f.kind == "no-toml")
            .expect("expected no-toml finding");
        assert!(
            no_toml.spec_path.contains("myspecs/"),
            "spec_path should use configured specs root 'myspecs', got: {}",
            no_toml.spec_path
        );
        assert!(
            !no_toml.spec_path.contains("openspec/specs"),
            "spec_path should not contain default 'openspec/specs', got: {}",
            no_toml.spec_path
        );
    }
    // ---- property-derived contract bindings (DDL migration, design D6) ----

    #[test]
    fn p_correspondence() {
        missing_contract_finding_present();
        orphan_contract_finding_present();
        findings_carry_action_fields_and_scenario_prose();
    }

    #[test]
    fn p_schema() {
        invalid_falsifiability_class_emits_structural_finding();
        invalid_falsifiability_class_blocks_scenario_execution();
        liveness_without_timeout_emits_warning_severity();
        liveness_with_timeout_emits_no_warning();
        structural_findings_carry_error_severity();
        contract_stale_emitted_when_row_hash_differs();
    }

    #[test]
    fn p_json() {
        run_check_reports_success_with_empty_findings();
        findings_ordered_by_spec_scenario_kind();
        four_findings_fixture_emits_exactly_four();
        findings_carry_action_fields_and_scenario_prose();
    }

    #[test]
    fn p_overlay() {
        run_check_with_change_adds_scenarios_to_scope();
        run_check_reports_overlay_conflict_for_duplicate_added_scenarios();
        run_check_overlay_modification_with_staged_contract_passes();
        run_check_unsignaled_scenario_redefinition_still_conflicts();
        run_check_conflicting_scenario_modifications_across_changes();
        staged_superseded_contract_requires_replacement_in_scope();
    }

    // ---- test selection (espectacular-0k8, Layer 1) ----

    /// Layer 1 selection (espectacular-0k8): with an explicit changed-file
    /// set, only the capability owning the changed spec file runs its
    /// contract tests; the untouched capability's contract tests are
    /// skipped, and the selection is reported in the JSON output.
    #[test]
    fn selection_changed_spec_file_runs_only_affected_capability() {
        let dir = two_capability_repo();

        // sanity: without a changed-file set, both run (current behavior)
        let output = run_check_with_selection(dir.path(), &[], true, &[], false).unwrap();
        assert_eq!(output.summary.passed, 2);

        let output = run_check_with_selection(
            dir.path(),
            &[],
            true,
            &["openspec/specs/compiler/spec.md".to_string()],
            false,
        )
        .unwrap();
        let selection = output
            .selection
            .as_ref()
            .expect("selection must be reported");
        assert_eq!(selection.selected.len(), 1, "one capability selected");
        assert_eq!(selection.selected[0], "compiler");
        assert_eq!(selection.skipped, 1, "untouched capability skipped");
        assert_eq!(selection.source, "spec-provenance");
        assert_eq!(selection.unmapped_files, 0, "spec file maps directly");
        assert_eq!(output.summary.passed, 1, "only the affected contract ran");
    }

    /// No changed-file set (or empty diff) means no selection signal —
    /// everything runs, selection reporting is omitted.
    #[test]
    fn selection_absent_changed_files_run_everything() {
        let dir = two_capability_repo();
        let output = run_check_with_selection(dir.path(), &[], true, &[], false).unwrap();
        assert!(
            output.selection.is_none(),
            "no selection without changed files"
        );
        assert_eq!(output.summary.passed, 2);
    }

    /// Escape hatch (espectacular-0k8): --all-tests must bypass selection
    /// and run every declared contract test even when a changed-file set
    /// is supplied.
    #[test]
    fn all_tests_reports_all_tests_source() {
        let dir = two_capability_repo();
        let output = run_check_with_selection(
            dir.path(),
            &[],
            true,
            &["openspec/specs/compiler/spec.md".to_string()],
            true,
        )
        .unwrap();
        let selection = output
            .selection
            .as_ref()
            .expect("--all-tests must report its bypass");
        assert_eq!(selection.source, "all-tests");
        assert_eq!(selection.skipped, 0, "everything ran");
        assert_eq!(output.summary.passed, 2);
    }

    /// A code file (neither spec nor contract) in the changed set with no
    /// testaruda store must NOT deselect anything — conservative fallback
    /// avoids false greens (espectacular-0k8 exit-20 correction).
    #[test]
    fn code_only_change_without_store_runs_everything() {
        let dir = two_capability_repo();
        let output =
            run_check_with_selection(dir.path(), &[], true, &["src/parser.rs".to_string()], false)
                .unwrap();
        assert_eq!(output.summary.passed, 2, "conservative: all contracts run");
    }

    /// Zero-overlap guard (review F1): a changed set that maps to
    /// capabilities owning NO in-scope scenario (e.g. .espectacular/
    /// config.toml -> bogus "config") must NOT deselect everything — the
    /// run is conservative (all pass through) and the bypass is reported
    /// in the JSON, never a hidden green. unmapped_files counts changed
    /// files that map to no capability at all (review F9).
    #[test]
    fn selection_zero_overlap_reports_bypassed_runs_everything() {
        let dir = two_capability_repo();
        let output = run_check_with_selection(
            dir.path(),
            &[],
            true,
            &[
                ".espectacular/config.toml".to_string(),
                "src/parser.rs".to_string(),
            ],
            false,
        )
        .unwrap();
        let selection = output
            .selection
            .as_ref()
            .expect("bypass must be reported, not silent");
        assert_eq!(selection.source, "bypassed");
        assert!(selection.selected.is_empty(), "nothing was selected");
        assert_eq!(selection.skipped, 0, "everything ran");
        assert_eq!(
            selection.unmapped_files, 1,
            "src/parser.rs maps to no capability"
        );
        assert_eq!(output.summary.passed, 2, "conservative: all contracts ran");
    }

    fn two_capability_repo() -> tempfile::TempDir {
        let dir = success_repo();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/parser")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/parser")).unwrap();
        fs::write(
            repo.join("openspec/specs/parser/spec.md"),
            "# Capability: parser\n\n#### Scenario: Parse path\n- **WHEN** it parses\n- **THEN** it passes\n",
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/parser/parse-path.toml"),
            "id = \"parse-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
        )
        .unwrap();
        dir
    }

    /// Overlay mapping (review F3): a changed file under
    /// openspec/changes/<change>/specs/<capability>/ must select the
    /// owning capability — change-overlay spec edits previously selected
    /// nothing.
    #[test]
    fn selection_change_overlay_spec_file_selects_capability() {
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

        let output = run_check_with_selection(
            dir.path(),
            &["add-parser".to_string()],
            true,
            &["openspec/changes/add-parser/specs/compiler/spec.md".to_string()],
            false,
        )
        .unwrap();
        let selection = output
            .selection
            .as_ref()
            .expect("overlay spec edit must select its capability");
        assert_eq!(selection.selected, vec!["compiler"]);
        assert_eq!(output.summary.passed, 2, "deployed + staged contract ran");
    }

    /// Selection is property-backed (review F6): selection reporting,
    /// conservative fallbacks, and the escape hatch all carry the p_ group.
    #[test]
    fn p_selection() {
        selection_changed_spec_file_runs_only_affected_capability();
        selection_absent_changed_files_run_everything();
        all_tests_reports_all_tests_source();
        selection_zero_overlap_reports_bypassed_runs_everything();
        selection_change_overlay_spec_file_selects_capability();
        code_only_change_without_store_runs_everything();
    }

    #[test]
    fn p_boundary() {
        findings_carry_action_fields_and_scenario_prose();
    }

    #[test]
    fn p_agent_actions() {
        findings_carry_action_fields_and_scenario_prose();
        findings_ordered_by_spec_scenario_kind();
        four_findings_fixture_emits_exactly_four();
    }

    #[test]
    fn p_apply_command() {
        findings_carry_action_fields_and_scenario_prose();
    }
}
