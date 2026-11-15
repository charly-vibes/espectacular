use super::structural_findings;
use crate::check::run_check;
use crate::check::test_support::{success_repo, CONTRACTS, SPECS};
use std::fs;

#[test]
pub(crate) fn four_findings_fixture_emits_exactly_four() {
    let findings = structural_findings(SPECS, CONTRACTS).unwrap();
    // 5 findings: missing-contract, orphan-toml, no-tests-declared, slug-collision x2
    // (both scenarios sharing a colliding slug are now flagged)
    assert_eq!(findings.len(), 5);
}

#[test]
pub(crate) fn findings_ordered_by_spec_scenario_kind() {
    let findings = structural_findings(SPECS, CONTRACTS).unwrap();
    let mut sorted = findings.clone();
    sorted.sort();
    assert_eq!(findings, sorted);
}

#[test]
pub(crate) fn missing_contract_finding_present() {
    let findings = structural_findings(SPECS, CONTRACTS).unwrap();
    assert!(findings
        .iter()
        .any(|f| f.kind == "no-toml" && f.scenario_id == "missing-contract"));
}

#[test]
pub(crate) fn orphan_contract_finding_present() {
    let findings = structural_findings(SPECS, CONTRACTS).unwrap();
    assert!(findings
        .iter()
        .any(|f| f.kind == "orphan-toml" && f.scenario_id == "orphan-contract"));
}

#[test]
fn no_tests_declared_finding_present() {
    let findings = structural_findings(SPECS, CONTRACTS).unwrap();
    assert!(findings
        .iter()
        .any(|f| f.kind == "no-tests-declared" && f.scenario_id == "no-tests-declared"));
}

#[test]
fn duplicate_id_finding_present() {
    let findings = structural_findings(SPECS, CONTRACTS).unwrap();
    assert!(findings.iter().any(|f| f.kind == "slug-collision"));
}

#[test]
pub(crate) fn invalid_falsifiability_class_emits_structural_finding() {
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

    let specs = dir.path().join("openspec/specs");
    let contracts = dir.path().join(".espectacular");
    let findings =
        structural_findings(specs.to_str().unwrap(), contracts.to_str().unwrap()).unwrap();
    assert!(findings
        .iter()
        .any(|f| f.kind == "invalid-falsifiability-class"));
}

#[test]
pub(crate) fn invalid_falsifiability_class_blocks_scenario_execution() {
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
    assert!(output
        .findings
        .iter()
        .any(|f| f.kind == "invalid-falsifiability-class"));
    assert_eq!(output.summary.passed, 0, "declared tests must not run");
}

#[test]
fn valid_falsifiability_class_values_pass_and_run_tests() {
    // safety: no findings at all
    let safety = success_repo();
    let safety_contract = safety.path().join(".espectacular/compiler/green-path.toml");
    let safety_text = fs::read_to_string(&safety_contract).unwrap();
    fs::write(
        &safety_contract,
        safety_text.replace(
            "status = \"active\"",
            "status = \"active\"\nfalsifiability_class = \"safety\"",
        ),
    )
    .unwrap();
    let output = run_check(safety.path(), &[], true).unwrap();
    assert!(
        output.findings.is_empty(),
        "falsifiability_class = safety must not produce findings"
    );
    assert_eq!(output.summary.passed, 1);

    // liveness: only the timeout warning (entry in success_repo has no timeout)
    let liveness = success_repo();
    let liveness_contract = liveness
        .path()
        .join(".espectacular/compiler/green-path.toml");
    let liveness_text = fs::read_to_string(&liveness_contract).unwrap();
    fs::write(
        &liveness_contract,
        liveness_text.replace(
            "status = \"active\"",
            "status = \"active\"\nfalsifiability_class = \"liveness\"",
        ),
    )
    .unwrap();
    let output = run_check(liveness.path(), &[], true).unwrap();
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.findings[0].kind, "missing-liveness-timeout");
    assert_eq!(output.findings[0].severity, "warning");
    assert_eq!(output.summary.passed, 1);
}

#[test]
pub(crate) fn liveness_without_timeout_emits_warning_severity() {
    let dir = success_repo();
    let contract = dir.path().join(".espectacular/compiler/green-path.toml");
    let text = fs::read_to_string(&contract).unwrap();
    fs::write(
        &contract,
        text.replace(
            "status = \"active\"",
            "status = \"active\"\nfalsifiability_class = \"liveness\"",
        ),
    )
    .unwrap();

    let output = run_check(dir.path(), &[], true).unwrap();
    let warnings: Vec<_> = output
        .findings
        .iter()
        .filter(|f| f.kind == "missing-liveness-timeout")
        .collect();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].severity, "warning");
    assert_eq!(warnings[0].category, "warning");
    assert_eq!(
        output.summary.structural, 0,
        "warning must not be structural"
    );
    assert_eq!(output.summary.passed, 1, "tests still run and pass");
}

#[test]
pub(crate) fn liveness_with_timeout_emits_no_warning() {
    let dir = success_repo();
    let contract = dir.path().join(".espectacular/compiler/green-path.toml");
    let text = fs::read_to_string(&contract).unwrap();
    fs::write(
        &contract,
        text.replace(
            "[[tests.unit]]\nflags = \"ok\"",
            "falsifiability_class = \"liveness\"\n\n[[tests.unit]]\nflags = \"ok\"\ntimeout_seconds = 30",
        ),
    )
    .unwrap();

    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(!output
        .findings
        .iter()
        .any(|f| f.kind == "missing-liveness-timeout"));
    assert_eq!(output.summary.passed, 1);
}

#[test]
pub(crate) fn structural_findings_carry_error_severity() {
    let dir = success_repo();
    let contract = dir.path().join(".espectacular/compiler/green-path.toml");
    fs::write(&contract, "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\nfalsifiability_class = \"liveness\"\n[tests]\n").unwrap();

    let output = run_check(dir.path(), &[], true).unwrap();
    let declared: Vec<_> = output
        .findings
        .iter()
        .filter(|f| f.kind == "no-tests-declared")
        .collect();
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].severity, "error");
    assert!(
        !output
            .findings
            .iter()
            .any(|f| f.kind == "missing-liveness-timeout"),
        "zero test entries must not emit the liveness warning"
    );
}
