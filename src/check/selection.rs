//! Test-selection layers (espectacular-0k8): which capability contracts run.
//!
//! Purpose: map a changed-file set to the capabilities it touches (Layer 1,
//! spec/contract provenance) and consult testaruda's dependency selection to
//! prune individual bindings (Layer 2).
//!
//! Responsibilities: changed-file → capability mapping, the conservative
//! run-all guards (all-tests, zero-overlap, unmapped files), the testaruda
//! `select --agent` invocation and its conservative parse, the prunable
//! filter predicate, and the selection report attached to `CheckOutput`.
//!
//! Rationale: every failure path is conservative — a selection bug must
//! never produce a hidden green, so any unknown degrades to running
//! everything and reporting the bypass.

use super::scope::ResolvedScenario;
use crate::config;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

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

/// Layer 1 selection state (espectacular-0k8): which capability contracts
/// the run narrows to, or a conservative run-all whose bypass is reported.
pub(crate) enum SelectionState {
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
    Caps(HashSet<String>),
}

/// Layer 2 selection facts parsed from `testaruda select --agent`.
pub(crate) struct TestarudaSelection {
    /// Selected test node_ids (includes always_run fallbacks — testaruda's
    /// own conservative over-approximation flows through).
    pub(crate) selected: Vec<String>,
    /// EMPTY selection (exit 20): no code affected. Refinement is skipped,
    /// the Layer 1 set still runs (spec change must verify own contracts).
    pub(crate) empty: bool,
    /// A changed file testaruda could not resolve to a content unit —
    /// its dependency view is incomplete, so pruning is unsafe.
    pub(crate) any_unresolved: bool,
}

/// Resolve the Layer 1 selection state for this run: `None` without a
/// changed-file set, a reported conservative run-all when the escape hatch
/// or a bypass applies, capability narrowing otherwise.
pub(crate) fn resolve_selection_state(
    cfg: &config::Config,
    scenarios: &[ResolvedScenario],
    changed_files: &[String],
    run_tests: bool,
    all_tests: bool,
) -> SelectionState {
    if !run_tests || changed_files.is_empty() {
        return SelectionState::None;
    }
    if all_tests {
        return SelectionState::All {
            source: "all-tests",
            unmapped_files: 0,
            selected: all_capabilities(scenarios),
        };
    }
    let (caps, unmapped) = mapped_capabilities(cfg, changed_files);
    // Zero-overlap guard (review F1/F9): selecting nothing, or selecting
    // capabilities that own no in-scope scenario, must not deselect
    // everything silently.
    let zero_overlap = caps.is_empty()
        || !scenarios
            .iter()
            .any(|scenario| caps.contains(&scenario.scenario.spec_path));
    if unmapped > 0 || zero_overlap {
        return SelectionState::All {
            source: "bypassed",
            unmapped_files: unmapped,
            selected: Vec::new(),
        };
    }
    SelectionState::Caps(caps)
}

/// Every in-scope capability, sorted and deduped (the --all-tests report).
fn all_capabilities(scenarios: &[ResolvedScenario]) -> Vec<String> {
    let mut caps: Vec<String> = scenarios
        .iter()
        .map(|scenario| scenario.scenario.spec_path.clone())
        .collect();
    caps.sort();
    caps.dedup();
    caps
}

/// Map each changed file to its capability; count the files that map to none.
fn mapped_capabilities(cfg: &config::Config, changed_files: &[String]) -> (HashSet<String>, usize) {
    let mut caps = HashSet::new();
    let mut unmapped = 0usize;
    for file in changed_files {
        match affected_capability(file, &cfg.paths.specs, &cfg.paths.changes) {
            Some(cap) => {
                caps.insert(cap);
            }
            None => unmapped += 1,
        }
    }
    (caps, unmapped)
}

/// Build the serialized selection report; `None` when nothing was selected.
pub(crate) fn selection_report(
    selection_state: &SelectionState,
    skipped: usize,
    testaruda_pruned: usize,
    testaruda_consulted: bool,
) -> Option<Selection> {
    let testaruda = testaruda_consulted.then_some(TestarudaRefinement {
        pruned: testaruda_pruned,
    });
    match selection_state {
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
            testaruda,
        }),
        SelectionState::Caps(caps) => Some(Selection {
            selected: sorted_capabilities(caps),
            skipped,
            source: "spec-provenance".to_string(),
            unmapped_files: 0,
            testaruda,
        }),
    }
}

fn sorted_capabilities(caps: &HashSet<String>) -> Vec<String> {
    let mut sorted: Vec<String> = caps.iter().cloned().collect();
    sorted.sort();
    sorted
}

pub(crate) fn testaruda_store_present(repo_root: &Path) -> bool {
    repo_root.join(".testaruda/store.db").is_file()
}

/// Run `testaruda select --agent --files <changed>` in the repo. Returns
/// None when the store is absent, the binary is missing, or the output
/// cannot be parsed — every failure path is conservative (no pruning).
pub(crate) fn testaruda_select(
    repo_root: &Path,
    changed_files: &[String],
) -> Option<TestarudaSelection> {
    let output = run_testaruda_select(repo_root, changed_files)?;
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let selected = parse_selected(&value)?;
    Some(TestarudaSelection {
        selected,
        empty: parse_selected_count(&value) == 0,
        any_unresolved: parse_any_unresolved(&value),
    })
}

fn run_testaruda_select(
    repo_root: &Path,
    changed_files: &[String],
) -> Option<std::process::Output> {
    use std::process::Stdio;
    std::process::Command::new("testaruda")
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
        .ok()
}

/// Selected node ids; `None` when the envelope carries no `selected` array
/// (unparseable selection → no pruning).
fn parse_selected(value: &serde_json::Value) -> Option<Vec<String>> {
    Some(
        value
            .get("selected")?
            .as_array()?
            .iter()
            .filter_map(|test| {
                test.get("node_id")
                    .and_then(|node| node.as_str())
                    .map(str::to_string)
            })
            .collect(),
    )
}

fn parse_selected_count(value: &serde_json::Value) -> u64 {
    value
        .get("summary")
        .and_then(|summary| summary.get("selected_count"))
        .and_then(|count| count.as_u64())
        .unwrap_or(0)
}

fn parse_any_unresolved(value: &serde_json::Value) -> bool {
    value
        .get("changed_units")
        .and_then(|units| units.as_array())
        .is_some_and(|units| {
            units.iter().any(|unit| {
                unit.get("unresolved")
                    .and_then(|flag| flag.as_bool())
                    .unwrap_or(false)
            })
        })
}

/// The runner filter a contract binding injects, if it is prunable at all.
/// Shell commands are opaque (no filter into a test runner) and never
/// pruned; bindings without a non-empty flags filter are never pruned.
pub(crate) fn prunable_filter<'a>(
    test_type: &str,
    entry: &'a crate::contracts::TestEntry,
) -> Option<&'a str> {
    if test_type == "shell" {
        return None;
    }
    entry.flags.as_deref().filter(|filter| !filter.is_empty())
}

pub(crate) fn node_contains_filter(selected: &[String], filter: &str) -> bool {
    selected.iter().any(|node| node.contains(filter))
}

/// Map a changed file (repo-relative) to the capability it belongs to,
/// via either the deployed specs tree (`<specs>/<capability>/...`), the
/// change-overlay specs tree (`<changes>/<change>/specs/<capability>/...`,
/// review F4: overlay spec edits previously selected nothing), or the
/// contracts tree (`.espectacular/<capability>/...`, including change
/// overlays `.espectacular/changes/<change>/<capability>/...`).
/// Everything else (agent code, change proposal/tasks files) maps to no
/// capability.
pub(crate) fn affected_capability(
    file: &str,
    specs_dir: &str,
    changes_dir: &str,
) -> Option<String> {
    spec_tree_capability(file, specs_dir)
        .or_else(|| overlay_spec_capability(file, changes_dir))
        .or_else(|| contracts_tree_capability(file))
}

/// deployed specs tree: `<specs_dir>/<capability>/...`
fn spec_tree_capability(file: &str, specs_dir: &str) -> Option<String> {
    let prefix = format!("{}/", specs_dir.trim_end_matches('/'));
    first_segment(file.strip_prefix(prefix.as_str())?)
}

/// change-overlay specs tree: `<changes_dir>/<change>/specs/<cap>/...`
/// Only files under `specs/` select a capability; change proposal, tasks
/// and design files select nothing.
fn overlay_spec_capability(file: &str, changes_dir: &str) -> Option<String> {
    let prefix = format!("{}/", changes_dir.trim_end_matches('/'));
    let mut segments = file.strip_prefix(prefix.as_str())?.split('/');
    let _change = segments.next();
    if segments.next() != Some("specs") {
        return None;
    }
    non_empty_segment(segments.next())
}

/// contracts tree: `.espectacular/<capability>/...`
/// staged contracts: `.espectacular/changes/<change>/<capability>/...`
fn contracts_tree_capability(file: &str) -> Option<String> {
    let rest = file.strip_prefix(".espectacular/")?;
    if let Some(staged) = rest.strip_prefix("changes/") {
        let mut segments = staged.split('/');
        let _change = segments.next();
        return non_empty_segment(segments.next());
    }
    first_segment(rest)
}

/// First non-empty path segment, mapped to a capability name.
fn first_segment(rest: &str) -> Option<String> {
    non_empty_segment(rest.split('/').next())
}

fn non_empty_segment(segment: Option<&str>) -> Option<String> {
    segment
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
pub(crate) mod tests;
