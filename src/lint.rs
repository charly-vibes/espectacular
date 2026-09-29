//! Spec quality linter (`ah lint`).
//!
//! Purpose: statically analyze OpenSpec scenario files for authoring defects
//! (vague qualifiers, imperative steps, conjunctive bloat, missing negative
//! scenarios, missing non-goals, unresolved ambiguity, entangled specs) and
//! emit advisory findings in the shared finding schema.
//!
//! Responsibilities:
//! - define `LintFinding` reusing the shared finding schema fields
//! - run the check registry over the spec walker output
//! - produce a `LintOutput` consumable by the CLI (human + JSON envelope)
//!
//! Rationale: `ah check` verifies spec-test correspondence, but nothing
//! evaluates intrinsic scenario quality at authoring time. Lint findings are
//! advisory (warning severity) in v1 — `ah lint` never gates.
//!
//! Registry note: this module is the registry — espectacular-rty/aar append
//! checks additively to [`checks`]; no check implementations live here.

pub mod checks;
pub mod walker;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use walker::SpecFile;

/// Finding severity. All lint findings are `Warning` in v1; `Error` is
/// reserved for structural issues such as malformed spec files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Warning,
    Error,
}

/// A lint finding reusing the shared finding schema fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintFinding {
    pub kind: String,
    pub severity: Severity,
    pub spec_path: String,
    pub scenario_id: String,
    pub message: String,
    pub suggestion: String,
    pub suggested_action: String,
    pub playbook_command: String,
}

impl LintFinding {
    /// Build a warning-severity finding (the only severity checks emit in v1).
    /// Unused until the first check lands (espectacular-rty) — kept as the
    /// canonical constructor for the registry's additions.
    #[allow(dead_code)]
    pub fn warning(
        kind: &str,
        spec_path: &str,
        scenario_id: &str,
        message: &str,
        suggestion: &str,
        suggested_action: &str,
        playbook_command: &str,
    ) -> Self {
        LintFinding {
            kind: kind.to_string(),
            severity: Severity::Warning,
            spec_path: spec_path.to_string(),
            scenario_id: scenario_id.to_string(),
            message: message.to_string(),
            suggestion: suggestion.to_string(),
            suggested_action: suggested_action.to_string(),
            playbook_command: playbook_command.to_string(),
        }
    }
}

/// Lint result: findings plus per-kind counts (mirrors `ah check`'s summary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintOutput {
    pub findings: Vec<LintFinding>,
    pub counts_by_kind: BTreeMap<String, usize>,
}

/// A lint check: inspects one walked spec file and appends findings.
///
/// Implementations live in `src/lint/checks/` (espectacular-rty/aar) and are
/// registered in [`checks`]. Advisory-only: emit `Severity::Warning`.
pub trait LintCheck: Send + Sync {
    /// Stable check kind — matches finding `kind` values and `--check` args.
    fn kind(&self) -> &'static str;
    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>);
}

/// The check registry. Append additively (one entry per check), do not
/// reorder — finding order follows spec-then-check iteration.
pub(crate) fn checks(cfg: &crate::config::LintConfig) -> Vec<Box<dyn LintCheck>> {
    vec![
        Box::new(checks::flow::VagueQualifierCheck),
        Box::new(checks::flow::ImperativeStepCheck),
        Box::new(checks::flow::ConjunctiveBloatCheck {
            max_and_steps: cfg.max_and_steps,
        }),
        Box::new(checks::flow::MissingNegativeScenarioCheck),
        Box::new(checks::shape::MissingNonGoalsCheck),
        Box::new(checks::shape::UnresolvedAmbiguityCheck),
        Box::new(checks::shape::EntangledSpecCheck),
    ]
}

/// Resolve the `[lint]` config for a specs directory by walking up to the
/// nearest repo root carrying an espectacular config. Falls back to defaults
/// when no config exists (adoption-friendly, mirroring `specs_dir_for`).
fn lint_config_for(specs_dir: &Path) -> anyhow::Result<crate::config::LintConfig> {
    for dir in specs_dir.ancestors() {
        if dir.join(crate::config::CONFIG_MARKER).exists() {
            return Ok(crate::config::load(dir)?.lint);
        }
    }
    Ok(crate::config::LintConfig::default())
}

/// Registered checks, optionally filtered to a single kind (`--check`).
/// An unknown kind is a hard error listing the valid kinds.
pub(crate) fn select_checks(
    cfg: &crate::config::LintConfig,
    check: Option<&str>,
) -> anyhow::Result<Vec<Box<dyn LintCheck>>> {
    let all = checks(cfg);
    let Some(kind) = check else {
        return Ok(all);
    };
    if !all.iter().any(|c| c.kind() == kind) {
        let valid = all.iter().map(|c| c.kind()).collect::<Vec<_>>().join(", ");
        anyhow::bail!("unknown check kind '{kind}' — valid kinds: {valid}");
    }
    Ok(all.into_iter().filter(|c| c.kind() == kind).collect())
}

/// Run lint with an optional single-kind filter (`--check <kind>`).
pub fn run_lint_query(specs_dir: &Path, check: Option<&str>) -> anyhow::Result<LintOutput> {
    let cfg = lint_config_for(specs_dir)?;
    let registered = select_checks(&cfg, check)?;
    run_lint_with(specs_dir, &registered)
}

/// Resolve the changes directory for a repo root (mirrors `specs_dir_for`).
pub fn changes_dir_for(repo_root: &Path) -> anyhow::Result<std::path::PathBuf> {
    if repo_root.join(crate::config::CONFIG_MARKER).exists() {
        let cfg = crate::config::load(repo_root)?;
        return Ok(repo_root.join(&cfg.paths.changes));
    }
    Ok(repo_root.join("openspec/changes"))
}

/// Lint one change overlay (`--changes <id>`): the change's spec directory
/// (`<changes>/<id>/specs/<capability>/spec.md`) is walked with the same
/// walker and checks as deployed specs. A missing change directory is a hard
/// error (typo protection, mirroring `archive`).
pub fn run_change_overlay(
    repo_root: &Path,
    change: &str,
    check: Option<&str>,
) -> anyhow::Result<LintOutput> {
    let overlay_specs = changes_dir_for(repo_root)?.join(change).join("specs");
    anyhow::ensure!(
        overlay_specs.is_dir(),
        "no change spec overlay found for '{change}' (expected {})",
        overlay_specs.display()
    );
    run_lint_query(&overlay_specs, check)
}

/// Merge deployed and overlay lint outputs: findings concatenate (deployed
/// first), per-kind counts sum.
pub fn merge_outputs(deployed: LintOutput, overlay: LintOutput) -> LintOutput {
    let mut findings = deployed.findings;
    findings.extend(overlay.findings);
    let mut counts_by_kind = deployed.counts_by_kind;
    for (kind, n) in overlay.counts_by_kind {
        *counts_by_kind.entry(kind).or_insert(0) += n;
    }
    LintOutput {
        findings,
        counts_by_kind,
    }
}

/// Exit-code semantics (task 4.5): zero on warning-only or empty findings;
/// non-zero only when error-severity findings are present. Warning findings
/// never gate (v1 advisory).
pub fn exit_code(output: &LintOutput) -> i32 {
    if output
        .findings
        .iter()
        .any(|f| f.severity == Severity::Error)
    {
        1
    } else {
        0
    }
}

/// Run explicit checks over the specs under `specs_dir` (test seam).
pub(crate) fn run_lint_with(
    specs_dir: &Path,
    registered: &[Box<dyn LintCheck>],
) -> anyhow::Result<LintOutput> {
    let specs = walker::walk_specs(specs_dir)?;
    let mut findings = Vec::new();
    for spec in &specs {
        for check in registered {
            check.check(spec, &mut findings);
        }
    }
    let mut counts_by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for finding in &findings {
        *counts_by_kind.entry(finding.kind.clone()).or_insert(0) += 1;
    }
    Ok(LintOutput {
        findings,
        counts_by_kind,
    })
}

/// Resolve the specs directory for a repo root.
///
/// Falls back to the plain openspec layout (`openspec/specs`) when no
/// espectacular config exists — lint is adoption-friendly and must work in
/// any openspec repo. An existing-but-invalid config is a hard error.
pub fn specs_dir_for(repo_root: &Path) -> anyhow::Result<std::path::PathBuf> {
    if repo_root.join(crate::config::CONFIG_MARKER).exists() {
        let cfg = crate::config::load(repo_root)?;
        return Ok(repo_root.join(&cfg.paths.specs));
    }
    Ok(repo_root.join("openspec/specs"))
}

/// Human-readable lint report. Findings go to stdout; stderr stays clean
/// for the error path (ErrorSink in main).
pub fn print_report(output: &LintOutput) {
    for f in &output.findings {
        let severity = match f.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        println!(
            "{severity} [{}] {}/{}: {}",
            f.kind, f.spec_path, f.scenario_id, f.message
        );
        println!(
            "  suggestion: {} (run: {})",
            f.suggestion, f.playbook_command
        );
    }
    let warnings = output
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();
    let errors = output
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    println!(
        "lint: {} findings ({} warnings, {} errors)",
        output.findings.len(),
        warnings,
        errors
    );
}

#[cfg(test)]
mod tests {
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
    fn specs_dir_for_uses_config_paths() {
        let repo = tempfile::tempdir().unwrap();
        let cfg_dir = repo.path().join(".espectacular");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("config.toml"),
            "tool_version = \"0.6.0\"\n[paths]\nspecs = \"custom/specs\"\nchanges = \"openspec/changes\"\n[runners]\n",
        )
        .unwrap();
        let dir = specs_dir_for(repo.path()).unwrap();
        assert_eq!(dir, repo.path().join("custom/specs"));
    }

    // ------------------------------------------------------------------
    // RED→GREEN tests: one per check kind. Each asserts the kind fires on
    // the defective fixture and stays silent on the clean fixture. They
    // were `#[ignore]`d at tracer-bullet scope; each was un-ignored in the
    // ticket that implemented its check (rty: scenario-flow, aar:
    // spec-shape).
    // ------------------------------------------------------------------

    #[test]
    fn vague_qualifier_fires_on_defective_fixture() {
        let output = run_lint_query(&defective_specs(), None).unwrap();
        assert!(output.findings.iter().any(|f| f.kind == "vague-qualifier"));
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "vague-qualifier" && f.spec_path == "auth"));
    }

    #[test]
    fn imperative_step_fires_on_defective_fixture() {
        let output = run_lint_query(&defective_specs(), None).unwrap();
        assert!(output.findings.iter().any(|f| f.kind == "imperative-step"));
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "imperative-step" && f.spec_path == "auth"));
    }

    #[test]
    fn conjunctive_bloat_fires_on_defective_fixture() {
        let output = run_lint_query(&defective_specs(), None).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "conjunctive-bloat"));
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "conjunctive-bloat" && f.spec_path == "auth"));
    }

    #[test]
    fn missing_negative_scenario_fires_on_defective_fixture() {
        let output = run_lint_query(&defective_specs(), None).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "missing-negative-scenario"));
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "missing-negative-scenario" && f.spec_path == "auth"));
    }

    #[test]
    fn missing_non_goals_fires_on_defective_fixture() {
        let output = run_lint_query(&defective_specs(), None).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "missing-non-goals"));
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "missing-non-goals" && f.spec_path == "auth"));
    }

    #[test]
    fn unresolved_ambiguity_fires_on_defective_fixture() {
        let output = run_lint_query(&defective_specs(), None).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "unresolved-ambiguity"));
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "unresolved-ambiguity" && f.spec_path == "auth"));
    }

    #[test]
    fn entangled_spec_fires_on_defective_fixture() {
        let output = run_lint_query(&defective_specs(), None).unwrap();
        assert!(output.findings.iter().any(|f| f.kind == "entangled-spec"));
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "entangled-spec" && f.spec_path == "auth"));
    }
}
