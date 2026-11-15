//! Lint engine, registry, path-resolution, and check-firing tests.
//!
//! Walker tests live in `walker.rs`; bridge tests live in `bridge/tests.rs`.

use super::walker::SpecFile;
use super::*;
use std::path::PathBuf;

fn clean_specs() -> PathBuf {
    PathBuf::from("tests/fixtures/lint/clean/openspec/specs")
}

fn defective_specs() -> PathBuf {
    PathBuf::from("tests/fixtures/lint/defective/openspec/specs")
}

/// Stub check proving the walker → registry → findings pipeline flows.
struct StubCheck {
    kind: &'static str,
}

impl LintCheck for StubCheck {
    fn kind(&self) -> &'static str {
        self.kind
    }
    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        findings.push(LintFinding::warning(
            self.kind,
            &spec.spec_path,
            "",
            "stub message",
            "stub suggestion",
            "edit_spec",
            "ah explain lint",
        ));
    }
}

#[test]
fn lint_finding_serializes_shared_schema_fields() {
    let finding = LintFinding::warning(
        "vague-qualifier",
        "auth",
        "user-login",
        "unbound qualifier 'fast'",
        "add a measurable bound (e.g. within 200 ms)",
        "edit_spec",
        "ah explain vague-qualifier",
    );
    let json = serde_json::to_value(&finding).unwrap();
    for field in [
        "kind",
        "severity",
        "spec_path",
        "scenario_id",
        "message",
        "suggestion",
        "suggested_action",
        "playbook_command",
    ] {
        assert!(json.get(field).is_some(), "missing field: {field}");
    }
    assert_eq!(json["severity"], "warning");
}

#[test]
fn stub_engine_flows_findings_through_pipeline() {
    let stubs: Vec<Box<dyn LintCheck>> = vec![Box::new(StubCheck { kind: "stub-kind" })];
    let output = run_lint_with(&defective_specs(), &stubs).unwrap();
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.findings[0].kind, "stub-kind");
    assert_eq!(output.findings[0].severity, Severity::Warning);
    assert_eq!(output.counts_by_kind.get("stub-kind"), Some(&1));
}

#[test]
fn clean_fixture_produces_zero_findings() {
    let output = run_lint_query(&clean_specs(), None).unwrap();
    assert!(output.findings.is_empty());
    assert!(output.counts_by_kind.is_empty());
}

#[test]
fn specs_dir_for_missing_config_falls_back_to_openspec_layout() {
    // Lint fixture repos carry no .espectacular/config.toml — adoption
    // fallback must resolve <root>/openspec/specs.
    let clean_root = clean_specs()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let dir = specs_dir_for(&clean_root).unwrap();
    assert_eq!(
        dir,
        PathBuf::from("tests/fixtures/lint/clean/openspec/specs")
    );
}

#[test]
fn select_checks_without_filter_registers_every_kind() {
    let registered = select_checks(&crate::config::LintConfig::default(), None).unwrap();
    let kinds: Vec<&str> = registered.iter().map(|c| c.kind()).collect();
    for kind in [
        "vague-qualifier",
        "imperative-step",
        "conjunctive-bloat",
        "missing-negative-scenario",
        "missing-non-goals",
        "unresolved-ambiguity",
        "entangled-spec",
    ] {
        assert!(kinds.contains(&kind), "missing kind {kind} in {kinds:?}");
    }
}

#[test]
fn select_checks_filter_runs_single_kind() {
    let registered = select_checks(
        &crate::config::LintConfig::default(),
        Some("vague-qualifier"),
    )
    .unwrap();
    assert_eq!(registered.len(), 1);
    let output = run_lint_with(&defective_specs(), &registered).unwrap();
    assert!(!output.findings.is_empty());
    assert!(output.findings.iter().all(|f| f.kind == "vague-qualifier"));
}

#[test]
fn select_checks_unknown_kind_errors_listing_valid_kinds() {
    let err = select_checks(&crate::config::LintConfig::default(), Some("no-such-check"))
        .map(|_| ())
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no-such-check"));
    assert!(
        msg.contains("vague-qualifier"),
        "must list valid kinds: {msg}"
    );
}

#[test]
fn merge_outputs_concats_findings_and_sums_counts() {
    let a = run_lint_with(&clean_specs(), &[Box::new(StubCheck { kind: "k-a" })]).unwrap();
    let b = run_lint_with(&defective_specs(), &[Box::new(StubCheck { kind: "k-b" })]).unwrap();
    let merged = merge_outputs(a, b);
    assert_eq!(merged.findings.len(), 2);
    assert_eq!(merged.findings[0].kind, "k-a");
    assert_eq!(merged.findings[1].kind, "k-b");
    assert_eq!(merged.counts_by_kind.get("k-a"), Some(&1));
    assert_eq!(merged.counts_by_kind.get("k-b"), Some(&1));
}

#[test]
fn exit_code_zero_on_warnings_and_empty_non_zero_on_errors() {
    let empty = LintOutput {
        findings: vec![],
        counts_by_kind: BTreeMap::new(),
    };
    assert_eq!(exit_code(&empty), 0);
    let warning_only =
        LintFinding::warning("k", "spec", "", "m", "s", "edit_spec", "ah explain lint");
    let warnings = LintOutput {
        findings: vec![warning_only.clone()],
        counts_by_kind: BTreeMap::from([("k".into(), 1)]),
    };
    assert_eq!(exit_code(&warnings), 0);
    let mut errored = warning_only;
    errored.severity = Severity::Error;
    let errors = LintOutput {
        findings: vec![errored],
        counts_by_kind: BTreeMap::from([("k".into(), 1)]),
    };
    assert_eq!(exit_code(&errors), 1);
}

#[test]
fn malformed_spec_file_emits_error_finding_and_other_specs_still_lint() {
    // Deployed lint spec C-finding-schema: "a malformed spec file emits a
    // severity = "error" finding and exits non-zero" (espectacular-pt8).
    let tmp = tempfile::tempdir().unwrap();
    let specs = tmp.path().join("specs");
    std::fs::create_dir_all(specs.join("good")).unwrap();
    std::fs::create_dir_all(specs.join("bad")).unwrap();
    std::fs::write(
        specs.join("good/spec.md"),
        "### Requirement: UX\nThe system SHALL show the result.\n\n#### Scenario: Feedback\n- **WHEN** the result arrives\n- **THEN** the response is user-friendly\n",
    )
    .unwrap();
    // Invalid UTF-8 — the file cannot be read as a spec (malformed).
    std::fs::write(specs.join("bad/spec.md"), b"\xff\xfe broken bytes").unwrap();

    let registered = select_checks(&crate::config::LintConfig::default(), None).unwrap();
    let output = run_lint_with(&specs, &registered).unwrap();

    let errors: Vec<_> = output
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    assert_eq!(errors.len(), 1, "exactly one error finding: {errors:?}");
    assert_eq!(errors[0].kind, "malformed-spec");
    assert_eq!(errors[0].spec_path, "bad");
    assert_eq!(errors[0].playbook_command, "ah explain lint");
    assert_eq!(exit_code(&output), 1);
    // The readable spec is still linted — no hard failure, no lost coverage.
    let good_warnings: Vec<_> = output
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning && f.spec_path == "good")
        .collect();
    assert!(!good_warnings.is_empty());
}

/// Dual-format spec with a dangling VERIFIES link, an unlinked scenario, and
/// an untraced property.
const TRACE_SPEC: &str = "---\nid: spec\nkind: intent\nstatement: \"WHEN x THE system SHALL y.\"\n---\n\n# Spec\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-a | unit | [[spec.C-a]] | input | output |\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n\n#### Scenario: dangling\n- **WHEN** a token is checked\n- **VERIFIES** [[spec.P-missing]]\n\n#### Scenario: unlinked\n- **WHEN** a session expires\n- **THEN** the user is logged out\n";

fn trace_kinds(output: &LintOutput) -> Vec<&str> {
    output
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.kind.as_str(),
                "verifies-dangling" | "scenario-unlinked" | "property-untraced"
            )
        })
        .map(|f| f.kind.as_str())
        .collect()
}

fn assert_trace_findings_advisory(output: &LintOutput) {
    let kinds = trace_kinds(output);
    for kind in [
        "verifies-dangling",
        "scenario-unlinked",
        "property-untraced",
    ] {
        assert!(kinds.contains(&kind), "{kind} flagged: {kinds:?}");
    }
    assert_eq!(
        exit_code(output),
        0,
        "advisory trace findings must not gate"
    );
}

#[test]
fn trace_findings_are_advisory_and_exit_zero() {
    // Deployed derive-contracts lint delta (task 5.3): verifies-dangling,
    // scenario-unlinked, and property-untraced never affect the exit code.
    let tmp = tempfile::tempdir().unwrap();
    let specs = tmp.path().join("specs");
    std::fs::create_dir_all(specs.join("auth")).unwrap();
    std::fs::write(specs.join("auth/spec.md"), TRACE_SPEC).unwrap();
    let registered = select_checks(&crate::config::LintConfig::default(), None).unwrap();
    let output = run_lint_with(&specs, &registered).unwrap();
    assert_trace_findings_advisory(&output);
}

#[test]
fn changes_dir_for_missing_config_falls_back_to_openspec_layout() {
    let clean_root = clean_specs()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let dir = changes_dir_for(&clean_root).unwrap();
    assert_eq!(
        dir,
        PathBuf::from("tests/fixtures/lint/clean/openspec/changes")
    );
}

#[test]
fn run_change_overlay_unknown_change_is_hard_error() {
    let root = defective_specs()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let err = run_change_overlay(&root, "no-such-change", None).unwrap_err();
    assert!(err.to_string().contains("no-such-change"));
}

#[test]
fn run_change_overlay_lints_change_specs() {
    let root = defective_specs()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let output = run_change_overlay(&root, "add-parser", None).unwrap();
    assert!(output.findings.iter().all(|f| f.spec_path == "parser"));
    assert!(!output.findings.is_empty());
}

#[test]
fn dual_format_fixture_receives_both_prose_and_relayed_spk_findings() {
    // espectacular-zlw (task 7.1) end-to-end against the real spk binary.
    // Skips where spk is not installed (CI) — the shim-based tests cover
    // the relay deterministically.
    let spk_present = std::process::Command::new("spk")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !spk_present {
        return;
    }
    let output = run_lint_query(
        &PathBuf::from("tests/fixtures/lint/dual-format/openspec/specs"),
        None,
    )
    .unwrap();
    assert!(output.findings.iter().any(|f| f.kind.starts_with("spk.")));
    // Prose checks still fire on the same dual-format file (task 7.1).
    assert!(output
        .findings
        .iter()
        .any(|f| f.kind == "vague-qualifier" && f.spec_path == "auth"));
    // Every relayed finding is warning severity (task 7.4).
    assert!(output
        .findings
        .iter()
        .filter(|f| f.kind.starts_with("spk."))
        .all(|f| f.severity == Severity::Warning));
}

#[test]
fn specs_dir_for_uses_config_paths() {
    let repo = tempfile::tempdir().unwrap();
    let cfg_dir = repo.path().join(".espectacular");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        concat!(
            "tool_version = \"",
            env!("CARGO_PKG_VERSION"),
            "\"\n[paths]\nspecs = \"custom/specs\"\nchanges = \"openspec/changes\"\n[runners]\n"
        ),
    )
    .unwrap();
    let dir = specs_dir_for(repo.path()).unwrap();
    assert_eq!(dir, repo.path().join("custom/specs"));
}

// ---- property-derived contract bindings (DDL pilot, design D6) --------

#[test]
fn p_schema() {
    lint_finding_serializes_shared_schema_fields();
    exit_code_zero_on_warnings_and_empty_non_zero_on_errors();
    malformed_spec_file_emits_error_finding_and_other_specs_still_lint();
}
