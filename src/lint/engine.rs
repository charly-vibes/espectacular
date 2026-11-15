//! Lint engine: registry resolution, the walk→check→relay pipeline, and
//! output merge/exit-code semantics.

use super::walker;
use super::{bridge, LintCheck, LintFinding, LintOutput};
use std::collections::BTreeMap;
use std::path::Path;

/// Run lint with an optional single-kind filter (`--check <kind>`).
pub fn run_lint_query(specs_dir: &Path, check: Option<&str>) -> anyhow::Result<LintOutput> {
    let cfg = super::lint_config_for(specs_dir)?;
    let registered = super::select_checks(&cfg, check)?;
    run_lint_with(specs_dir, &registered)
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
    let overlay_specs = super::changes_dir_for(repo_root)?
        .join(change)
        .join("specs");
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
        .any(|f| f.severity == super::Severity::Error)
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
    let (specs, read_errors) = walker::walk_specs_with_errors(specs_dir)?;
    let mut findings = Vec::new();
    for spec in &specs {
        for check in registered {
            check.check(spec, &mut findings);
        }
    }
    // Malformed spec files surface as error-severity findings (deployed lint
    // spec C-finding-schema) instead of aborting the walk — the rest of the
    // corpus still lints and exit_code() maps Error to non-zero.
    findings.extend(read_errors.iter().map(malformed_spec_finding));
    // Dual-format bridge: relay spk findings after the prose checks. Runs
    // regardless of --check filtering — the bridge is not a registry check.
    bridge::relay_dual_format(&specs, &mut findings);
    Ok(LintOutput {
        counts_by_kind: count_by_kind(&findings),
        findings,
    })
}

/// Error-severity finding for one unreadable spec file.
fn malformed_spec_finding(err: &walker::SpecReadError) -> LintFinding {
    LintFinding::error(
        "malformed-spec",
        &err.spec_path,
        "",
        &format!("spec file could not be read as UTF-8: {}", err.message),
        "fix or remove the malformed spec file so it parses as valid openspec markdown",
        "review_and_apply",
        "ah explain lint",
    )
}

/// Tally findings per kind.
fn count_by_kind(findings: &[LintFinding]) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for finding in findings {
        *counts.entry(finding.kind.clone()).or_insert(0) += 1;
    }
    counts
}
