//! Session-scoped advisory suggestions (lint run, falsifiability tagging).

use super::types::SessionSuggestion;
use std::path::Path;

/// Marker recording a completed lint run. Its absence is the doctor's signal
/// that no lint run has completed in the session — a suggested step, never a
/// gate blocker.
pub fn lint_marker_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".espectacular/state/lint-last-run")
}

/// Record a completed lint run. Best-effort: only configured repos get the
/// marker (fixture and adopting repos stay untouched), and write failures
/// never fail the lint command.
pub fn record_lint_run(repo_root: &Path) {
    if !repo_root.join(crate::config::CONFIG_MARKER).exists() {
        return;
    }
    let marker = lint_marker_path(repo_root);
    if let Some(parent) = marker.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&marker, chrono_now_iso());
}

/// Session timestamp (no chrono dependency in this tool — UTC from unix time).
fn chrono_now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("lint-run unix_ts={secs}")
}

/// Suggest `ah lint` when the repo is configured but no lint run has
/// completed in the session. Advisory-only by construction: it lands in
/// `session_suggestions`, which never influences the health verdict.
pub(super) fn falsifiability_session_suggestion(
    repo_root: &Path,
    config: &crate::config::Config,
) -> Option<SessionSuggestion> {
    let specs_dir = repo_root.join(&config.paths.specs);
    let scenarios =
        crate::openspec::discover_scenarios(specs_dir.to_str().unwrap_or_default()).ok()?;
    let contracts_root = repo_root.join(".espectacular");
    let mut candidates: Vec<String> = Vec::new();
    for scenario in &scenarios {
        if !scenario.body.to_lowercase().contains("eventually") {
            continue;
        }
        let path = crate::check::contract_path(&contracts_root, &scenario.spec_path, &scenario.id);
        let Ok(contract) = crate::contracts::load_contract(path.to_str().unwrap_or_default())
        else {
            continue;
        };
        if contract.falsifiability_class.is_empty() {
            candidates.push(format!("{}/{}", scenario.spec_path, scenario.id));
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let first = &candidates[0];
    Some(SessionSuggestion {
        detail: format!(
            "{} scenario(s) containing \"eventually\" lack a falsifiability_class tag (e.g. {first}) — consider tagging safety claims \"safety\" and bounded liveness claims \"liveness\"",            candidates.len()
        ),
        apply_command: "ah explain missing-liveness-timeout".to_string(),
    })
}

pub(super) fn lint_session_suggestion(repo_root: &Path) -> Option<SessionSuggestion> {
    if !repo_root.join(crate::config::CONFIG_MARKER).exists() {
        return None;
    }
    if lint_marker_path(repo_root).exists() {
        return None;
    }
    Some(SessionSuggestion {
        detail:
            "no lint run has completed in this session — surface authoring-quality findings early"
                .to_string(),
        apply_command: "ah lint".to_string(),
    })
}
