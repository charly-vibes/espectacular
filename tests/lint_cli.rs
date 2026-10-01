//! Integration tests for the `ah lint` subcommand (tracer-bullet scope:
//! routing, clean-fixture exit code, JSON envelope shape).

use assert_cmd::Command;
use serde_json::Value;

fn ah_lint(args: &[&str]) -> std::process::Output {
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["lint"])
        .args(args)
        .output()
        .unwrap()
}

fn envelope_data(stdout: &[u8]) -> Value {
    let v: Value = serde_json::from_slice(stdout).expect("valid JSON envelope");
    v["data"].clone()
}
#[test]
fn ah_lint_clean_fixture_exits_zero_with_empty_findings() {
    let output = ah_lint(&["tests/fixtures/lint/clean", "--json"]);
    assert!(
        output.status.success(),
        "clean fixture must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data = envelope_data(&output.stdout);
    assert_eq!(data["findings"].as_array().unwrap().len(), 0);
}
#[test]
fn ah_lint_clean_fixture_human_output_exits_zero() {
    let output = ah_lint(&["tests/fixtures/lint/clean"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0 findings"), "stdout: {stdout}");
}
#[test]
fn ah_lint_without_root_lints_deployed_specs() {
    // From the espectacular repo itself (config present): deployed specs are
    // linted; the registry is empty at tracer-bullet scope so zero findings.
    let output = ah_lint(&["--json"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data = envelope_data(&output.stdout);
    assert!(data["findings"].is_array());
}
#[test]
fn ah_lint_json_finding_shape_matches_shared_schema() {
    // The shape contract: any emitted finding carries the shared schema
    // fields. With an empty registry the array is empty, so assert the
    // container shape here; field-level shape is covered by the unit test
    // `lint_finding_serializes_shared_schema_fields`.
    let output = ah_lint(&["tests/fixtures/lint/clean", "--json"]);
    let data = envelope_data(&output.stdout);
    assert!(data["findings"].is_array());
    assert!(data["counts_by_kind"].is_object());
}
#[test]
fn ah_lint_defective_fixture_reports_all_scenario_flow_kinds_exits_zero() {
    // es espectacular-rty: the four scenario-flow checks fire end-to-end,
    // all findings stay warning severity, and the command still exits 0
    // (advisory-only in v1 — exit non-zero is reserved for error severity).
    let output = ah_lint(&["tests/fixtures/lint/defective", "--json"]);
    assert!(
        output.status.success(),
        "warning-only findings must not change the exit code, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data = envelope_data(&output.stdout);
    let findings = data["findings"].as_array().unwrap();
    assert!(!findings.is_empty());
    let kinds: Vec<&str> = findings
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    for kind in [
        "vague-qualifier",
        "imperative-step",
        "conjunctive-bloat",
        "missing-negative-scenario",
    ] {
        assert!(kinds.contains(&kind), "missing kind {kind} in {kinds:?}");
    }
    assert!(findings.iter().all(|f| f["severity"] == "warning"));
    for field in [
        "kind",
        "severity",
        "spec_path",
        "scenario_id",
        "message",
        "suggestion",
        "suggested_action",
        "playbook_command",
    ] {
        assert!(findings[0].get(field).is_some(), "finding missing {field}");
    }
    assert!(data["counts_by_kind"]["imperative-step"].as_u64().unwrap() >= 1);
}
#[test]
fn ah_lint_changes_overlay_lints_change_specs_in_addition_to_deployed() {
    // espectacular-6fp (task 4.2): --changes <id> analyzes deployed specs
    // plus the change overlay. The add-parser overlay introduces a "parser"
    // capability that only exists in the change dir — its findings prove the
    // overlay was walked without disturbing deployed findings.
    let output = ah_lint(&[
        "tests/fixtures/lint/defective",
        "--changes",
        "add-parser",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "warning-only findings must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data = envelope_data(&output.stdout);
    let findings = data["findings"].as_array().unwrap();
    assert!(
        findings.iter().any(|f| f["spec_path"] == "parser"),
        "overlay spec not linted: {findings:?}"
    );
    // Deployed findings still present.
    assert!(findings.iter().any(|f| f["spec_path"] == "ui"));
}
#[test]
fn ah_lint_changes_unknown_change_id_fails() {
    let output = ah_lint(&[
        "tests/fixtures/lint/defective",
        "--changes",
        "no-such-change",
    ]);
    assert!(!output.status.success(), "unknown change id must fail");
}
#[test]
fn ah_lint_check_filter_runs_single_category() {
    // espectacular-6fp (task 4.4): --check <kind> runs only that check.
    let output = ah_lint(&[
        "tests/fixtures/lint/defective",
        "--check",
        "vague-qualifier",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data = envelope_data(&output.stdout);
    let findings = data["findings"].as_array().unwrap();
    assert!(
        !findings.is_empty(),
        "vague-qualifier must fire on defective fixture"
    );
    assert!(
        findings.iter().all(|f| f["kind"] == "vague-qualifier"),
        "filter leaked other kinds: {findings:?}"
    );
}
#[test]
fn ah_lint_check_unknown_kind_fails_with_valid_kinds() {
    let output = ah_lint(&["tests/fixtures/lint/defective", "--check", "no-such-check"]);
    assert!(!output.status.success(), "unknown check kind must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("vague-qualifier"),
        "stderr must list valid kinds: {stderr}"
    );
}

const SPK_SHIM_ENVELOPE: &str = r#"#!/bin/sh
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"files_linted":1,"issues":[{"file":"spec.md","message":"intent statement has no imperative","rule_id":"linter.ears_syntax","rule_semantics":"statement must match an EARS pattern"}]},"warnings":[],"hints":[],"meta":{}}'
"#;

fn spk_shim_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spk");
    std::fs::write(&path, SPK_SHIM_ENVELOPE).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    dir
}
#[test]
fn ah_lint_relays_spk_findings_for_dual_format_fixtures() {
    // espectacular-zlw: the bridge invokes spk for dual-format files and
    // relays each issue as kind = "spk.<rule_id>" at warning severity.
    let shim = spk_shim_dir();
    let path_env = std::env::join_paths(
        std::env::split_paths(&std::env::var("PATH").unwrap()).chain([shim.path().to_path_buf()]),
    )
    .unwrap();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("PATH", &path_env)
        .args(["lint", "tests/fixtures/lint/dual-format", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "warning-only findings must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data = envelope_data(&output.stdout);
    let findings = data["findings"].as_array().unwrap();
    let relayed: Vec<&Value> = findings
        .iter()
        .filter(|f| f["kind"].as_str().unwrap().starts_with("spk."))
        .collect();
    assert!(
        relayed
            .iter()
            .any(|f| f["kind"] == "spk.linter.ears_syntax"),
        "relayed spk finding missing: {findings:?}"
    );
    assert!(relayed.iter().all(|f| f["severity"] == "warning"));
    // The prose checks still fire on the same dual-format file.
    assert!(findings.iter().any(|f| f["kind"] == "vague-qualifier"));
}
#[test]
fn ah_lint_bridge_is_inert_for_plain_openspec_fixtures() {
    // Plain fixtures produce no spk.* and no advisory bridge findings —
    // output identical to pre-bridge behavior.
    let output = ah_lint(&["tests/fixtures/lint/clean", "--json"]);
    let data = envelope_data(&output.stdout);
    let findings = data["findings"].as_array().unwrap();
    assert!(findings.is_empty());
}

#[test]
fn p_lint() {
    ah_lint_clean_fixture_exits_zero_with_empty_findings();
    ah_lint_defective_fixture_reports_all_scenario_flow_kinds_exits_zero();
    ah_lint_changes_overlay_lints_change_specs_in_addition_to_deployed();
    ah_lint_check_filter_runs_single_category();
    ah_lint_json_finding_shape_matches_shared_schema();
}
