use assert_cmd::Command;
use std::fs;

fn make_minimal_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    let version = env!("CARGO_PKG_VERSION");
    fs::write(
        root.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{version}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n"
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
        "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\n",
    )
    .unwrap();
    dir
}

// ── 7.5/7.6: --enable writes exact config tables ─────────────────────────────

#[test]
fn doctor_enable_pytest_writes_runner_and_reports_table() {
    let repo = make_minimal_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "pytest"])
        .assert()
        .success()
        .stdout(predicates::str::contains("runners.pytest"));
    let config = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
    assert!(
        config.contains("pytest = [\"pytest\"]"),
        "pytest runner must be in config; got:\n{config}"
    );
}

#[test]
fn doctor_enable_cargo_writes_runner_and_reports_table() {
    let repo = make_minimal_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "cargo"])
        .assert()
        .success()
        .stdout(predicates::str::contains("runners.cargo"));
    let config = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
    assert!(
        config.contains(r#"cargo = ["cargo", "test"]"#),
        "cargo runner must be in config; got:\n{config}"
    );
}

#[test]
fn doctor_enable_vitest_writes_runner_and_reports_table() {
    let repo = make_minimal_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "vitest"])
        .assert()
        .success()
        .stdout(predicates::str::contains("runners.vitest"));
    let config = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
    assert!(
        config.contains(r#"vitest = ["vitest", "run"]"#),
        "vitest runner must be in config; got:\n{config}"
    );
}

#[test]
fn doctor_enable_mutation_writes_capability_block() {
    let repo = make_minimal_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "mutation"])
        .assert()
        .success()
        .stdout(predicates::str::contains("capabilities.mutation"));
    let config = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
    assert!(
        config.contains("[capabilities.mutation]") && config.contains("enabled = true"),
        "mutation capability block must be in config; got:\n{config}"
    );
}

#[test]
fn doctor_enable_property_writes_capability_block() {
    let repo = make_minimal_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "property"])
        .assert()
        .success()
        .stdout(predicates::str::contains("capabilities.property"));
    let config = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
    assert!(
        config.contains("[capabilities.property]") && config.contains("enabled = true"),
        "property capability block must be in config; got:\n{config}"
    );
}

#[test]
fn doctor_enable_snapshot_writes_capability_block() {
    let repo = make_minimal_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "snapshot"])
        .assert()
        .success()
        .stdout(predicates::str::contains("capabilities.snapshot"));
    let config = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
    assert!(
        config.contains("[capabilities.snapshot]") && config.contains("enabled = true"),
        "snapshot capability block must be in config; got:\n{config}"
    );
}

// ── 7.7/7.8: unknown capability error and already-enabled no-op ───────────────

#[test]
fn doctor_enable_unknown_capability_exits_nonzero() {
    let repo = make_minimal_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "jest"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown capability"));
}

#[test]
fn doctor_enable_already_enabled_exits_zero_with_already_enabled_message() {
    let repo = make_minimal_repo();
    // Enable pytest first
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "pytest"])
        .assert()
        .success();
    // Enable again: should report already-enabled, not fail
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "pytest"])
        .assert()
        .success()
        .stdout(predicates::str::contains("already-enabled"));
}

// ── 7.1/7.2: detection and recommendation output in plain doctor ──────────────

#[test]
fn doctor_reports_configured_framework_as_framework_line() {
    let repo = make_minimal_repo();
    // Enable pytest first so it shows as configured
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "pytest"])
        .assert()
        .success();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("framework: pytest (configured)"));
}

#[test]
fn doctor_reports_manifest_detected_framework_as_recommendation() {
    let repo = make_minimal_repo();
    fs::write(repo.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("recommendation:"))
        .stdout(predicates::str::contains("ah doctor --enable cargo"));
}
