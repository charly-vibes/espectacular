use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::data_from_envelope;
use crate::git_commit_all;
use crate::write_executable;

// ── ah report ──────────────────────────────────────────────────────────────────

/// Create the spec and contract directories of one capability and write its
/// spec.
fn report_capability(repo: &Path, capability: &str, spec: &str) {
    fs::create_dir_all(repo.join(format!("openspec/specs/{capability}"))).unwrap();
    fs::create_dir_all(repo.join(format!(".espectacular/{capability}"))).unwrap();
    fs::write(
        repo.join(format!("openspec/specs/{capability}/spec.md")),
        spec,
    )
    .unwrap();
}

/// Write the default report config (one `unit` runner through `runner.sh`).
fn report_config(repo: &Path) {
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n"),
    )
    .unwrap();
}

/// Write one PF contract whose trailing `[[tests.<kind>]]` body is `body`.
fn report_contract(repo: &Path, capability: &str, id: &str, kind: &str, body: &str) {
    fs::write(
        repo.join(format!(".espectacular/{capability}/{id}.toml")),
        format!(
            "id = \"{id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.{kind}]]\n{body}\n"
        ),
    )
    .unwrap();
}

fn report_full_coverage_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    report_capability(
        repo,
        "compiler",
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Shell path\n- **WHEN** shell command runs\n- **THEN** it passes\n",
    );
    report_capability(
        repo,
        "adapters",
        "# Capability: adapters\n\n#### Scenario: Detects cargo\n- **WHEN** cargo exists\n- **THEN** it detects\n",
    );
    report_config(repo);
    // Two specs, both with PF archetype, both covered
    report_contract(repo, "compiler", "green-path", "unit", "flags = \"ok\"");
    report_contract(
        repo,
        "compiler",
        "shell-path",
        "shell",
        "command = \"printf shell\"",
    );
    report_contract(repo, "adapters", "detects-cargo", "unit", "flags = \"ok\"");
    write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");
    git_commit_all(repo);
    dir
}

fn report_missing_contract_repo() -> tempfile::TempDir {
    let dir = report_full_coverage_repo();
    // Remove one contract to create a gap
    fs::remove_file(dir.path().join(".espectacular/compiler/shell-path.toml")).unwrap();
    dir
}

#[test]
fn ah_report_json_emits_matrix_with_coverage_counts() {
    let repo = report_full_coverage_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["report", "--json"])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    let matrix = data["matrix"].as_array().expect("expected matrix array");
    assert!(matrix.len() >= 2, "expected at least 2 specs");

    // Each row: spec, archetype, covered, missing, failing, total
    let compiler_row = matrix
        .iter()
        .find(|r| r["spec"] == "compiler")
        .expect("expected compiler row");
    assert_eq!(compiler_row["covered"], 2);
    assert_eq!(compiler_row["missing"], 0);
    assert_eq!(compiler_row["failing"], 0);
    assert_eq!(compiler_row["total"], 2);

    let adapters_row = matrix
        .iter()
        .find(|r| r["spec"] == "adapters")
        .expect("expected adapters row");
    assert_eq!(adapters_row["covered"], 1);
    assert_eq!(adapters_row["total"], 1);

    // Verify summary
    assert!(data["summary"]["total_scenarios"].as_u64() >= Some(3));
    assert!(data["summary"]["total_contracts"].as_u64() >= Some(3));
}

#[test]
fn ah_report_json_failure_flips_ok_false_with_exit_one() {
    // espectacular-yr9 (evallerina-54f drift class): a report with gaps
    // exits 1 while the envelope carried ok:true — machine consumers
    // classifying by (ok, exit_code) get an unclassifiable pair. ok must
    // agree with the exit code; a fully-covered report stays ok:true.
    let repo = report_missing_contract_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["report", "--json"])
        .assert()
        .failure();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(
        output["ok"], false,
        "ok must agree with exit code 1: {}",
        output
    );
    let data = data_from_envelope(&output);
    assert!(data["summary"]["missing"].as_u64().unwrap() >= 1);
}

#[test]
fn ah_report_exits_zero_when_coverage_complete() {
    let repo = report_full_coverage_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("report")
        .assert()
        .success();
    assert_eq!(
        assert.get_output().status.code(),
        Some(0),
        "a fully covered report exits zero"
    );
}

#[test]
fn ah_report_exits_nonzero_when_missing_contracts() {
    let repo = report_missing_contract_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("report")
        .assert()
        .code(1);
    assert_eq!(
        assert.get_output().status.code(),
        Some(1),
        "a report with a gap exits one"
    );
}

#[test]
fn ah_report_table_output_has_header() {
    let repo = report_full_coverage_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("report")
        .assert()
        .success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    assert!(
        stdout.contains("spec") || stdout.contains("compiler"),
        "table output should contain spec names"
    );
}

fn report_archetype_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    report_capability(
        repo,
        "compiler",
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Uncontracted path\n- **WHEN** it runs\n- **THEN** nothing declares it\n",
    );
    report_config(repo);
    report_contract(repo, "compiler", "green-path", "unit", "flags = \"ok\"");
    write_executable(&repo.join("runner.sh"), "exit 0");
    dir
}

#[test]
fn ah_report_json_attributes_scenarios_to_contract_archetype() {
    let repo = report_archetype_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["report", "--json"])
        .assert()
        .code(1);
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    let matrix = data["matrix"].as_array().expect("expected matrix array");

    // The PF contract scenario is attributed to the PF row with real counts.
    let pf_row = matrix
        .iter()
        .find(|r| r["spec"] == "compiler" && r["archetype"] == "PF")
        .expect("expected PF row");
    assert_eq!(pf_row["covered"], 1);
    assert_eq!(pf_row["missing"], 0);
    assert_eq!(pf_row["failing"], 0);
    assert_eq!(pf_row["total"], 1);

    // The uncontracted scenario remains under an explicit empty archetype.
    let blank_row = matrix
        .iter()
        .find(|r| r["spec"] == "compiler" && r["archetype"] == "")
        .expect("expected blank archetype row");
    assert_eq!(blank_row["covered"], 0);
    assert_eq!(blank_row["missing"], 1);
    assert_eq!(blank_row["total"], 1);

    // No other rows exist for compiler.
    let compiler_rows: Vec<_> = matrix.iter().filter(|r| r["spec"] == "compiler").collect();
    assert_eq!(compiler_rows.len(), 2);
}

fn report_failing_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    report_capability(
        repo,
        "compiler",
        "# Capability: compiler\n\n#### Scenario: Failing path\n- **WHEN** it runs\n- **THEN** it fails\n",
    );
    report_config(repo);
    report_contract(repo, "compiler", "failing-path", "unit", "flags = \"fail\"");
    write_executable(&repo.join("runner.sh"), "exit 1");
    dir
}

#[test]
fn ah_report_json_emits_failing_counts() {
    let repo = report_failing_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["report", "--json"])
        .assert()
        .code(1);
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    let matrix = data["matrix"].as_array().expect("expected matrix array");
    let compiler_row = matrix
        .iter()
        .find(|r| r["spec"] == "compiler")
        .expect("expected compiler row");
    assert_eq!(compiler_row["covered"], 0);
    assert_eq!(compiler_row["missing"], 0);
    assert_eq!(compiler_row["failing"], 1);
    assert_eq!(compiler_row["total"], 1);
    assert_eq!(data["summary"]["failing"], 1);
}

#[test]
fn p_report() {
    ah_report_json_emits_matrix_with_coverage_counts();
    ah_report_exits_zero_when_coverage_complete();
    ah_report_exits_nonzero_when_missing_contracts();
    ah_report_table_output_has_header();
    assert!(
        report_full_coverage_repo()
            .path()
            .join(".espectacular/compiler/shell-path.toml")
            .is_file(),
        "full-coverage fixture keeps both compiler contracts"
    );
}

#[test]
fn p_matrix() {
    ah_report_json_emits_matrix_with_coverage_counts();
    ah_report_exits_nonzero_when_missing_contracts();
    let missing = report_missing_contract_repo();
    assert!(
        !missing
            .path()
            .join(".espectacular/compiler/shell-path.toml")
            .exists(),
        "missing-contract fixture drops exactly the shell-path contract"
    );
}
