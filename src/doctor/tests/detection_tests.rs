//! Framework detection and capability recommendation tests.

use super::super::runner::run_doctor;
use super::super::types::TOOL_VERSION;
use super::fixtures::{issues, make_healthy_repo};
use crate::adapters::DetectionSource;
use std::fs;

#[test]
fn configured_pytest_runner_appears_in_detections() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\npytest = [\"pytest\"]\n"
        ),
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        report
            .detections
            .iter()
            .any(|d| d.name == "pytest" && d.detection_source == DetectionSource::Configured),
        "configured pytest must appear in detections; got: {:?}",
        report.detections
    );
}

#[test]
fn configured_cargo_runner_appears_in_detections() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\ncargo = [\"cargo\", \"test\"]\n"
        ),
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        report
            .detections
            .iter()
            .any(|d| d.name == "cargo" && d.detection_source == DetectionSource::Configured),
        "configured cargo must appear in detections; got: {:?}",
        report.detections
    );
}

#[test]
fn configured_vitest_runner_appears_in_detections() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\nvitest = [\"vitest\", \"run\"]\n"
        ),
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        report
            .detections
            .iter()
            .any(|d| d.name == "vitest" && d.detection_source == DetectionSource::Configured),
        "configured vitest must appear in detections; got: {:?}",
        report.detections
    );
}

#[test]
fn configured_property_capability_appears_in_detections() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n[capabilities.property]\nenabled = true\n"
        ),
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        report
            .detections
            .iter()
            .any(|d| d.name == "property" && d.detection_source == DetectionSource::Configured),
        "configured property capability must appear in detections; got: {:?}",
        report.detections
    );
}

#[test]
fn framework_detection_does_not_affect_healthy_flag() {
    let repo = make_healthy_repo();
    fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        report.genesis_report.is_healthy(),
        "recommendations must not make healthy=false; diagnostics: {:?}",
        issues(&report)
    );
}

#[test]
fn pytest_manifest_detected_but_not_configured_emits_recommendation() {
    let repo = make_healthy_repo();
    fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
    let report = run_doctor(repo.path()).unwrap();
    let rec = report.suggestions.iter().find(|r| r.capability == "pytest");
    assert!(
        rec.is_some(),
        "pytest detected via manifest must emit recommendation; got: {:?}",
        report.suggestions
    );
    let rec = rec.unwrap();
    assert_eq!(
        rec.apply_command, "ah doctor --enable pytest",
        "recommendation apply_command must be --enable invocation"
    );
}

#[test]
fn cargo_manifest_detected_but_not_configured_emits_recommendation() {
    let repo = make_healthy_repo();
    fs::write(repo.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    let report = run_doctor(repo.path()).unwrap();
    let rec = report.suggestions.iter().find(|r| r.capability == "cargo");
    assert!(
        rec.is_some(),
        "cargo detected via manifest must emit recommendation; got: {:?}",
        report.suggestions
    );
    assert_eq!(rec.unwrap().apply_command, "ah doctor --enable cargo");
}

#[test]
fn vitest_manifest_detected_but_not_configured_emits_recommendation() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join("package.json"),
        r#"{"devDependencies":{"vitest":"^1.0"}}"#,
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    let rec = report.suggestions.iter().find(|r| r.capability == "vitest");
    assert!(
        rec.is_some(),
        "vitest detected via manifest must emit recommendation; got: {:?}",
        report.suggestions
    );
    assert_eq!(rec.unwrap().apply_command, "ah doctor --enable vitest");
}

#[test]
fn property_hypothesis_detected_emits_recommendation() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join("pyproject.toml"),
        "[tool.pytest.ini_options]\n\n[project]\ndependencies = [\"hypothesis\"]\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    let rec = report
        .suggestions
        .iter()
        .find(|r| r.capability == "property");
    assert!(
        rec.is_some(),
        "hypothesis in pyproject must emit property recommendation; got: {:?}",
        report.suggestions
    );
    assert_eq!(rec.unwrap().apply_command, "ah doctor --enable property");
}

#[test]
fn configured_framework_emits_detection_not_recommendation() {
    let repo = make_healthy_repo();
    fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\npytest = [\"pytest\"]\n"
        ),
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        report
            .detections
            .iter()
            .any(|d| d.name == "pytest" && d.detection_source == DetectionSource::Configured),
        "configured pytest should be in detections"
    );
    assert!(
        !report.suggestions.iter().any(|r| r.capability == "pytest"),
        "configured pytest must NOT be in recommendations; got: {:?}",
        report.suggestions
    );
}
