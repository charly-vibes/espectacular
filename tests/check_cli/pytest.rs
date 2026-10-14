use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::data_from_envelope;
use crate::git_commit_all;
use crate::schema::assert_schema_valid;
use crate::write_executable;
#[test]

fn ah_check_pytest_contract_uses_adapter_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/python")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/python")).unwrap();
    fs::write(
        repo.join("openspec/specs/python/spec.md"),
        "# Capability: python\n\n#### Scenario: Pytest green\n- **WHEN** pytest runs\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(repo.join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npytest = [\"/bin/sh\", \"pytest.sh\"]\n"),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/python/pytest-green.toml"),
        "id = \"pytest-green\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.pytest]]\nflags = \"tests/test_demo.py::test_green\"\n",
    )
    .unwrap();
    write_executable(&repo.join("pytest.sh"), "printf '%s' \"$1\"");
    git_commit_all(repo);

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    assert_eq!(data["summary"]["passed"], 1);
}

#[test]
fn ah_check_pytest_failure_emits_execution_details() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/python")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/python")).unwrap();
    fs::write(
        repo.join("openspec/specs/python/spec.md"),
        "# Capability: python\n\n#### Scenario: Pytest red\n- **WHEN** pytest fails\n- **THEN** it reports an execution finding\n",
    )
    .unwrap();
    fs::write(repo.join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npytest = [\"/bin/sh\", \"pytest.sh\"]\n"),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/python/pytest-red.toml"),
        "id = \"pytest-red\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.pytest]]\nflags = \"tests/test_demo.py::test_red\"\n",
    )
    .unwrap();
    write_executable(&repo.join("pytest.sh"), "printf 'import boom' >&2\nexit 9");
    git_commit_all(repo);

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    let failing = data["findings"].as_array().unwrap()[0].clone();
    assert_eq!(failing["kind"], "test-failing");
    assert_eq!(failing["test"]["type"], "pytest");
    assert_eq!(failing["test"]["exit_code"], 9);
    assert_eq!(failing["test"]["stderr_tail"], "import boom");
}

fn write_pytest_repo(
    repo: &Path,
    scenario_id: &str,
    title: &str,
    expectation: &str,
    script_body: &str,
) {
    fs::create_dir_all(repo.join("openspec/specs/python")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/python")).unwrap();
    fs::write(
        repo.join("openspec/specs/python/spec.md"),
        format!(
            "# Capability: python\n\n#### Scenario: {title}\n- **WHEN** pytest runs\n- **THEN** {expectation}\n",
        ),
    )
    .unwrap();
    fs::write(repo.join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npytest = [\"/bin/sh\", \"pytest.sh\"]\n"),
    )
    .unwrap();
    fs::write(
        repo.join(format!(".espectacular/python/{scenario_id}.toml")),
        format!(
            "id = \"{scenario_id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.pytest]]\nflags = \"tests/test_demo.py::{scenario_id}\"\n",
        ),
    )
    .unwrap();
    write_executable(&repo.join("pytest.sh"), script_body);
    git_commit_all(repo);
}

#[test]
fn ah_check_pytest_json_failure_is_classified_by_adapter() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    write_pytest_repo(
        repo,
        "pytest-import-error",
        "Pytest import error",
        "it reports an import error",
        "printf '%s' '{\"collectors\":[{\"longrepr\":\"ImportError: cannot import name \\\"boom\\\"\"}]}'\nexit 2",
    );

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    let failing = data["findings"].as_array().unwrap()[0].clone();
    assert_eq!(failing["kind"], "test-failing");
    assert_eq!(failing["test"]["type"], "pytest-import-error");
    assert_eq!(failing["test"]["exit_code"], 2);
}

#[test]
fn ah_check_pytest_fixture_failure_is_classified_by_adapter() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    write_pytest_repo(
        repo,
        "pytest-fixture-error",
        "Pytest fixture error",
        "it reports a fixture failure",
        "printf '%s' '{\"tests\":[{\"setup\":{\"crash\":{\"message\":\"fixture \'db\' not found\"}}}]}'\nexit 1",
    );

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    let failing = data["findings"].as_array().unwrap()[0].clone();
    assert_eq!(failing["test"]["type"], "pytest-fixture-error");
    assert_eq!(failing["test"]["exit_code"], 1);
}

#[test]
fn ah_check_pytest_collection_failure_is_classified_by_adapter() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    write_pytest_repo(
        repo,
        "pytest-collection-error",
        "Pytest collection error",
        "it reports a collection failure",
        "printf '%s' '{\"collectors\":[{\"longrepr\":\"ERROR collecting tests/test_demo.py\"}]}'\nexit 2",
    );

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    let failing = data["findings"].as_array().unwrap()[0].clone();
    assert_eq!(failing["test"]["type"], "pytest-collection-error");
    assert_eq!(failing["test"]["exit_code"], 2);
}
