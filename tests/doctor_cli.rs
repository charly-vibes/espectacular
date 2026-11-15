use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
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
        "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\npre-push:\n  commands:\n    ah-check:\n      run: ah check\n",
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
    assert!(config.contains("pytest = [\"pytest\"]"),);
}

// ── 7.7: ah doctor --json emits recommendation findings ───────────────────────

#[test]
fn doctor_json_emits_recommendation_findings() {
    let repo = make_minimal_repo();
    fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--json"])
        .assert()
        .success();
    let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout).unwrap();
    let data = &json["data"];
    let suggestions = data["suggestions"]
        .as_array()
        .expect("expected a suggestions array");
    let rec = suggestions
        .iter()
        .find(|f| f["kind"] == "recommendation")
        .expect("expected a recommendation suggestion");
    assert_eq!(rec["playbook_command"], "ah explain enable_capability");
    assert!(!rec["suggested_action"].as_str().unwrap_or("").is_empty());
    assert_eq!(rec["apply_command"], "ah doctor --enable pytest");
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
fn doctor_enable_mutation_writes_quality_mutation_config() {
    let repo = make_minimal_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "mutation"])
        .assert()
        .success()
        .stdout(predicates::str::contains("quality.mutation"));
    let config = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
    assert!(
        config.contains("[quality.mutation]") && config.contains("enabled = true"),
        "mutation quality block must be in config; got:\n{config}"
    );
    assert!(
        config.contains("threshold = 0.80"),
        "must include default threshold; got:\n{config}"
    );
    assert!(
        config.contains("command = [\"{}\"]"),
        "must include the {{}} placeholder the mutation engine substitutes; got:\n{config}"
    );
}

// ── djw decision A: pretender owns mutation execution + scoring ───────────────

#[test]
fn doctor_mutation_non_pretender_command_warns_ownership() {
    let repo = make_minimal_repo();
    let config = repo.path().join(".espectacular/config.toml");
    fs::write(
        &config,
        fs::read_to_string(&config).unwrap()
            + "\n[quality.mutation]\nenabled = true\nthreshold = 0.80\ncommand = [\"cargo-mutants\"]\n",
    )
    .unwrap();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("mutation-ownership"))
        .stderr(predicates::str::contains("pretender"));
    assert_eq!(
        output.get_output().status.code(),
        Some(1),
        "an unowned mutation command must fail the doctor run"
    );
}

#[test]
fn doctor_mutation_pretender_command_emits_no_ownership_warning() {
    let repo = make_minimal_repo();
    let config = repo.path().join(".espectacular/config.toml");
    fs::write(
        &config,
        fs::read_to_string(&config).unwrap()
            + "\n[quality.mutation]\nenabled = true\nthreshold = 0.80\ncommand = [\"pretender\", \"mutation\", \"--format\", \"json\"]\n",
    )
    .unwrap();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("mutation-ownership").not());
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("healthy: all checks passed"),
        "a pretender-owned mutation command must stay healthy; got:\n{stdout}"
    );
}

#[test]
fn doctor_mutation_default_runner_template_emits_no_ownership_warning() {
    let repo = make_minimal_repo();
    let config = repo.path().join(".espectacular/config.toml");
    fs::write(
        &config,
        fs::read_to_string(&config).unwrap()
            + "\n[quality.mutation]\nenabled = true\nthreshold = 0.80\ncommand = [\"{}\"]\n",
    )
    .unwrap();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("mutation-ownership").not());
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("healthy: all checks passed"),
        "the generated-runner default must stay healthy; got:\n{stdout}"
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
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "jest"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown capability"));
    let stderr = String::from_utf8(output.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("known: pytest, cargo, vitest, mutation, property, snapshot"),
        "the error must list the known capabilities; got:\n{stderr}"
    );
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
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "pytest"])
        .assert()
        .success()
        .stdout(predicates::str::contains("already-enabled"));
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert_eq!(
        stdout.trim(),
        "already-enabled: pytest",
        "re-enabling must report a no-op instead of rewriting the table"
    );
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
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("framework: pytest (configured)"));
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("healthy: all checks passed"),
        "a configured framework must not leave findings behind; got:\n{stdout}"
    );
}

#[test]
fn doctor_reports_manifest_detected_framework_as_recommendation() {
    let repo = make_minimal_repo();
    fs::write(repo.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("recommendation:"))
        .stdout(predicates::str::contains("ah doctor --enable cargo"));
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let rec_line = stdout
        .lines()
        .find(|line| {
            line.starts_with("recommendation:") && line.contains("cargo detected via manifest")
        })
        .unwrap_or("");
    assert!(
        rec_line.contains(" — run: ah doctor --enable cargo"),
        "the recommendation must pair its detail with the apply command; got:\n{stdout}"
    );
}

#[test]
fn p_doctor() {
    doctor_reports_configured_framework_as_framework_line();
    doctor_reports_manifest_detected_framework_as_recommendation();

    let repo = make_minimal_repo();
    fs::write(repo.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert();
    assert_eq!(
        assert.get_output().status.code(),
        Some(0),
        "plain doctor must exit 0 when every check passes"
    );
}

#[test]
fn p_enable() {
    doctor_enable_pytest_writes_runner_and_reports_table();
    doctor_enable_cargo_writes_runner_and_reports_table();
    doctor_enable_vitest_writes_runner_and_reports_table();
    doctor_enable_mutation_writes_quality_mutation_config();
    doctor_enable_property_writes_capability_block();
    doctor_enable_snapshot_writes_capability_block();
    doctor_enable_unknown_capability_exits_nonzero();
    doctor_enable_already_enabled_exits_zero_with_already_enabled_message();

    let repo = make_minimal_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--enable", "pytest"])
        .assert();
    assert_eq!(
        assert.get_output().status.code(),
        Some(0),
        "enabling a known capability must exit 0"
    );
}
#[test]
fn p_recommendation() {
    doctor_json_emits_recommendation_findings();

    let repo = make_minimal_repo();
    fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor", "--json"])
        .assert();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let has_rec = stdout.contains("\"recommendation\"");
    assert!(has_rec, "no recommendation: {stdout}");
}
