use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn assert_schema_valid(instance: &Value) {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string("schemas/check-output.schema.json").unwrap())
            .unwrap();
    let compiled = jsonschema::JSONSchema::compile(&raw).unwrap();
    let validation = compiled.validate(instance);
    if let Err(errors) = validation {
        let messages: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("schema validation failed: {messages:?}");
    }
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

fn base_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Shell path\n- **WHEN** shell command runs\n- **THEN** it passes\n",
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
    fs::write(
        repo.join(".espectacular/compiler/shell-path.toml"),
        "id = \"shell-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.shell]]\ncommand = \"printf shell\"\n",
    )
    .unwrap();
    write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");
    dir
}

#[test]
fn ah_check_success_emits_schema_valid_json() {
    let repo = base_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("check")
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    assert_eq!(output["findings"], Value::Array(vec![]));
    assert_eq!(output["summary"]["passed"], 2);
    assert_eq!(output["scope"]["deployed"], true);
}

#[test]
fn ah_check_failure_emits_execution_details_and_exit_one() {
    let repo = base_repo();
    write_executable(&repo.path().join("runner.sh"), "printf 'boom' >&2\nexit 7");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("check")
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);

    let findings = output["findings"].as_array().unwrap();
    let failing = findings
        .iter()
        .find(|finding| finding["kind"] == "test-failing")
        .unwrap();
    assert_eq!(failing["category"], "execution");
    assert_eq!(failing["scenario"]["id"], "green-path");
    assert_eq!(failing["test"]["type"], "unit");
    assert_eq!(failing["test"]["exit_code"], 7);
    assert_eq!(failing["test"]["stderr_tail"], "boom");
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
        .args(["check", "--changes", "add-parser"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    assert_eq!(
        output["scope"]["changes"],
        serde_json::json!(["add-parser"])
    );
    assert_eq!(output["summary"]["passed"], 3);
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
