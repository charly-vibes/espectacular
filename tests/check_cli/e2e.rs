use assert_cmd::Command;
use serde_json::Value;
use std::fs;

use crate::data_from_envelope;
use crate::git_commit_all;
use crate::schema::assert_schema_valid;
use crate::write_executable;

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
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 1);
}
