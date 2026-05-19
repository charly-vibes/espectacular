use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn make_change_repo(change: &str, spec: &str, requirement: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let spec_dir = root
        .join("openspec/changes")
        .join(change)
        .join("specs")
        .join(spec);
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_md = format!(
        "# Capability: {spec}\n\n## ADDED Requirements\n\n### Requirement: {requirement}\n"
    );
    fs::write(spec_dir.join("spec.md"), spec_md).unwrap();
    dir
}

fn make_deployed_repo(spec: &str, scenario_id: &str, new_scenario_id: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let deployed_spec_dir = root.join("openspec/specs").join(spec);
    fs::create_dir_all(&deployed_spec_dir).unwrap();
    let old_heading = scenario_id.replace('-', " ");
    let new_heading = new_scenario_id.replace('-', " ");
    let spec_md = format!(
        "# Capability: {spec}\n\n## Requirements\n\n### Requirement: Core\n\n#### Scenario: {old_heading}\n- **WHEN** action\n- **THEN** result\n\n#### Scenario: {new_heading}\n- **WHEN** new action\n- **THEN** new result\n"
    );
    fs::write(deployed_spec_dir.join("spec.md"), spec_md).unwrap();

    let contract_dir = root.join(".espectacular").join(spec);
    fs::create_dir_all(&contract_dir).unwrap();
    let contract = format!(
        "id = \"{scenario_id}\"\ndescription = \"Test.\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"tests/test_foo.py\"\ntimeout_seconds = 60\n"
    );
    fs::write(contract_dir.join(format!("{scenario_id}.toml")), contract).unwrap();
    dir
}

#[test]
fn ah_scenario_new_exits_zero_and_prints_paths() {
    let dir = make_change_repo("s5", "compiler", "Input validation");
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "scenario",
            "new",
            "s5",
            "compiler",
            "--requirement",
            "Input validation",
            "Null bytes rejected",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "openspec/changes/s5/specs/compiler/spec.md",
        ))
        .stdout(predicates::str::contains(
            ".espectacular/changes/s5/compiler/null-bytes-rejected.toml",
        ));
}

#[test]
fn ah_scenario_new_fails_when_change_spec_missing() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "scenario",
            "new",
            "s5",
            "compiler",
            "--requirement",
            "Input validation",
            "Some scenario",
        ])
        .assert()
        .failure();
}

#[test]
fn ah_scenario_new_fails_when_requirement_missing() {
    let dir = make_change_repo("s5", "compiler", "Input validation");
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "scenario",
            "new",
            "s5",
            "compiler",
            "--requirement",
            "Nonexistent requirement",
            "Some scenario",
        ])
        .assert()
        .failure();
}

#[test]
fn ah_scenario_supersede_exits_zero_and_prints_path() {
    let dir = make_deployed_repo("compiler", "empty-input-rejected", "null-bytes-rejected");
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "scenario",
            "supersede",
            "compiler",
            "empty-input-rejected",
            "--with",
            "null-bytes-rejected",
            "--in-change",
            "s5",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            ".espectacular/changes/s5/compiler/empty-input-rejected.toml",
        ));
}

#[test]
fn ah_scenario_supersede_fails_when_new_id_not_in_scope() {
    let dir = make_deployed_repo("compiler", "empty-input-rejected", "null-bytes-rejected");
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "scenario",
            "supersede",
            "compiler",
            "empty-input-rejected",
            "--with",
            "does-not-exist",
            "--in-change",
            "s5",
        ])
        .assert()
        .failure();
}
