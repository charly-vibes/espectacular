use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::data_from_envelope;
use crate::git_commit_all;
use crate::schema::assert_schema_valid;
use crate::write_executable;

/// Create one capability's spec/contract directories and write its spec.
fn e2e_repo(capability: &str, spec: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join(format!("openspec/specs/{capability}"))).unwrap();
    fs::create_dir_all(repo.join(format!(".espectacular/{capability}"))).unwrap();
    fs::write(
        repo.join(format!("openspec/specs/{capability}/spec.md")),
        spec,
    )
    .unwrap();
    dir
}

/// Write the runner config for `runner` plus its `<runner>.sh` shim.
fn e2e_config(repo: &Path, runner: &str, script: &str) {
    fs::write(
        repo.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{}\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n{runner} = [\"/bin/sh\", \"{runner}.sh\"]\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    write_executable(&repo.join(format!("{runner}.sh")), script);
}

/// Write one PF contract binding a single `[[tests.<runner>]]` flags entry.
fn e2e_contract(repo: &Path, capability: &str, id: &str, runner: &str, flags: &str) {
    fs::write(
        repo.join(format!(".espectacular/{capability}/{id}.toml")),
        format!(
            "id = \"{id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.{runner}]]\nflags = \"{flags}\"\n"
        ),
    )
    .unwrap();
}

/// Run `ah check --run-tests --json`, require success, validate the envelope
/// and return its data payload.
fn run_check_envelope(repo: &Path) -> Value {
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    data_from_envelope(&output).clone()
}

// 11.1 — E2E: Python project with pytest, ah check produces zero findings
#[test]
fn ah_check_python_pytest_e2e_zero_findings() {
    let dir = e2e_repo(
        "app",
        "# Capability: app\n\n#### Scenario: Pytest green\n- **WHEN** pytest runs\n- **THEN** it passes\n",
    );
    let repo = dir.path();
    fs::write(repo.join("pytest.ini"), "[pytest]\n").unwrap();
    e2e_config(repo, "pytest", "exit 0");
    e2e_contract(
        repo,
        "app",
        "pytest-green",
        "pytest",
        "tests/test_app.py::test_passes",
    );
    git_commit_all(repo);

    let data = run_check_envelope(repo);
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 1);
}

// 11.2 — E2E: Rust project with cargo test, ah check produces zero findings
#[test]
fn ah_check_rust_cargo_e2e_zero_findings() {
    let dir = e2e_repo(
        "lib",
        "# Capability: lib\n\n#### Scenario: Cargo green\n- **WHEN** cargo test runs\n- **THEN** it passes\n",
    );
    let repo = dir.path();
    fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    e2e_config(repo, "cargo", "exit 0");
    e2e_contract(repo, "lib", "cargo-green", "cargo", "lib::tests::it_works");
    git_commit_all(repo);

    let data = run_check_envelope(repo);
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 1);
}

// 11.3 — E2E: TypeScript project with vitest, ah check produces zero findings
#[test]
fn ah_check_typescript_vitest_e2e_zero_findings() {
    let dir = e2e_repo(
        "ui",
        "# Capability: ui\n\n#### Scenario: Vitest green\n- **WHEN** vitest runs\n- **THEN** it passes\n",
    );
    let repo = dir.path();
    fs::write(
        repo.join("package.json"),
        r#"{"devDependencies":{"vitest":"^1.0.0"}}"#,
    )
    .unwrap();
    e2e_config(repo, "vitest", "exit 0");
    e2e_contract(repo, "ui", "vitest-green", "vitest", "src/ui.test.ts");
    git_commit_all(repo);

    let data = run_check_envelope(repo);
    assert_eq!(data["findings"], Value::Array(vec![]));
    assert_eq!(data["summary"]["passed"], 1);
}
