use assert_cmd::Command;
use std::fs;

fn create_simple_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    fs::write(
        root.join(".espectacular/config.toml"),
        r#"tool_version = "0.5.0"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
"#,
    )
    .unwrap();
    // Write an AGENTS.md with the managed block so doctor doesn't complain
    fs::write(
        root.join("AGENTS.md"),
        "# Project\n\n<!-- ah:managed:start -->\n<!-- ah:managed:end -->\n",
    )
    .unwrap();
    dir
}

#[test]
fn ah_check_json_emits_shared_envelope() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "check"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    // Shared envelope shape
    assert!(json.get("ok").is_some(), "must have 'ok'");
    assert!(
        json.get("envelope_version").is_some(),
        "must have 'envelope_version'"
    );
    assert!(json.get("cli_version").is_some(), "must have 'cli_version'");
    assert!(
        json.get("envelope_kind").is_some(),
        "must have 'envelope_kind'"
    );
    assert!(json.get("data").is_some(), "must have 'data'");
    assert!(json.get("warnings").is_some(), "must have 'warnings'");
    assert!(json.get("hints").is_some(), "must have 'hints'");
    assert!(json.get("meta").is_some(), "must have 'meta'");

    // Existing findings/summary nested under data
    let data = json.get("data").unwrap();
    assert!(data.get("findings").is_some(), "data must have 'findings'");
    assert!(data.get("summary").is_some(), "data must have 'summary'");
}

#[test]
fn ah_doctor_json_emits_shared_envelope() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "doctor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert!(json.get("ok").is_some(), "must have 'ok'");
    assert!(
        json.get("envelope_version").is_some(),
        "must have 'envelope_version'"
    );
    assert!(
        json.get("envelope_kind").is_some(),
        "must have 'envelope_kind'"
    );
    assert!(json.get("data").is_some(), "must have 'data'");
}

#[test]
fn ah_report_json_emits_shared_envelope() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "report"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert!(json.get("ok").is_some(), "must have 'ok'");
    assert!(
        json.get("envelope_version").is_some(),
        "must have 'envelope_version'"
    );
    assert!(
        json.get("envelope_kind").is_some(),
        "must have 'envelope_kind'"
    );
    assert!(json.get("data").is_some(), "must have 'data'");
}

#[test]
fn ah_signals_json_emits_shared_envelope() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "signals"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert!(json.get("ok").is_some(), "must have 'ok'");
    assert!(
        json.get("envelope_version").is_some(),
        "must have 'envelope_version'"
    );
    assert!(
        json.get("envelope_kind").is_some(),
        "must have 'envelope_kind'"
    );
    assert!(json.get("data").is_some(), "must have 'data'");
}
