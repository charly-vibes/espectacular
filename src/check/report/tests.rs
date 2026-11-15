use super::{report_finding, ReportFinding, ScenarioContext};
use crate::check::run_check;
use crate::check::test_support::success_repo;
use std::fs;

#[test]
fn non_liveness_findings_serialize_error_severity() {
    let dir = success_repo();
    let contract = dir.path().join(".espectacular/compiler/green-path.toml");
    let text = fs::read_to_string(&contract).unwrap();
    fs::write(
        &contract,
        text.replace(
            "status = \"active\"",
            "status = \"active\"\nfalsifiability_class = \"eventual\"",
        ),
    )
    .unwrap();

    let output = run_check(dir.path(), &[], true).unwrap();
    let findings: Vec<_> = output
        .findings
        .iter()
        .filter(|f| f.kind == "invalid-falsifiability-class")
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "error");
    // byte-compatibility: serialized JSON keeps every pre-existing field
    let json = serde_json::to_value(findings[0].clone()).unwrap();
    for key in [
        "kind",
        "category",
        "spec",
        "spec_path",
        "scenario",
        "suggested_action",
        "playbook_command",
        "message",
    ] {
        assert!(json.get(key).is_some(), "missing pre-existing field {key}");
    }
    // external (custom-runner) findings without severity deserialize as error
    let external: ReportFinding =
        serde_json::from_value(serde_json::json!({"kind": "test-failing", "category": "execution", "spec": "s", "spec_path": "p", "scenario": {"id": "i", "title": "t", "body_markdown": "b"}, "suggested_action": "edit_code_not_scenario", "playbook_command": "ah explain edit_code_not_scenario"})).unwrap();
    assert_eq!(external.severity, "error");
}

/// One pre-existing kind: error severity plus every legacy JSON field.
fn assert_preexisting_kind_serializes(kind: &str) {
    let finding = report_finding(
        kind,
        "structural",
        "s".to_string(),
        "p".to_string(),
        ScenarioContext {
            id: "i".to_string(),
            title: "t".to_string(),
            body_markdown: "b".to_string(),
        },
        None,
        None,
        None,
    );
    assert_eq!(finding.severity, "error", "kind {kind} must be error");
    let json = serde_json::to_value(&finding).unwrap();
    for key in [
        "kind",
        "category",
        "spec",
        "spec_path",
        "scenario",
        "suggested_action",
        "playbook_command",
    ] {
        assert!(
            json.get(key).is_some(),
            "kind {kind}: missing pre-existing field {key}"
        );
    }
}

#[test]
fn every_preexisting_finding_kind_serializes_error_severity() {
    // All kinds that existed before the severity field (schema enum minus
    // the two falsifiability kinds introduced alongside it).
    let kinds = [
        "no-toml",
        "orphan-toml",
        "slug-collision",
        "id-mismatch",
        "invalid-status",
        "no-tests-declared",
        "missing-runner",
        "malformed-contract",
        "missing-replacement",
        "overlay-conflict",
        "test-failing",
        "no-tests-ran",
    ];
    for kind in kinds {
        assert_preexisting_kind_serializes(kind);
    }
}
