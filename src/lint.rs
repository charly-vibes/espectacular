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
    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>);
}

/// The check registry. Empty until espectacular-rty/aar land their checks —
/// append additively (one entry per check), do not reorder.
pub(crate) fn checks() -> Vec<Box<dyn LintCheck>> {
    Vec::new()
}

/// Run the registered checks over the specs under `specs_dir`.
pub fn run_lint(specs_dir: &Path) -> anyhow::Result<LintOutput> {
    run_lint_with(specs_dir, &checks())
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

/// Human-readable lint report (stderr stays clean for findings on stdout).
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
        let output = run_lint(&clean_specs()).unwrap();
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
    // RED tests: one per check kind. Each asserts the kind fires on the
    // defective fixture and stays silent on the clean fixture. They stay
    // `#[ignore]`d here because the checks themselves land in
    // espectacular-rty (scenario-flow) and espectacular-aar (spec-shape);
    // un-ignore the test in the ticket that implements the check.
    // ------------------------------------------------------------------

    #[test]
    #[ignore = "RED: vague-qualifier check lands in espectacular-rty (task 3.1)"]
    fn vague_qualifier_fires_on_defective_fixture() {
        let output = run_lint(&defective_specs()).unwrap();
        assert!(output.findings.iter().any(|f| f.kind == "vague-qualifier"));
        assert!(!output
            .findings
            .iter()
            .any(|f| f.kind == "vague-qualifier" && f.spec_path == "auth"));
    }

    #[test]
    #[ignore = "RED: imperative-step check lands in espectacular-rty (task 3.2)"]
    fn imperative_step_fires_on_defective_fixture() {
        let output = run_lint(&defective_specs()).unwrap();
        assert!(output.findings.iter().any(|f| f.kind == "imperative-step"));
    }

    #[test]
    #[ignore = "RED: conjunctive-bloat check lands in espectacular-rty (task 3.3)"]
    fn conjunctive_bloat_fires_on_defective_fixture() {
        let output = run_lint(&defective_specs()).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "conjunctive-bloat"));
    }

    #[test]
    #[ignore = "RED: missing-negative-scenario check lands in espectacular-rty (task 3.4)"]
    fn missing_negative_scenario_fires_on_defective_fixture() {
        let output = run_lint(&defective_specs()).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "missing-negative-scenario"));
    }

    #[test]
    #[ignore = "RED: missing-non-goals check lands in espectacular-aar (task 3.5)"]
    fn missing_non_goals_fires_on_defective_fixture() {
        let output = run_lint(&defective_specs()).unwrap();
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
    #[ignore = "RED: unresolved-ambiguity check lands in espectacular-aar (task 3.6)"]
    fn unresolved_ambiguity_fires_on_defective_fixture() {
        let output = run_lint(&defective_specs()).unwrap();
        assert!(output
            .findings
            .iter()
            .any(|f| f.kind == "unresolved-ambiguity"));
    }

    #[test]
    #[ignore = "RED: entangled-spec check lands in espectacular-aar (task 3.7)"]
    fn entangled_spec_fires_on_defective_fixture() {
        let output = run_lint(&defective_specs()).unwrap();
        assert!(output.findings.iter().any(|f| f.kind == "entangled-spec"));
    }
}
