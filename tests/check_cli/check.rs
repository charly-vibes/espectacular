use assert_cmd::Command;
use serde_json::Value;
use std::fs;

use crate::base_repo;
use crate::data_from_envelope;
use crate::schema::assert_schema_valid;
use crate::write_executable;

#[test]
fn ah_check_success_emits_schema_valid_json() {
    let repo = base_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 2);
    assert_eq!(data["summary"]["counts_by_kind"], serde_json::json!({}));
    assert_eq!(data["scope"]["deployed"], true);
}

#[test]
fn ah_check_failure_emits_execution_details_and_exit_one() {
    let repo = base_repo();
    write_executable(&repo.path().join("runner.sh"), "printf 'boom' >&2\nexit 7");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);

    let findings = data["findings"].as_array().unwrap();
    let failing = findings
        .iter()
        .find(|finding| finding["kind"] == "test-failing")
        .unwrap();
    assert_eq!(failing["category"], "execution");
    assert_eq!(failing["scenario"]["id"], "green-path");
    assert_eq!(failing["suggested_action"], "edit_code_not_scenario");
    assert_eq!(
        failing["playbook_command"],
        "ah explain edit_code_not_scenario"
    );
    assert_eq!(
        failing["scenario_prose"],
        serde_json::json!("- **WHEN** it runs\n- **THEN** it passes")
    );
    assert_eq!(failing["test"]["type"], "unit");
    assert_eq!(failing["test"]["exit_code"], 7);
    assert_eq!(failing["test"]["stderr_tail"], "boom");
    assert_eq!(data["summary"]["counts_by_kind"]["test-failing"], 1);
}

#[test]
pub fn ah_check_warning_only_findings_exit_zero() {
    let repo = base_repo();
    // Tag green-path as liveness with no timeout_seconds: exactly one
    // missing-liveness-timeout warning, nothing gate-failing.
    let contract = repo.path().join(".espectacular/compiler/green-path.toml");
    let text = fs::read_to_string(&contract).unwrap();
    fs::write(
        &contract,
        text.replace(
            "status = \"active\"",
            "status = \"active\"\nfalsifiability_class = \"liveness\"",
        ),
    )
    .unwrap();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    let findings = data["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["kind"], "missing-liveness-timeout");
    assert_eq!(findings[0]["severity"], "warning");
    assert_eq!(findings[0]["category"], "warning");
    assert_eq!(data["summary"]["passed"], 2, "tests still run and pass");
}

#[test]
fn ah_check_with_changes_includes_overlay_scope() {
    let repo = base_repo();
    fs::create_dir_all(
        repo.path()
            .join("openspec/changes/add-parser/specs/compiler"),
    )
    .unwrap();
    fs::create_dir_all(
        repo.path()
            .join(".espectacular/changes/add-parser/compiler"),
    )
    .unwrap();
    fs::write(
        repo.path().join("openspec/changes/add-parser/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Added path\n- **WHEN** overlay applies\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".espectacular/changes/add-parser/compiler/added-path.toml"),
        "id = \"added-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
    )
    .unwrap();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json", "--changes", "add-parser"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    assert_eq!(data["scope"]["changes"], serde_json::json!(["add-parser"]));
    assert_eq!(data["summary"]["passed"], 3);
}

#[test]
fn ah_check_missing_change_has_clear_diagnostic() {
    let repo = base_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--changes", "missing-change"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "change 'missing-change' does not exist",
        ));
}

#[test]
fn ah_check_human_readable_output_shows_findings() {
    let repo = base_repo();
    write_executable(&repo.path().join("runner.sh"), "printf 'boom' >&2\nexit 7");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests"])
        .assert()
        .failure();

    let stdout = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    assert!(
        stdout.contains("found 1 issue(s)"),
        "expected 'found 1 issue(s)' in output; got: {stdout:?}"
    );
    assert!(
        stdout.contains("execution: compiler/green-path — test-failing"),
        "expected finding line in output; got: {stdout:?}"
    );
    assert!(
        stdout.contains("stderr: boom"),
        "expected stderr in output; got: {stdout:?}"
    );
    assert!(
        stdout.contains("summary:"),
        "expected summary in output; got: {stdout:?}"
    );
    assert!(
        stdout.contains("test-failing: 1"),
        "expected kind count in output; got: {stdout:?}"
    );
}

#[test]
fn p_check() {
    ah_check_success_emits_schema_valid_json();
    ah_check_failure_emits_execution_details_and_exit_one();
    ah_check_warning_only_findings_exit_zero();
    ah_check_with_changes_includes_overlay_scope();
    ah_check_missing_change_has_clear_diagnostic();
}
