//! Per-spec relay of `spk lint` findings into the shared finding schema.
//!
//! Plain openspec files are never touched; a `kind: intent` file without a
//! frontmatter id is flagged instead of silently skipped.

use super::graph::relay_graph;
use super::invoke::{invoke, Failure, SpkIssue};
use super::is_dual_format;
use crate::lint::walker::SpecFile;
use crate::lint::LintFinding;

const SPK_PROGRAM: &str = "spk";

/// Relay spk findings for every dual-format spec. Plain openspec files never
/// trigger an invocation.
pub fn relay_dual_format(specs: &[SpecFile], findings: &mut Vec<LintFinding>) {
    relay_with(SPK_PROGRAM, specs, findings);
}

/// Test seam: same relay against an explicit spk program path.
pub(crate) fn relay_with(program: &str, specs: &[SpecFile], findings: &mut Vec<LintFinding>) {
    for spec in specs {
        if !is_dual_format(&spec.raw) {
            relay_intent_mismatch(spec, findings);
            continue;
        }
        let Some(path) = spec.source_path.to_str() else {
            continue;
        };
        relay_one(program, spec, path, findings);
    }
}

/// Task 5.1: a kind-intent file the bridge would otherwise skip with a bare
/// `continue` is a silent miss — flag the frontmatter gap instead.
fn relay_intent_mismatch(spec: &SpecFile, findings: &mut Vec<LintFinding>) {
    let Some(msg) = intent_mismatch(spec) else {
        return;
    };
    findings.push(LintFinding::warning(
        "spk-frontmatter-mismatch",
        &spec.spec_path,
        "",
        &msg,
        "set the frontmatter id to the expected value (filename stem with '-' mapped to '.') or drop the kind: intent frontmatter if the file is not meant to be specodelic",
        "edit_spec",
        "ah explain spk-frontmatter-mismatch",
    ));
}

/// Invoke spk for one dual-format spec, mapping every failure mode to an
/// advisory finding.
fn relay_one(program: &str, spec: &SpecFile, path: &str, findings: &mut Vec<LintFinding>) {
    match invoke(program, path) {
        Ok(issues) => {
            relay_graph(program, spec, path, findings);
            push_issues(spec, issues, findings);
        }
        Err(Failure::Unavailable(msg)) => {
            findings.push(failure_finding(spec, "spk-unavailable", &msg))
        }
        Err(Failure::Broken(msg)) => {
            findings.push(failure_finding(spec, "spk-bridge-failure", &msg))
        }
    }
}

/// Advisory finding for a degraded spk invocation; the remediation pair is
/// keyed off the degradation kind.
fn failure_finding(spec: &SpecFile, kind: &str, msg: &str) -> LintFinding {
    let (suggestion, action) = match kind {
        "spk-unavailable" => (
            "install specodelic to lint dual-format specs",
            "install_tool",
        ),
        _ => (
            "inspect the spk invocation failure; relayed findings are skipped",
            "edit_spec",
        ),
    };
    LintFinding::warning(
        kind,
        &spec.spec_path,
        "",
        msg,
        suggestion,
        action,
        "ah explain lint",
    )
}

/// One warning per relayed spk issue: `kind = "spk.<rule_id>"`.
fn push_issues(spec: &SpecFile, issues: Vec<SpkIssue>, findings: &mut Vec<LintFinding>) {
    for issue in issues {
        findings.push(LintFinding::warning(
            &format!("spk.{}", issue.rule_id),
            &spec.spec_path,
            "",
            &issue.message,
            &suggestion_for(&issue),
            "edit_spec",
            "ah explain lint",
        ));
    }
}

/// spk's own rule semantics when present, else a generic remediation.
fn suggestion_for(issue: &SpkIssue) -> String {
    if issue.rule_semantics.is_empty() {
        "address the specodelic invariant".to_string()
    } else {
        issue.rule_semantics.clone()
    }
}

/// Task 5.1: a non-dual-format file whose frontmatter declares `kind: intent`
/// but carries NO `id` line — the bridge would skip it silently, so flag it.
/// Files WITH an id line are dual-format (routed to spk); a wrong id VALUE is
/// spk's `linter.id_matches_file` to report, never this check's.
fn intent_mismatch(spec: &SpecFile) -> Option<String> {
    let rest = spec.raw.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let front = &rest[..end];
    let mut is_intent = false;
    let mut has_id = false;
    for line in front.lines() {
        let trimmed = line.trim();
        if trimmed == "kind: intent" {
            is_intent = true;
        }
        if trimmed.starts_with("id:") {
            has_id = true;
        }
    }
    if is_intent && !has_id {
        Some(
            "file declares kind: intent but has no frontmatter id — add one; \
             specodelic (Revision 18) derives the expected id from the parent \
             directory and `spk lint` enforces it"
                .to_string(),
        )
    } else {
        None
    }
}
