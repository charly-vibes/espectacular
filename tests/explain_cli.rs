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
    Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "--list"])
        .assert()
        .success()
        .stdout(contains("test-failing"))
        .stdout(contains("edit_code_not_scenario"))
        .stdout(contains("workflow"))
        .stdout(contains("adapter-pytest"));
}

// 9.3 — explain a finding kind prints markdown body
#[test]
fn ah_explain_finding_kind_prints_body() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "test-failing"])
        .assert()
        .success()
        .stdout(contains("## test-failing"))
        .stdout(contains("non-zero"));
}

// 9.3 — explain a suggested action prints markdown body
#[test]
fn ah_explain_suggested_action_prints_body() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "edit_code_not_scenario"])
        .assert()
        .success()
        .stdout(contains("## edit_code_not_scenario"))
        .stdout(contains("implementation"));
}

// 9.3 — quality finding kinds have bodies
#[test]
fn ah_explain_quality_finding_kinds() {
    for slug in &["quality-mutation", "quality-property", "quality-snapshot"] {
        Command::cargo_bin("ah")
            .unwrap()
            .args(["explain", slug])
            .assert()
            .success()
            .stdout(contains("##"));
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
    ] {
        Command::cargo_bin("ah")
            .unwrap()
            .args(["explain", slug])
            .assert()
            .success()
            .stdout(contains("##"));
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
    Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "totally-unknown-xyzzy"])
        .assert()
        .failure()
        .stderr(contains("unknown topic"));
}

// 9.9 — unknown topic with partial match shows did-you-mean
#[test]
fn ah_explain_unknown_topic_did_you_mean() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["explain", "test"])
        .assert()
        .failure()
        .stderr(contains("Did you mean"))
        .stderr(contains("test-failing"));
}

// 9.11 — adapter topics are present and have content
#[test]
fn ah_explain_adapter_topics() {
    for slug in &["adapter-pytest", "adapter-cargo", "adapter-vitest"] {
        Command::cargo_bin("ah")
            .unwrap()
            .args(["explain", slug])
            .assert()
            .success()
            .stdout(contains("##"));
    }
}
