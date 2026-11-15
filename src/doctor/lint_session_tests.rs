//! Advisory session suggestions (lint run marker, falsifiability tagging).

use super::lint_marker_path;
use super::runner::run_doctor;
use super::session::record_lint_run;
use super::tests::fixtures::make_healthy_repo;
use super::types::TOOL_VERSION;
use std::fs;
use tempfile::TempDir;

// espectacular-eia (task 5.1): doctor suggests `ah lint` when no lint
// run has completed in the session. Advisory-only — never a gate blocker.

#[test]
fn doctor_suggests_lint_when_no_lint_run_completed() {
    let repo = make_healthy_repo();
    let outcome = run_doctor(repo.path()).unwrap();
    let sugg = outcome
        .session_suggestions
        .iter()
        .find(|s| s.apply_command == "ah lint");
    assert!(sugg.is_some(), "expected an ah lint suggestion");
    assert!(sugg.unwrap().detail.contains("lint"));
}

#[test]
fn doctor_stays_silent_after_lint_run_completed() {
    let repo = make_healthy_repo();
    record_lint_run(repo.path());
    let outcome = run_doctor(repo.path()).unwrap();
    assert!(
        !outcome
            .session_suggestions
            .iter()
            .any(|s| s.apply_command == "ah lint"),
        "lint suggestion must disappear once a lint run completed"
    );
}

#[test]
fn doctor_suggests_falsifiability_tagging_for_eventually_scenarios() {
    let repo = make_healthy_repo();
    let root = repo.path();
    fs::create_dir_all(root.join("openspec/specs/queue")).unwrap();
    fs::create_dir_all(root.join(".espectacular/queue")).unwrap();
    fs::write(
        root.join("openspec/specs/queue/spec.md"),
        "# Capability: queue\n\n#### Scenario: Request settles\n- **WHEN** a request is submitted\n- **THEN** it eventually settles\n",
    )
    .unwrap();
    fs::write(
        root.join(".espectacular/queue/request-settles.toml"),
        format!(
            "id = \"request-settles\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"{}\"\n\n[[tests.unit]]\nflags = \"t::f\"\n",
            TOOL_VERSION
        ),
    )
    .unwrap();

    let outcome = run_doctor(root).unwrap();
    let sugg = outcome
        .session_suggestions
        .iter()
        .find(|s| s.apply_command == "ah explain missing-liveness-timeout");
    assert!(
        sugg.is_some(),
        "expected a falsifiability tagging suggestion"
    );
    assert!(sugg.unwrap().detail.contains("eventually"));
}

#[test]
fn doctor_no_falsifiability_suggestion_when_contract_tagged() {
    let repo = make_healthy_repo();
    let root = repo.path();
    fs::create_dir_all(root.join("openspec/specs/queue")).unwrap();
    fs::create_dir_all(root.join(".espectacular/queue")).unwrap();
    fs::write(
        root.join("openspec/specs/queue/spec.md"),
        "# Capability: queue\n\n#### Scenario: Request settles\n- **WHEN** a request is submitted\n- **THEN** it eventually settles\n",
    )
    .unwrap();
    fs::write(
        root.join(".espectacular/queue/request-settles.toml"),
        format!(
            "id = \"request-settles\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"{}\"\nfalsifiability_class = \"liveness\"\n\n[[tests.unit]]\nflags = \"t::f\"\n",
            TOOL_VERSION
        ),
    )
    .unwrap();

    let outcome = run_doctor(root).unwrap();
    assert!(
        !outcome
            .session_suggestions
            .iter()
            .any(|s| s.apply_command == "ah explain missing-liveness-timeout"),
        "tagged contracts must not trigger the suggestion"
    );
}

#[test]
fn falsifiability_suggestion_never_fails_the_doctor_run() {
    // Anti-goal guard (EXCL-003): a tagging nudge must ride the suggestion
    // path only — genesis maps any LintResult to CheckStatus::Warn, so a
    // finding here would make `ah doctor` exit 1 (mu5/djw foot-gun).
    let repo = make_healthy_repo();
    let root = repo.path();
    fs::create_dir_all(root.join("openspec/specs/queue")).unwrap();
    fs::create_dir_all(root.join(".espectacular/queue")).unwrap();
    fs::write(
        root.join("openspec/specs/queue/spec.md"),
        "# Capability: queue\n\n#### Scenario: Request settles\n- **WHEN** a request is submitted\n- **THEN** it eventually settles\n",
    )
    .unwrap();
    fs::write(
        root.join(".espectacular/queue/request-settles.toml"),
        format!(
            "id = \"request-settles\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"{}\"\n\n[[tests.unit]]\nflags = \"t::f\"\n",
            TOOL_VERSION
        ),
    )
    .unwrap();

    let outcome = run_doctor(root).unwrap();
    assert!(outcome
        .session_suggestions
        .iter()
        .any(|s| s.apply_command == "ah explain missing-liveness-timeout"),);
    assert!(
        outcome.genesis_report.is_healthy(),
        "suggestion must never flip the health verdict"
    );
}

#[test]
fn lint_suggestion_never_fails_the_doctor_run() {
    // Anti-goal guard: a pending lint suggestion must not flip the
    // genesis health verdict.
    let repo = make_healthy_repo();
    let outcome = run_doctor(repo.path()).unwrap();
    assert!(outcome.genesis_report.is_healthy());
    assert!(!outcome.session_suggestions.is_empty());
}

#[test]
fn record_lint_run_writes_marker_best_effort() {
    let repo = TempDir::new().unwrap();
    // No .espectacular dir → no marker, no error (adoption-friendly).
    record_lint_run(repo.path());
    assert!(!lint_marker_path(repo.path()).exists());
    // Configured repo → marker written.
    fs::create_dir_all(repo.path().join(".espectacular")).unwrap();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        concat!(
            "tool_version = \"",
            env!("CARGO_PKG_VERSION"),
            "\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n"
        ),
    )
    .unwrap();
    record_lint_run(repo.path());
    assert!(lint_marker_path(repo.path()).exists());
}
