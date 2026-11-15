use assert_cmd::Command;
use predicates::str::contains;

// 9.9 — --list emits stable sorted output
#[test]
fn ah_explain_list_is_sorted() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "--list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let mut sorted = lines.clone();
    sorted.sort_unstable();
    assert_eq!(lines, sorted, "--list output is not sorted");
}

// 9.9 — --list includes all expected topic categories
#[test]
fn ah_explain_list_includes_all_categories() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "--list"])
        .assert()
        .success()
        .stdout(contains("test-failing"))
        .stdout(contains("edit_code_not_scenario"))
        .stdout(contains("workflow"))
        .stdout(contains("adapter-pytest"))
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    for category in [
        "test-failing",
        "edit_code_not_scenario",
        "workflow",
        "adapter-pytest",
    ] {
        let hits = stdout.lines().filter(|line| *line == category).count();
        assert_eq!(
            hits, 1,
            "--list must print {category} exactly once; got:\n{stdout}"
        );
    }
}

// 9.3 — explain a finding kind prints markdown body
#[test]
fn ah_explain_finding_kind_prints_body() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "test-failing"])
        .assert()
        .success()
        .stdout(contains("## test-failing"))
        .stdout(contains("non-zero"))
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(
        stdout.trim_start().starts_with("## test-failing"),
        "the body must open with its own heading; got:\n{stdout}"
    );
}

// 9.3 — explain a suggested action prints markdown body
#[test]
fn ah_explain_suggested_action_prints_body() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "edit_code_not_scenario"])
        .assert()
        .success()
        .stdout(contains("## edit_code_not_scenario"))
        .stdout(contains("implementation"))
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(
        stdout.trim_start().starts_with("## edit_code_not_scenario"),
        "the body must open with its own heading; got:\n{stdout}"
    );
}

// 9.3 — quality finding kinds have bodies
#[test]
fn ah_explain_quality_finding_kinds() {
    for slug in &["quality-mutation", "quality-property", "quality-snapshot"] {
        let output = Command::cargo_bin("ah")
            .unwrap()
            .args(["explain", slug])
            .assert()
            .success()
            .stdout(contains("##"))
            .get_output()
            .stdout
            .clone();
        let stdout = String::from_utf8(output).unwrap();
        assert!(
            stdout.trim_start().starts_with(&format!("## {slug}")),
            "{slug} body must open with its own heading; got:\n{stdout}"
        );
    }
}

// 9.5 — general topics have bodies
#[test]
fn ah_explain_general_topics() {
    for slug in &[
        "workflow",
        "supersession",
        "archetypes",
        "progressive-enablement",
        "scenario-scoped-tests",
    ] {
        let output = Command::cargo_bin("ah")
            .unwrap()
            .args(["explain", slug])
            .assert()
            .success()
            .stdout(contains("##"))
            .get_output()
            .stdout
            .clone();
        let stdout = String::from_utf8(output).unwrap();
        assert!(
            stdout.trim_start().starts_with(&format!("## {slug}")),
            "{slug} body must open with its own heading; got:\n{stdout}"
        );
    }
}

// 9.7 — --json emits required fields
#[test]
fn ah_explain_json_has_required_fields() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "test-failing", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).expect("invalid JSON");
    assert_eq!(value["topic"], "test-failing");
    assert!(value["summary"].is_string());
    assert!(value["when"].is_string());
    assert!(value["do"].is_string());
    assert!(value["human_approval"].is_boolean());
    assert!(value["related_topics"].is_array());
    assert!(value["hints"].is_array());
}

// 9.7 — hints array items have kind and message
#[test]
fn ah_explain_json_hints_shape() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "test-failing", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let hints = value["hints"].as_array().unwrap();
    for hint in hints {
        assert!(hint["kind"].is_string(), "hint missing 'kind'");
        assert!(hint["message"].is_string(), "hint missing 'message'");
    }
}

// 9.9 — unknown topic exits non-zero
#[test]
fn ah_explain_unknown_topic_exits_nonzero() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "totally-unknown-xyzzy"])
        .assert()
        .failure()
        .stderr(contains("unknown topic"))
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).unwrap();
    assert!(
        stderr.contains("unknown topic: totally-unknown-xyzzy"),
        "the error must name the rejected topic; got:\n{stderr}"
    );
}

// 9.9 — unknown topic with partial match shows did-you-mean
#[test]
fn ah_explain_unknown_topic_did_you_mean() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "test"])
        .assert()
        .failure()
        .stderr(contains("Did you mean"))
        .stderr(contains("test-failing"))
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).unwrap();
    let hint = stderr
        .lines()
        .find(|line| line.contains("Did you mean"))
        .unwrap_or("");
    assert!(
        hint.starts_with("unknown topic: test."),
        "the did-you-mean line must quote the rejected topic; got:\n{stderr}"
    );
}

// 9.11 — adapter topics are present and have content
#[test]
fn ah_explain_adapter_topics() {
    for slug in &["adapter-pytest", "adapter-cargo", "adapter-vitest"] {
        let output = Command::cargo_bin("ah")
            .unwrap()
            .args(["explain", slug])
            .assert()
            .success()
            .stdout(contains("##"))
            .get_output()
            .stdout
            .clone();
        let stdout = String::from_utf8(output).unwrap();
        assert!(
            stdout.trim_start().starts_with(&format!("## {slug}")),
            "{slug} body must open with its own heading; got:\n{stdout}"
        );
    }
}

// 11.4 — ah explain no-toml exits zero with non-empty output
#[test]
fn ah_explain_no_toml_exits_zero_with_content() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "no-toml"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(
        !output.is_empty(),
        "ah explain no-toml produced empty stdout"
    );
}

// espectacular-eia (task 6.1) — `ah explain <lint-kind> --json` returns a
// topic envelope for every lint finding kind.
#[test]
fn ah_explain_lint_kinds_return_topic_envelopes() {
    for kind in [
        "vague-qualifier",
        "imperative-step",
        "conjunctive-bloat",
        "missing-negative-scenario",
        "missing-non-goals",
        "unresolved-ambiguity",
        "entangled-spec",
    ] {
        let output = String::from_utf8(
            Command::cargo_bin("ah")
                .unwrap()
                .args(["explain", kind, "--json"])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone(),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["topic"], kind, "topic mismatch for {kind}");
    }
    // The bridge playbook topic is reachable too.
    Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "lint", "--json"])
        .assert()
        .success();
}

#[test]
fn p_explain() {
    ah_explain_finding_kind_prints_body();
    ah_explain_suggested_action_prints_body();
    ah_explain_general_topics();
    ah_explain_json_has_required_fields();
    ah_explain_list_is_sorted();
    ah_explain_unknown_topic_exits_nonzero();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "--list"])
        .assert();
    assert_eq!(
        assert.get_output().status.code(),
        Some(0),
        "ah explain --list must exit 0"
    );
}
