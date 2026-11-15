use crate::check::run_check;
use crate::check::test_support::write_executable;
use std::fs;

fn custom_repo(runner_body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Custom check\n- **WHEN** custom runner invoked\n- **THEN** it reports results\n",
    )
    .unwrap();
    let runner_path = repo.join("custom-runner.sh");
    write_executable(&runner_path, runner_body);
    fs::write(
        repo.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncustom = [\"{runner}\"]\n",
            runner = runner_path.display()
        ),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/custom-check.toml"),
        "id = \"custom-check\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.custom]]\nflags = \"custom-check\"\n",
    )
    .unwrap();
    dir
}

#[test]
fn custom_runner_passed_envelope_counts_as_passed() {
    let dir = custom_repo("printf '{\"exit_code\":0,\"passed\":true,\"findings\":[]}'");
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(output.findings.is_empty());
    assert_eq!(output.summary.passed, 1);
}

#[test]
fn custom_runner_non_zero_exit_emits_test_failing() {
    let dir = custom_repo("printf 'error' >&2; exit 1");
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(output.findings.iter().any(|f| f.kind == "test-failing"));
    assert_eq!(output.summary.passed, 0);
}

/// A custom-runner envelope carrying a schema-valid finding, wired so the
/// runner cats it back: the same repo shape `custom_repo` builds, plus the
/// envelope file the runner prints.
fn envelope_repo(envelope: &serde_json::Value) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Custom check\n- **WHEN** custom runner invoked\n- **THEN** it reports results\n",
    )
    .unwrap();
    let data = repo.join("envelope.json");
    fs::write(&data, envelope.to_string()).unwrap();
    let runner_path = repo.join("custom-runner.sh");
    write_executable(&runner_path, &format!("cat {}", data.display()));
    fs::write(
        repo.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncustom = [\"{runner}\"]\n",
            runner = runner_path.display()
        ),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/custom-check.toml"),
        "id = \"custom-check\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.custom]]\nflags = \"custom-check\"\n",
    )
    .unwrap();
    dir
}

#[test]
fn custom_runner_envelope_findings_flow_into_check_output() {
    let finding = serde_json::json!({
        "kind": "test-failing",
        "category": "execution",
        "spec": "compiler",
        "spec_path": "openspec/specs/compiler/spec.md",
        "scenario": {"id": "custom-check", "title": "Custom check", "body_markdown": "B"},
        "suggested_action": "edit_code_not_scenario",
        "playbook_command": "ah explain edit_code_not_scenario"
    });
    let envelope = serde_json::json!({
        "exit_code": 0,
        "passed": false,
        "findings": [finding]
    });
    let dir = envelope_repo(&envelope);

    let output = run_check(dir.path(), &[], true).unwrap();
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.findings[0].kind, "test-failing");
    assert_eq!(output.summary.passed, 0);
}

fn quality_test_repo(test_type: &str, runner_body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/suite")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/suite")).unwrap();
    fs::write(
        repo.join("openspec/specs/suite/spec.md"),
        "# Capability: suite\n\n#### Scenario: Quality check\n- **WHEN** it runs\n- **THEN** it reports quality\n",
    )
    .unwrap();
    let runner_path = repo.join(format!("{test_type}-runner.sh"));
    write_executable(&runner_path, runner_body);
    fs::write(
        repo.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n{test_type} = [\"{runner}\"]\n",
            runner = runner_path.display()
        ),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/suite/quality-check.toml"),
        format!(
            "id = \"quality-check\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.{test_type}]]\nflags = \"all\"\n"
        ),
    )
    .unwrap();
    dir
}

fn mutation_failure_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs")).unwrap();
    fs::create_dir_all(repo.join(".espectacular")).unwrap();
    let script = repo.join("mutation-tool.sh");
    write_executable(&script, "exit 1");
    fs::write(
        repo.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n\n[quality.mutation]\nenabled = true\nthreshold = 0.80\ncommand = [\"/bin/sh\", \"{}\"]\n",
            script.display()
        ),
    )
    .unwrap();
    dir
}

/// A passing property/snapshot run is an advisory quality signal, never a
/// regular finding.
fn assert_quality_passing(test_type: &str) {
    let expected = format!("quality-{test_type}");
    let dir = quality_test_repo(test_type, "exit 0");
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        output.quality_findings.iter().any(|f| f.kind == expected),
        "{test_type} passing: expected {expected} finding; got: {:?}",
        output.quality_findings
    );
    assert!(
        output.findings.is_empty(),
        "{test_type} passing: expected no regular findings"
    );
}

/// A failing property/snapshot run is a regular execution finding and must
/// not also produce an advisory one.
fn assert_quality_failing(test_type: &str) {
    let dir = quality_test_repo(test_type, "exit 1");
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "test-failing"),
        "{test_type} failing: expected test-failing finding"
    );
    assert!(
        output.quality_findings.is_empty(),
        "{test_type} failing: expected no quality findings on failure"
    );
}

/// A broken mutation tool fails the gate and never shows up as advisory.
fn assert_mutation_tool_failure() {
    let dir = mutation_failure_repo();
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "test-failing"),
        "mutation tool failure: expected test-failing finding; got findings: {:?}",
        output.findings
    );

    // mutation failure does not produce quality findings
    let dir = mutation_failure_repo();
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        output.quality_findings.is_empty(),
        "mutation failure: must not appear as advisory quality finding"
    );
}

#[test]
fn quality_tests_run_sequentially() {
    // 8.5 Red: property/snapshot test entries emit quality findings
    //
    // Quality tests are combined into one function to avoid parallel execution
    // interference — these tests create temp dirs with runner scripts that can
    // collide when run concurrently (see espectacular-9q4).
    assert_quality_passing("property");
    assert_quality_failing("property");
    assert_quality_passing("snapshot");
    assert_quality_failing("snapshot");
    assert_mutation_tool_failure();
}

fn shell_check_repo(runner_body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/app")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/app")).unwrap();
    fs::write(
        repo.join("openspec/specs/app/spec.md"),
        "# Capability: app\n\n#### Scenario: Shell check\n- **WHEN** the command runs\n- **THEN** tests pass\n",
    ).unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
    ).unwrap();
    let script = repo.join("check.sh");
    write_executable(&script, runner_body);
    fs::write(
        repo.join(".espectacular/app/shell-check.toml"),
        format!(
            "id = \"shell-check\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.shell]]\ncommand = \"/bin/sh {}\"\n",
            script.display()
        ),
    ).unwrap();
    dir
}

#[test]
fn shell_zero_tests_ran_emits_no_tests_ran_finding() {
    let dir = shell_check_repo(
        "printf 'test result: ok. 0 passed; 0 failed; 0 ignored; 5 filtered out\\n'",
    );
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "no-tests-ran"),
        "expected no-tests-ran finding; got: {:?}",
        output.findings
    );
    assert_eq!(output.summary.passed, 0);
}

#[test]
fn shell_with_passing_tests_does_not_emit_no_tests_ran() {
    let dir = shell_check_repo("printf 'test result: ok. 3 passed; 0 failed\\n'");
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        !output.findings.iter().any(|f| f.kind == "no-tests-ran"),
        "unexpected no-tests-ran finding"
    );
    assert_eq!(output.summary.passed, 1);
}

#[test]
fn shell_multiple_binaries_no_false_positive_no_tests_ran() {
    let dir = shell_check_repo(
        "printf 'test result: ok. 3 passed; 0 failed\\ntest result: ok. 0 passed; 0 failed\\n'",
    );
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        !output.findings.iter().any(|f| f.kind == "no-tests-ran"),
        "multi-binary output with 3 passed then 0 passed must not emit no-tests-ran; got: {:?}",
        output.findings
    );
    assert_eq!(output.summary.passed, 1);
}

#[test]
fn shell_non_cargo_output_exit_zero_still_passes() {
    let dir = shell_check_repo("exit 0");
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        !output.findings.iter().any(|f| f.kind == "no-tests-ran"),
        "plain exit 0 must not emit no-tests-ran"
    );
    assert_eq!(output.summary.passed, 1);
}

fn cargo_check_repo(cargo_shim_body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/app")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/app")).unwrap();
    fs::write(
        repo.join("openspec/specs/app/spec.md"),
        "# Capability: app\n\n#### Scenario: Cargo token\n- **WHEN** the token is checked\n- **THEN** tests pass\n",
    ).unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        format!("tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncargo = [\"{}\"]\n", repo.join("cargo").display()),
    ).unwrap();
    write_executable(&repo.join("cargo"), cargo_shim_body);
    fs::write(
        repo.join(".espectacular/app/cargo-token.toml"),
        "id = \"cargo-token\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.cargo]]\nflags = \"p_token::case1\"\n",
    ).unwrap();
    dir
}

// Task 4.3 (derive-contracts-from-specodelic): a `flags` binding whose
// runner matched zero tests must fail the gate instead of passing —
// `cargo test -- nonexistent-filter` exits 0 with "0 passed".
#[test]
fn cargo_flags_binding_matching_zero_tests_emits_no_tests_ran() {
    let dir = cargo_check_repo("printf 'test result: ok. 0 passed; 1 filtered out; 0 finished\\n'");
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "no-tests-ran"),
        "expected no-tests-ran for zero-matched cargo binding; got: {:?}",
        output.findings
    );
    assert_eq!(output.summary.passed, 0);
}

#[test]
fn cargo_flags_binding_with_passes_still_counts_passed() {
    let dir = cargo_check_repo("printf 'test result: ok. 2 passed; 0 failed\\n'");
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(
        !output.findings.iter().any(|f| f.kind == "no-tests-ran"),
        "unexpected no-tests-ran for positive cargo output"
    );
    assert_eq!(output.summary.passed, 1);
}
