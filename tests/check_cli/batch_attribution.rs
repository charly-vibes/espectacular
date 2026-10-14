use assert_cmd::Command;
use serde_json::Value;
use std::fs;

use crate::batching::{vitest_pattern_repo, vitest_pattern_repo_with};
use crate::data_from_envelope;
use crate::git_commit_all;
use crate::schema::assert_schema_valid;
use crate::write_executable;

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
