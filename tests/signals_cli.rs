use assert_cmd::Command;
use std::fs;

fn ah() -> Command {
    Command::cargo_bin("ah").unwrap()
}

fn write_rejection_event(project_root: &std::path::Path, violations: &[(&str, &str)]) {
    let events_dir = project_root.join(".dont").join("events");
    fs::create_dir_all(&events_dir).unwrap();
    let vs: serde_json::Value = violations
        .iter()
        .map(|(id, detail)| serde_json::json!({"entity_id": id, "detail": detail}))
        .collect::<Vec<_>>()
        .into();
    let event = serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": "1.0",
        "tool": "dont",
        "event_kind": "rule_rejection",
        "rule_name": "ungrounded",
        "timestamp": "2026-06-10T12:00:00Z",
        "violations": vs
    }))
    .unwrap();
    fs::write(events_dir.join("rejection-ungrounded.json"), event).unwrap();
}

#[test]
fn signals_returns_empty_array_with_no_events() {
    let dir = tempfile::tempdir().unwrap();
    let output = ah()
        .arg("signals")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(v.as_array().unwrap().is_empty());
}

#[test]
fn signals_returns_drift_signals_for_ungrounded_event() {
    let dir = tempfile::tempdir().unwrap();
    write_rejection_event(
        dir.path(),
        &[("claim:abc", "depends on unresolved CURIE 'foo:bar'")],
    );
    let output = ah()
        .arg("signals")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let signals = v.as_array().unwrap();
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0]["rule_name"], "ungrounded");
    assert_eq!(signals[0]["signal_kind"], "dont-rejection");
    assert_eq!(signals[0]["violation_count"], 1);
    assert_eq!(signals[0]["violations"][0]["entity_id"], "claim:abc");
    assert!(signals[0]["openspec_hint"]
        .as_str()
        .unwrap()
        .contains("CURIE"));
}

#[test]
fn signals_output_is_valid_json_array() {
    let dir = tempfile::tempdir().unwrap();
    write_rejection_event(
        dir.path(),
        &[("claim:x", "bad dep"), ("claim:y", "bad dep 2")],
    );
    let output = ah()
        .arg("signals")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(v.is_array());
    let signals = v.as_array().unwrap();
    assert_eq!(signals.len(), 1); // one event file → one signal
    assert_eq!(signals[0]["violation_count"], 2);
}
