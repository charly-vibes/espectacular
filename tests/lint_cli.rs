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
