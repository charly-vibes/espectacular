use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn make_healthy_doctor_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    fs::write(
        root.join(".espectacular/config.toml"),
        "tool_version = \"0.2.2\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n",
    ).unwrap();
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
        "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\n",
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
        .stderr(predicates::str::contains("bad-config"));
}

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

fn assert_custom_runner_schema_valid(instance: &Value) {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string("schemas/custom-runner.schema.json").unwrap())
            .unwrap();
    let compiled = jsonschema::JSONSchema::compile(&raw).unwrap();
    let validation = compiled.validate(instance);
    if let Err(errors) = validation {
        let messages: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("custom runner schema validation failed: {messages:?}");
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
        "tool_version = \"0.2.2\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n",
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
        .args(["check", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    assert_eq!(output["findings"], Value::Array(vec![]));
    assert_eq!(output["summary"]["passed"], 2);
    assert_eq!(output["summary"]["counts_by_kind"], serde_json::json!({}));
    assert_eq!(output["scope"]["deployed"], true);
}

#[test]
fn ah_check_failure_emits_execution_details_and_exit_one() {
    let repo = base_repo();
    write_executable(&repo.path().join("runner.sh"), "printf 'boom' >&2\nexit 7");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--json"])
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
    assert_eq!(output["summary"]["counts_by_kind"]["test-failing"], 1);
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
        .args(["check", "--json", "--changes", "add-parser"])
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
        "tool_version = \"0.2.2\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npytest = [\"/bin/sh\", \"pytest.sh\"]\n",
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
        .args(["check", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    assert_eq!(output["summary"]["passed"], 1);
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
        "tool_version = \"0.2.2\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npytest = [\"/bin/sh\", \"pytest.sh\"]\n",
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
        .args(["check", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let failing = output["findings"].as_array().unwrap()[0].clone();
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
        "tool_version = \"0.2.2\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npytest = [\"/bin/sh\", \"pytest.sh\"]\n",
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
        .args(["check", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let failing = output["findings"].as_array().unwrap()[0].clone();
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
        .args(["check", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let failing = output["findings"].as_array().unwrap()[0].clone();
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
        .args(["check", "--json"])
        .assert()
        .failure();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let failing = output["findings"].as_array().unwrap()[0].clone();
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
        .arg("check")
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
        "tool_version = \"0.2.2\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\npytest = [\"/bin/sh\", \"pytest.sh\"]\n",
    ).unwrap();
    fs::write(
        repo.join(".espectacular/app/pytest-green.toml"),
        "id = \"pytest-green\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.pytest]]\nflags = \"tests/test_app.py::test_passes\"\n",
    ).unwrap();
    write_executable(&repo.join("pytest.sh"), "exit 0");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    assert_eq!(output["findings"], Value::Array(vec![]));
    assert_eq!(output["summary"]["passed"], 1);
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
        "tool_version = \"0.2.2\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncargo = [\"/bin/sh\", \"cargo.sh\"]\n",
    ).unwrap();
    fs::write(
        repo.join(".espectacular/lib/cargo-green.toml"),
        "id = \"cargo-green\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.cargo]]\nflags = \"lib::tests::it_works\"\n",
    ).unwrap();
    write_executable(&repo.join("cargo.sh"), "exit 0");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    assert_eq!(output["findings"], Value::Array(vec![]));
    assert_eq!(output["summary"]["passed"], 1);
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
        "tool_version = \"0.2.2\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nvitest = [\"/bin/sh\", \"vitest.sh\"]\n",
    ).unwrap();
    fs::write(
        repo.join(".espectacular/ui/vitest-green.toml"),
        "id = \"vitest-green\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.vitest]]\nflags = \"src/ui.test.ts\"\n",
    ).unwrap();
    write_executable(&repo.join("vitest.sh"), "exit 0");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    assert_eq!(output["findings"], Value::Array(vec![]));
    assert_eq!(output["summary"]["passed"], 1);
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
            "tool_version = \"0.2.2\"\n\
             [paths]\n\
             specs = \"openspec/specs\"\n\
             changes = \"openspec/changes\"\n\
             [runners]\n\
             [quality.mutation]\n\
             enabled = true\n\
             threshold = 0.80\n\
             command = [\"{script}\"]\n",
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
        .args(["check", "--json"])
        .assert()
        .success();
}

#[test]
fn ah_check_quality_mutation_finding_present_in_output() {
    let (repo, _runner) = make_mutation_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let qf = &output["quality_findings"];
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
    assert!(
        output.get("quality_findings").is_none()
            || output["quality_findings"].as_array().unwrap().is_empty(),
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
        "tool_version = \"0.2.2\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n",
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
    let matrix = output["matrix"].as_array().expect("expected matrix array");
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
    assert!(output["summary"]["total_scenarios"].as_u64() >= Some(3));
    assert!(output["summary"]["total_contracts"].as_u64() >= Some(3));
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
