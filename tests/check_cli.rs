use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Extract the inner data payload from a genesis envelope.
fn data_from_envelope(output: &Value) -> &Value {
    &output["data"]
}

fn make_healthy_doctor_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    fs::write(
        root.join(".espectacular/config.toml"),
        concat!(
            "tool_version = \"",
            env!("CARGO_PKG_VERSION"),
            "\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join("AGENTS.md"),
        "# P\n\n<!-- ah:managed:start -->\n<!-- ah:managed:end -->\n",
    )
    .unwrap();
    fs::write(
        root.join("CLAUDE.md"),
        "# P\n\n<!-- ah:managed:start -->\n<!-- ah:managed:end -->\n",
    )
    .unwrap();
    fs::write(
        root.join("lefthook.yml"),
        "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\npre-push:\n  commands:\n    ah-check:\n      run: ah check\n",
    )
    .unwrap();
    dir
}

#[test]
fn ah_doctor_healthy_repo_exits_zero() {
    let repo = make_healthy_doctor_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("healthy"));
}

#[test]
fn ah_doctor_bad_config_exits_nonzero() {
    let repo = make_healthy_doctor_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        "tool_version = \"\"\n[paths]\nspecs = \"\"\nchanges = \"\"\n[runners]\n",
    )
    .unwrap();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicates::str::contains("config: bad config"));
}

fn assert_schema_valid(instance: &Value) {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string("schemas/check-output.schema.json").unwrap())
            .unwrap();
    let compiled = jsonschema::Validator::new(&raw).unwrap();
    // If the instance is a genesis envelope, unwrap the data payload
    let data = if instance.get("data").is_some() {
        &instance["data"]
    } else {
        instance
    };
    if let Err(error) = compiled.validate(data) {
        panic!("schema validation failed: {error}");
    }
}

fn assert_custom_runner_schema_valid(instance: &Value) {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string("schemas/custom-runner.schema.json").unwrap())
            .unwrap();
    // The custom-runner schema $refs the check-output schema by absolute URL.
    // Register it locally so validation never touches the network.
    let check_output: Value =
        serde_json::from_str(&fs::read_to_string("schemas/check-output.schema.json").unwrap())
            .unwrap();
    let registry = jsonschema::Registry::new()
        .add(
            "https://charly.dev/espectacular/check-output.schema.json",
            jsonschema::Resource::from_contents(check_output),
        )
        .and_then(|builder| builder.prepare())
        .unwrap();
    let compiled = jsonschema::Validator::options()
        .with_registry(&registry)
        .build(&raw)
        .unwrap();
    // If the instance is a genesis envelope, unwrap the data payload
    let data = if instance.get("data").is_some() {
        &instance["data"]
    } else {
        instance
    };
    if let Err(error) = compiled.validate(data) {
        panic!("custom runner schema validation failed: {error}");
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
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n"),
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
fn custom_runner_schema_accepts_empty_findings_pass_case() {
    let instance = serde_json::json!({
        "exit_code": 0,
        "passed": true,
        "findings": []
    });
    assert_custom_runner_schema_valid(&instance);
}

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
fn ah_check_warning_only_findings_exit_zero() {
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

// 11.1 — E2E: Python project with pytest, ah check produces zero findings
#[test]
fn ah_check_python_pytest_e2e_zero_findings() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/app")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/app")).unwrap();
    fs::write(
        repo.join("openspec/specs/app/spec.md"),
        "# Capability: app\n\n#### Scenario: Pytest green\n- **WHEN** pytest runs\n- **THEN** it passes\n",
    ).unwrap();
    fs::write(repo.join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npytest = [\"/bin/sh\", \"pytest.sh\"]\n"),
    ).unwrap();
    fs::write(
        repo.join(".espectacular/app/pytest-green.toml"),
        "id = \"pytest-green\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.pytest]]\nflags = \"tests/test_app.py::test_passes\"\n",
    ).unwrap();
    write_executable(&repo.join("pytest.sh"), "exit 0");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 1);
}

// 11.2 — E2E: Rust project with cargo test, ah check produces zero findings
#[test]
fn ah_check_rust_cargo_e2e_zero_findings() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/lib")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/lib")).unwrap();
    fs::write(
        repo.join("openspec/specs/lib/spec.md"),
        "# Capability: lib\n\n#### Scenario: Cargo green\n- **WHEN** cargo test runs\n- **THEN** it passes\n",
    ).unwrap();
    fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncargo = [\"/bin/sh\", \"cargo.sh\"]\n"),
    ).unwrap();
    fs::write(
        repo.join(".espectacular/lib/cargo-green.toml"),
        "id = \"cargo-green\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.cargo]]\nflags = \"lib::tests::it_works\"\n",
    ).unwrap();
    write_executable(&repo.join("cargo.sh"), "exit 0");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 1);
}

// 11.3 — E2E: TypeScript project with vitest, ah check produces zero findings
#[test]
fn ah_check_typescript_vitest_e2e_zero_findings() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/ui")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/ui")).unwrap();
    fs::write(
        repo.join("openspec/specs/ui/spec.md"),
        "# Capability: ui\n\n#### Scenario: Vitest green\n- **WHEN** vitest runs\n- **THEN** it passes\n",
    ).unwrap();
    fs::write(
        repo.join("package.json"),
        r#"{"devDependencies":{"vitest":"^1.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nvitest = [\"/bin/sh\", \"vitest.sh\"]\n"),
    ).unwrap();
    fs::write(
        repo.join(".espectacular/ui/vitest-green.toml"),
        "id = \"vitest-green\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.vitest]]\nflags = \"src/ui.test.ts\"\n",
    ).unwrap();
    write_executable(&repo.join("vitest.sh"), "exit 0");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 1);
}

// ── batched vitest bindings (batch-runner-spawns, GH#40) ─────────────────────

// Fixture: a repo with vitest-bound contracts (scenario-scoped
// `--testNamePattern=p_<id>`, the tambor shape — pattern-scoped, no file
// notion) plus a vitest shim that appends one line per invocation to
// `invocations.log` and emits the vitest JSON-reporter shape with each test's
// declared status, so per-contract attribution from structured output is
// exercisable. A status of `None` omits the test from the report entirely
// (matched-zero shape).
fn vitest_pattern_repo_with(statuses: &[(&str, Option<&str>)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/ui")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/ui")).unwrap();

    let mut spec = String::from("# Capability: ui\n\n");
    let mut assertions: Vec<String> = Vec::new();
    let mut declared = 0usize;
    for (id, status) in statuses.iter() {
        spec.push_str(&format!(
            "#### Scenario: {id}\n- **WHEN** vitest runs\n- **THEN** it passes\n\n"
        ));
        fs::write(
            repo.join(format!(".espectacular/ui/{id}.toml")),
            format!(
                "id = \"{id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.vitest]]\nflags = \"--testNamePattern=p_{id}\"\n"
            ),
        )
        .unwrap();
        if let Some(status) = status {
            declared += 1;
            assertions.push(format!(
                r#"{{"ancestorTitles": [], "fullName": "p_{id}", "status": "{status}", "title": "{id}"}}"#
            ));
        }
    }
    let assertions = assertions.join(", ");
    fs::write(repo.join("openspec/specs/ui/spec.md"), spec).unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!(
            "tool_version = \"",
            env!("CARGO_PKG_VERSION"),
            "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nvitest = [\"/bin/sh\", \"vitest.sh\"]\n"
        ),
    )
    .unwrap();

    let json = format!(
        r#"{{"numTotalTests": {declared}, "numPassedTests": {declared}, "numFailedTests": 0, "success": true, "testResults": [{{"name": "src/ui.test.ts", "status": "passed", "assertionResults": [{assertions}]}}]}}"#
    );
    write_executable(
        &repo.join("vitest.sh"),
        &format!("printf '%s\\n' \"$*\" >> invocations.log\ncat <<'JSON'\n{json}\nJSON\n"),
    );

    dir
}

fn vitest_pattern_repo(n: usize) -> tempfile::TempDir {
    let statuses: Vec<(String, Option<&str>)> = (1..=n)
        .map(|i| (format!("spawn-{i}"), Some("passed")))
        .collect();
    let statuses: Vec<(&str, Option<&str>)> =
        statuses.iter().map(|(id, s)| (id.as_str(), *s)).collect();
    vitest_pattern_repo_with(&statuses)
}

// batch-runner-spawns task 1.1 (GH#40): above the batching threshold, all
// pattern-scoped vitest bindings share ONE runner invocation instead of one
// spawn per contract; per-contract verdicts still come from the structured
// output, so the summary and findings are unchanged.
#[test]
fn ah_check_batched_vitest_bindings_spawn_one_invocation_above_threshold() {
    let repo = vitest_pattern_repo(9);
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    assert_eq!(
        data["findings"],
        Value::Array(vec![]),
        "batched run must attribute per-contract verdicts from structured output"
    );
    assert_eq!(data["summary"]["passed"], 9);

    let log = fs::read_to_string(repo.path().join("invocations.log")).unwrap();
    let invocations = log.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(
        invocations, 1,
        "9 above-threshold vitest bindings must share 1 runner invocation; log:\n{log}"
    );
}

// batch-runner-spawns task 1.4: a JS-only-regex binding (lookahead — Rust
// regex cannot compile it) is excluded from batching at eligibility time and
// keeps its own per-binding spawn: 10 bindings → 1 batched invocation (9
// patterns, no lookahead) + 1 per-binding invocation carrying the lookahead.
#[test]
fn ah_check_js_only_pattern_binding_runs_per_binding_alongside_batch() {
    let repo = vitest_pattern_repo(10);
    fs::write(
        repo.path().join(".espectacular/ui/spawn-1.toml"),
        concat!(
            "id = \"spawn-1\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\n",
            "superseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.vitest]]\n",
            "flags = \"--testNamePattern=p_spawn-(?=1)\"\n"
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
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 10);

    let log = fs::read_to_string(repo.path().join("invocations.log")).unwrap();
    let lines: Vec<&str> = log.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "1 batched + 1 per-binding spawn expected; log:\n{log}"
    );
    let per_binding = lines.iter().filter(|l| l.contains("(?=")).count();
    assert_eq!(
        per_binding, 1,
        "exactly one per-binding invocation must carry the lookahead; log:\n{log}"
    );
}

// batch-runner-spawns task 2.1 (C-batch-attribution, e2e): in a batched run,
// a failed test matched by one binding's pattern fails ONLY that contract —
// the batched exit code never leaks into other bindings' verdicts.
#[test]
fn ah_check_batched_failed_test_fails_only_its_contract() {
    let statuses: Vec<(String, Option<&str>)> = (1..=10)
        .map(|i| {
            let id = format!("spawn-{i}");
            let status = if i == 3 { "failed" } else { "passed" };
            (id, Some(status))
        })
        .collect();
    let statuses: Vec<(&str, Option<&str>)> =
        statuses.iter().map(|(id, s)| (id.as_str(), *s)).collect();
    let repo = vitest_pattern_repo_with(&statuses);

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
    let failing: Vec<_> = findings
        .iter()
        .filter(|f| f["kind"] == "test-failing")
        .collect();
    assert_eq!(failing.len(), 1, "only the failed test's contract fails");
    assert_eq!(failing[0]["scenario"]["id"], "spawn-3");
    assert_eq!(failing[0]["category"], "execution");
    assert_eq!(failing[0]["suggested_action"], "edit_code_not_scenario");
    assert_eq!(failing[0]["test"]["type"], "vitest");
    assert_eq!(data["summary"]["passed"], 9);
    assert_eq!(data["summary"]["counts_by_kind"]["test-failing"], 1);
}

// batch-runner-spawns task 2.1 (C-batch-matched-zero, e2e): a pattern that
// matches nothing in the structured output emits no-tests-ran even though the
// batched invocation exited zero — the exit code never covers a contract.
#[test]
fn ah_check_batched_matched_zero_emits_no_tests_ran_despite_zero_exit() {
    let statuses: Vec<(String, Option<&str>)> = (1..=10)
        .map(|i| {
            let id = format!("spawn-{i}");
            // spawn-7's test is absent from the report entirely.
            let status = if i == 7 { None } else { Some("passed") };
            (id, status)
        })
        .collect();
    let statuses: Vec<(&str, Option<&str>)> =
        statuses.iter().map(|(id, s)| (id.as_str(), *s)).collect();
    let repo = vitest_pattern_repo_with(&statuses);

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
    let no_tests: Vec<_> = findings
        .iter()
        .filter(|f| f["kind"] == "no-tests-ran")
        .collect();
    assert_eq!(no_tests.len(), 1, "matched-zero contract must be flagged");
    assert_eq!(no_tests[0]["scenario"]["id"], "spawn-7");
    assert_eq!(data["summary"]["passed"], 9);
    assert_eq!(data["summary"]["counts_by_kind"]["no-tests-ran"], 1);
}

// batch-runner-spawns task 2.1 (e2e): a pattern matching only skipped tests
// emits no-tests-ran — a skipped test cannot cover a contract.
#[test]
fn ah_check_batched_skipped_match_emits_no_tests_ran() {
    let statuses: Vec<(String, Option<&str>)> = (1..=10)
        .map(|i| {
            let id = format!("spawn-{i}");
            let status = if i == 4 { "skipped" } else { "passed" };
            (id, Some(status))
        })
        .collect();
    let statuses: Vec<(&str, Option<&str>)> =
        statuses.iter().map(|(id, s)| (id.as_str(), *s)).collect();
    let repo = vitest_pattern_repo_with(&statuses);

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
    let no_tests: Vec<_> = findings
        .iter()
        .filter(|f| f["kind"] == "no-tests-ran")
        .collect();
    assert_eq!(no_tests.len(), 1);
    assert_eq!(no_tests[0]["scenario"]["id"], "spawn-4");
    assert_eq!(data["summary"]["passed"], 9);
    assert_eq!(data["summary"]["counts_by_kind"]["no-tests-ran"], 1);
}

// batch-runner-spawns task 2.2 (C-batch-fallback, e2e): a batched invocation
// whose structured output is unparseable emits a NAMED fallback signal (a
// warning-severity batch-fallback finding per affected scenario) and re-runs
// every binding per-contract with exit-code verdicts — correctness comes
// from the re-runs, the signal makes the degradation non-silent.
#[test]
fn ah_check_batched_unparseable_output_falls_back_per_binding_with_named_signal() {
    let repo = vitest_pattern_repo(10);
    // The batched invocation (only it carries --reporter=json) emits garbage;
    // per-binding invocations exit zero silently.
    write_executable(
        &repo.path().join("vitest.sh"),
        concat!(
            "#!/bin/sh\n",
            "printf '%s\\n' \"$*\" >> invocations.log\n",
            "case \"$*\" in *--reporter=json*) printf 'not json\\n' ;; *) exit 0 ;; esac\n"
        ),
    );

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);

    // Per-binding re-runs are authoritative: all 10 pass via exit-code verdicts.
    assert_eq!(data["summary"]["passed"], 10);
    assert_eq!(data["summary"]["counts_by_kind"].get("test-failing"), None);

    // Named, non-gating signal: one warning-severity batch-fallback finding
    // per affected scenario, message carrying the reason.
    let findings = data["findings"].as_array().unwrap();
    let fallbacks: Vec<_> = findings
        .iter()
        .filter(|f| f["kind"] == "batch-fallback")
        .collect();
    assert_eq!(fallbacks.len(), 10, "one signal per affected binding");
    for f in &fallbacks {
        assert_eq!(f["severity"], "warning");
        assert_eq!(f["category"], "warning");
        let message = f["message"].as_str().unwrap();
        assert!(
            message.contains("unparseable"),
            "signal must name the reason: {message}"
        );
    }
    let ids: std::collections::BTreeSet<&str> = fallbacks
        .iter()
        .map(|f| f["scenario"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 10, "every scenario is covered by a signal");

    // 1 batched invocation + 10 per-binding re-runs.
    let log = fs::read_to_string(repo.path().join("invocations.log")).unwrap();
    let invocations = log.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(invocations, 11, "batch + 10 re-runs; log:\n{log}");
}

// batch-runner-spawns task 2.4 (C-unbatched-unchanged, e2e): cargo/shell and
// any other runner without structured-reporter support keep per-binding
// exit-code spawns — even with more bindings than the batching threshold,
// no batched (structured-reporter) invocation is ever composed for them.
#[test]
fn ah_check_shell_bindings_above_threshold_never_batch() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/ui")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/ui")).unwrap();

    let mut spec = String::from("# Capability: ui\n\n");
    for i in 1..=10 {
        spec.push_str(&format!(
            "#### Scenario: Shell {i}\n- **WHEN** shell runs\n- **THEN** it exits zero\n\n"
        ));
        fs::write(
            repo.join(format!(".espectacular/ui/shell-{i}.toml")),
            format!(
                "id = \"shell-{i}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.shell]]\ncommand = \"printf 'shell-{i}\\n' >> invocations.log\"\n"
            ),
        )
        .unwrap();
    }
    fs::write(repo.join("openspec/specs/ui/spec.md"), spec).unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!(
            "tool_version = \"",
            env!("CARGO_PKG_VERSION"),
            "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nvitest = [\"/bin/sh\", \"vitest.sh\"]\n"
        ),
    )
    .unwrap();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert_schema_valid(&output);
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 10);

    let log = fs::read_to_string(repo.join("invocations.log")).unwrap();
    let lines: Vec<&str> = log.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        10,
        "10 shell bindings keep 10 per-binding spawns; log:\n{log}"
    );
    assert!(
        lines.iter().all(|l| !l.contains("--reporter=json")),
        "shell bindings must never join a batched invocation; log:\n{log}"
    );
}

// batch-runner-spawns task 1.2: below the threshold (≤ 8 bindings) the
// per-binding spawn behavior is unchanged — one invocation per contract.
#[test]
fn ah_check_below_threshold_vitest_bindings_keep_per_binding_spawns() {
    let repo = vitest_pattern_repo(3);
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
    assert_eq!(data["summary"]["passed"], 3);

    let log = fs::read_to_string(repo.path().join("invocations.log")).unwrap();
    let invocations = log.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(
        invocations, 3,
        "below-threshold bindings keep one spawn per contract; log:\n{log}"
    );
}

// 8.7/8.8: quality findings do not cause non-zero exit

fn make_mutation_repo() -> (tempfile::TempDir, tempfile::TempDir) {
    let runner_dir = tempfile::tempdir().unwrap();
    let script = runner_dir.path().join("mutation-runner.sh");
    fs::write(&script, "#!/bin/sh\nprintf '{\"kill_rate\": 0.50}'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    fs::write(
        root.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{version}\"\n\
             [paths]\n\
             specs = \"openspec/specs\"\n\
             changes = \"openspec/changes\"\n\
             [runners]\n\
             [quality.mutation]\n\
             enabled = true\n\
             threshold = 0.80\n\
             command = [\"{script}\"]\n",
            version = env!("CARGO_PKG_VERSION"),
            script = script.to_string_lossy()
        ),
    )
    .unwrap();
    (dir, runner_dir)
}

#[test]
fn ah_check_quality_mutation_finding_exits_zero() {
    let (repo, _runner) = make_mutation_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();
}

#[test]
fn ah_check_quality_mutation_finding_present_in_output() {
    let (repo, _runner) = make_mutation_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    // Schema must validate even when quality_findings are present
    assert_schema_valid(&output);
    let qf = &data["quality_findings"];
    assert!(qf.is_array(), "quality_findings must be an array");
    assert_eq!(qf.as_array().unwrap().len(), 1);
    assert_eq!(qf[0]["kind"], "quality-mutation");
    assert_eq!(qf[0]["category"], "quality");
    assert!(qf[0]["kill_rate"].as_f64().is_some());
}

#[test]
fn ah_check_mutation_skipped_in_precommit_scope() {
    let (repo, _runner) = make_mutation_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .env("AH_SCOPE", "pre-commit")
        .args(["check", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert!(
        output.get("quality_findings").is_none()
            || data["quality_findings"].as_array().unwrap().is_empty(),
        "quality findings must be empty in pre-commit scope"
    );
}

// ── ah report ──────────────────────────────────────────────────────────────────

fn report_full_coverage_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Shell path\n- **WHEN** shell command runs\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::create_dir_all(repo.join("openspec/specs/adapters")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/adapters")).unwrap();
    fs::write(
        repo.join("openspec/specs/adapters/spec.md"),
        "# Capability: adapters\n\n#### Scenario: Detects cargo\n- **WHEN** cargo exists\n- **THEN** it detects\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n"),
    )
    .unwrap();
    // Two specs, both with PF archetype, both covered
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
    fs::write(
        repo.join(".espectacular/adapters/detects-cargo.toml"),
        "id = \"detects-cargo\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
    )
    .unwrap();
    write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");
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
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("report")
        .assert()
        .success();
}

#[test]
fn ah_report_exits_nonzero_when_missing_contracts() {
    let repo = report_missing_contract_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("report")
        .assert()
        .code(1);
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
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Uncontracted path\n- **WHEN** it runs\n- **THEN** nothing declares it\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n"),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/green-path.toml"),
        "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
    )
    .unwrap();
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
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Failing path\n- **WHEN** it runs\n- **THEN** it fails\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n"),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/failing-path.toml"),
        "id = \"failing-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"fail\"\n",
    )
    .unwrap();
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
fn p_check() {
    ah_check_success_emits_schema_valid_json();
    ah_check_failure_emits_execution_details_and_exit_one();
    ah_check_warning_only_findings_exit_zero();
    ah_check_with_changes_includes_overlay_scope();
    ah_check_missing_change_has_clear_diagnostic();
}

#[test]
fn p_report() {
    ah_report_json_emits_matrix_with_coverage_counts();
    ah_report_exits_zero_when_coverage_complete();
    ah_report_exits_nonzero_when_missing_contracts();
    ah_report_table_output_has_header();
}

#[test]
fn p_schema() {
    ah_check_warning_only_findings_exit_zero();
}

#[test]
fn p_matrix() {
    ah_report_json_emits_matrix_with_coverage_counts();
    ah_report_exits_nonzero_when_missing_contracts();
}

#[test]
fn p_testaruda_l2() {
    ah_check_testaruda_refinement_prunes_unaffected_bindings();
    ah_check_testaruda_empty_selection_runs_everything();
}

// ===== Layer 2: testaruda-store refinement (espectacular-0k8) =====

fn testaruda_available() -> bool {
    Command::new("testaruda")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One capability (compiler) with two unit contracts whose flags are
/// distinctive test names, plus a seeded testaruda store binding
/// src/parser.rs -> parses_input and src/other.rs -> other_thing.
/// The store has passing run history for both tests so neither lands in
/// the always_run fallback (SAFE-007) and selection stays precise.
fn testaruda_l2_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Shell path\n- **WHEN** shell command runs\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/green-path.toml"),
        "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"src::parser::tests::parses_input\"\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/shell-path.toml"),
        "id = \"shell-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"src::other::tests::other_thing\"\n",
    )
    .unwrap();
    write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");
    fs::write(repo.join("src/parser.rs"), "pub fn p() {}\n").unwrap();
    fs::write(repo.join("src/other.rs"), "pub fn o() {}\n").unwrap();

    // Seed the testaruda store: init, import a deterministic graph, then
    // refresh fingerprints from disk so a later in-place edit registers
    // as a change.
    let graph = serde_json::json!({
        "format": "testaruda-graph-v1",
        "content_units": [
            {"id": 1, "kind": "source", "path": "src/parser.rs", "symbol": null, "fingerprint": "unknown", "component": "default"},
            {"id": 2, "kind": "source", "path": "src/other.rs", "symbol": null, "fingerprint": "unknown", "component": "default"}
        ],
        "test_items": [
            {"id": 1, "node_id": "src::parser::tests::parses_input(Test)", "adapter": "rust-adapter", "component": "default", "quarantined": false},
            {"id": 2, "node_id": "src::other::tests::other_thing(Test)", "adapter": "rust-adapter", "component": "default", "quarantined": false}
        ],
        "edges": [
            {"from": 1, "to": 1, "from_node_id": "src::parser::tests::parses_input(Test)", "to_path": "src/parser.rs", "origin": "static", "k": 1000000, "environment": "default"},
            {"from": 2, "to": 2, "from_node_id": "src::other::tests::other_thing(Test)", "to_path": "src/other.rs", "origin": "static", "k": 1000000, "environment": "default"}
        ],
        "run_history": [
            {"run_id": "r1", "node_id": "src::parser::tests::parses_input(Test)", "test_item_id": 1, "outcome": "passed", "environment": "default", "duration_ms": 10},
            {"run_id": "r2", "node_id": "src::other::tests::other_thing(Test)", "test_item_id": 2, "outcome": "passed", "environment": "default", "duration_ms": 10}
        ]
    });
    fs::write(
        repo.join("graph.json"),
        serde_json::to_string(&graph).unwrap(),
    )
    .unwrap();
    // testaruda resolves the project root by walking up for .git — the git
    // repo must exist before any testaruda step, otherwise the store lands
    // in a parent directory and `select` fails with "not initialized".
    for args in [
        vec!["init"],
        vec!["add", "."],
        vec![
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "init",
        ],
    ] {
        let out = Command::new("git")
            .args(&args)
            .current_dir(repo)
            .output()
            .expect("git seed step failed");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    // Seed the testaruda store: init, import a deterministic graph, then
    // refresh fingerprints from disk so a later in-place edit registers
    // as a change.
    for args in [
        vec!["init", "-q"],
        vec!["import", "graph.json", "-q"],
        vec!["fingerprint", "-q"],
    ] {
        let out = Command::new("testaruda")
            .args(&args)
            .current_dir(repo)
            .output()
            .expect("testaruda seed step failed");
        assert!(
            out.status.success(),
            "testaruda {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fs::remove_file(repo.join("graph.json")).unwrap();
    dir
}

/// Layer 2 red: seeded store + changed code file -> ah prunes the contract
/// binding whose underlying test testaruda did not select, and reports the
/// refinement in the JSON selection report.
#[test]
fn ah_check_testaruda_refinement_prunes_unaffected_bindings() {
    if !testaruda_available() {
        eprintln!("skipping: testaruda binary not on PATH");
        return;
    }
    let dir = testaruda_l2_repo();
    let repo = dir.path();
    fs::write(repo.join("src/parser.rs"), "pub fn p() { /* v2 */ }\n").unwrap();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--files", "src/parser.rs", "--json"])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let data = data_from_envelope(&output);
    assert_eq!(
        data["selection"]["testaruda"]["pruned"], 1,
        "other_thing binding must be pruned by testaruda refinement"
    );
    assert_eq!(
        data["summary"]["passed"], 1,
        "only the parses_input contract ran"
    );
}

/// Layer 2 exit-20 semantics: EMPTY testaruda selection means "no code
/// affected" — skip the refinement only; the Layer 1 set (here a
/// conservative bypassed run-all for a code-only change) still runs.
#[test]
fn ah_check_testaruda_empty_selection_runs_everything() {
    if !testaruda_available() {
        eprintln!("skipping: testaruda binary not on PATH");
        return;
    }
    let dir = testaruda_l2_repo();
    let repo = dir.path();
    fs::write(repo.join("README.md"), "# docs\n").unwrap();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--files", "README.md", "--json"])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let data = data_from_envelope(&output);
    assert_eq!(
        data["selection"]["testaruda"]["pruned"], 0,
        "EMPTY selection must not prune anything"
    );
    assert_eq!(
        data["summary"]["passed"], 2,
        "conservative: both contracts ran"
    );
}
