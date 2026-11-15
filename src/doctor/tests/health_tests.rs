//! Health-verdict diagnostics: config, paths, hooks, managed blocks.

use super::super::runner::run_doctor;
use super::fixtures::{has_issue, issues, make_healthy_repo};
use std::fs;

#[test]
fn healthy_repo_exits_zero_with_no_diagnostics() {
    let repo = make_healthy_repo();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        report.genesis_report.is_healthy(),
        "healthy repo must report healthy=true; diagnostics: {:?}",
        issues(&report)
    );
    assert!(
        issues(&report).is_empty(),
        "healthy repo must have no diagnostics; got: {:?}",
        issues(&report)
    );
}

#[test]
fn bad_config_emits_config_schema_diagnostic() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        "tool_version = \"\"\n[paths]\nspecs = \"\"\nchanges = \"\"\n[runners]\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "config"),
        "bad config must emit config diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn missing_specs_path_emits_missing_paths_diagnostic() {
    let repo = make_healthy_repo();
    fs::remove_dir_all(repo.path().join("openspec/specs")).unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "missing-specs-dir"),
        "missing specs dir must emit missing-specs-dir diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn missing_changes_path_emits_missing_paths_diagnostic() {
    let repo = make_healthy_repo();
    fs::remove_dir_all(repo.path().join("openspec/changes")).unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "missing-changes-dir"),
        "missing changes dir must emit missing-changes-dir diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn version_drift_emits_version_drift_diagnostic() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        r#"tool_version = "0.0.1"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
"#,
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "version-drift"),
        "version mismatch must emit version-drift diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn missing_managed_block_in_agents_md_emits_diagnostic() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join("AGENTS.md"),
        "# Project\n\nNo block here.\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "managed-block"),
        "missing managed block must emit managed-block diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn missing_managed_block_in_claude_md_emits_diagnostic() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join("CLAUDE.md"),
        "# Project\n\nNo block here.\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "managed-block"),
        "missing managed block in CLAUDE.md must emit managed-block diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn hook_absent_emits_hook_absent_diagnostic() {
    let repo = make_healthy_repo();
    fs::remove_file(repo.path().join("lefthook.yml")).unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "hook-framework"),
        "no hook framework must emit hook-framework diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn hook_prek_toml_is_supported_framework() {
    // RED (espectacular-6ye R1): genesis::git_hooks detects `prek.toml`;
    // the legacy detector only knew .prek/prek.yml, so a prek.toml repo
    // wrongly got a hook-framework Error diagnostic.
    let repo = make_healthy_repo();
    fs::remove_file(repo.path().join("lefthook.yml")).unwrap();
    fs::write(repo.path().join("prek.toml"), "[hooks]\n").unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        !has_issue(&report, "hook-framework"),
        "prek.toml repo must not emit hook-framework diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn hook_prek_dotfile_is_no_longer_a_detection_signal() {
    // BEHAVIORAL DRIFT accepted (6ye): genesis detects prek via prek.toml
    // only; the legacy .prek/prek.yml signals are dropped with it.
    let repo = make_healthy_repo();
    fs::remove_file(repo.path().join("lefthook.yml")).unwrap();
    fs::write(repo.path().join(".prek"), "").unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        has_issue(&report, "hook-framework"),
        ".prek alone must not count as a supported framework; got: {:?}",
        issues(&report)
    );
}

#[test]
fn hook_wired_missing_in_prepush_emits_hook_wired_diagnostic() {
    // RED (espectacular-6ye R4): lefthook present but ah check not wired
    // into the pre-push stage → hook-wired diagnostic.
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        has_issue(&report, "hook-wired"),
        "lefthook without pre-push ah check must emit hook-wired; got: {:?}",
        issues(&report)
    );
}

#[test]
fn hook_wired_both_stages_passes() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\npre-push:\n  commands:\n    ah-check:\n      run: ah check\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        !has_issue(&report, "hook-wired"),
        "both stages wired must pass; got: {:?}",
        issues(&report)
    );
}
