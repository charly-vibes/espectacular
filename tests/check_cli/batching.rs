use assert_cmd::Command;
use serde_json::Value;
use std::fs;

use crate::data_from_envelope;
use crate::git_commit_all;
use crate::schema::assert_schema_valid;
use crate::write_executable;

// ── batched vitest bindings (batch-runner-spawns, GH#40) ─────────────────────

// Fixture: a repo with vitest-bound contracts (scenario-scoped
// `--testNamePattern=p_<id>`, the tambor shape — pattern-scoped, no file
// notion) plus a vitest shim that appends one line per invocation to
// `invocations.log` and emits the vitest JSON-reporter shape with each test's
// declared status, so per-contract attribution from structured output is
// exercisable. A status of `None` omits the test from the report entirely
// (matched-zero shape).
pub fn vitest_pattern_repo_with(statuses: &[(&str, Option<&str>)]) -> tempfile::TempDir {
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
    git_commit_all(repo);

    dir
}

pub fn vitest_pattern_repo(n: usize) -> tempfile::TempDir {
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
