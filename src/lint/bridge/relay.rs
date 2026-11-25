//! Per-spec relay of `spk lint` findings into the shared finding schema.
//!
//! Plain openspec files are never touched; a `kind: intent` file without a
//! frontmatter id is flagged instead of silently skipped.

use super::graph::relay_graph;
use super::invoke::{invoke, invoke_dir, issue_matches_path, Failure, SpkIssue};
use super::is_dual_format;
use crate::lint::walker::SpecFile;
use crate::lint::LintFinding;
use std::path::Path;

const SPK_PROGRAM: &str = "spk";

/// Relay spk findings for every dual-format spec under `root`: ONE
/// `spk lint <root>` spawn per linted root (espectacular-hcz) with issues
/// partitioned by the per-issue `file` field. Specs outside `root` (test
/// seams with virtual paths) keep per-file invocations. Plain openspec files
/// never trigger an invocation.
pub fn relay_dual_format(root: &Path, specs: &[SpecFile], findings: &mut Vec<LintFinding>) {
    relay_with_root(SPK_PROGRAM, root, specs, findings);
}

/// Dir-mode orchestrator: one `spk lint <root>` spawn per linted root, with
/// named fallbacks to the per-file path.
///
/// - `Unavailable` (spk missing): one `spk-unavailable` advisory per dual
///   spec — identical to per-file mode.
/// - `Broken` (dir envelope failed as a whole, e.g. one unparseable spec):
///   per-file re-run preserves per-file isolation — broken files get their
///   advisory, good files their findings (C-dual-bridge: linting continues).
/// - Ok but any issue lacks the `file` field (older spk pins): named
///   `spk-bridge-fallback` advisory, then per-file re-run. No silent loss.
/// - Ok with attribution: graph relay + issues per spec; issues for files
///   outside the walked set still surface, attributed by their file path.
///   Skipped-file envelope warnings never become findings.
pub(crate) fn relay_with_root(
    program: &str,
    root: &Path,
    specs: &[SpecFile],
    findings: &mut Vec<LintFinding>,
) {
    let in_root = partition_dual(program, root, specs, findings);
    if in_root.is_empty() {
        return;
    }
    match invoke_dir(program, root) {
        Err(Failure::Unavailable(msg)) => unavailable_findings(&in_root, &msg, findings),
        Err(Failure::Broken(_)) => per_file_relay(program, &in_root, findings),
        Ok(issues) => relay_attributed(
            &DirEnvelope {
                program,
                root,
                issues,
            },
            &in_root,
            findings,
        ),
    }
}

/// An attributed dir envelope plus its invocation identity, bundled to keep
/// `relay_attributed` under the param budget.
struct DirEnvelope<'a> {
    program: &'a str,
    root: &'a Path,
    issues: Vec<SpkIssue>,
}

/// Route each walked spec: plain openspec files get the frontmatter-mismatch
/// check, dual-format specs under `root` join the dir-mode batch, and specs
/// outside `root` (test seams with virtual paths) keep per-file invocations.
fn partition_dual<'a>(
    program: &str,
    root: &Path,
    specs: &'a [SpecFile],
    findings: &mut Vec<LintFinding>,
) -> Vec<(&'a SpecFile, String)> {
    let mut in_root = Vec::new();
    for spec in specs {
        if !is_dual_format(&spec.raw) {
            relay_intent_mismatch(spec, findings);
        } else if let Some(path) = spec.source_path.to_str() {
            if spec.source_path.starts_with(root) {
                in_root.push((spec, path.to_string()));
            } else {
                relay_one(program, spec, path, findings);
            }
        }
    }
    in_root
}

/// `spk` missing: one advisory per dual spec, identical to per-file mode.
fn unavailable_findings(
    in_root: &[(&SpecFile, String)],
    msg: &str,
    findings: &mut Vec<LintFinding>,
) {
    for (spec, _) in in_root {
        findings.push(failure_finding(spec, "spk-unavailable", msg));
    }
}

/// The authoritative per-file path (named fallback target): broken files
/// get their advisory, good files their findings — linting continues
/// (C-dual-bridge).
fn per_file_relay(program: &str, in_root: &[(&SpecFile, String)], findings: &mut Vec<LintFinding>) {
    for (spec, path) in in_root {
        relay_one(program, spec, path, findings);
    }
}

/// Relay an attributed dir envelope: `file`-field issues are partitioned per
/// spec (graph relay interleaved, per-file order); issues for files outside
/// the walked set still surface, attributed by their file path. Skipped-file
/// envelope warnings never become findings.
fn relay_attributed(
    env: &DirEnvelope,
    in_root: &[(&SpecFile, String)],
    findings: &mut Vec<LintFinding>,
) {
    if env.issues.iter().any(|i| i.file.is_none()) {
        findings.push(fallback_finding(env.root));
        per_file_relay(env.program, in_root, findings);
        return;
    }
    for (spec, _) in in_root {
        relay_graph(
            env.program,
            spec,
            &spec.source_path.to_string_lossy(),
            findings,
        );
        let own: Vec<SpkIssue> = env
            .issues
            .iter()
            .filter(|i| issue_matches_path(i, &spec.source_path))
            .cloned()
            .collect();
        push_issues(spec, own, findings);
    }
    relay_unmatched(env, in_root, findings);
}

/// Attributed issues for files the walker did not surface (e.g. a dual-
/// format file the walker errored on): surfaced by path, never dropped.
fn relay_unmatched(
    env: &DirEnvelope,
    in_root: &[(&SpecFile, String)],
    findings: &mut Vec<LintFinding>,
) {
    for issue in env.issues.iter().filter(|i| {
        !in_root
            .iter()
            .any(|(spec, _)| issue_matches_path(i, &spec.source_path))
    }) {
        findings.push(unmatched_issue_finding(issue));
    }
}

/// Named degradation for envelopes without per-issue attribution
/// (espectacular-hcz; C-batch-fallback precedent): the advisory names the
/// degradation, the per-file re-run carries the authoritative findings.
fn fallback_finding(root: &Path) -> LintFinding {
    LintFinding::warning(
        "spk-bridge-fallback",
        &root.to_string_lossy(),
        "",
        "dir-mode spk lint envelope lacked per-issue file attribution; findings re-relayed per file",
        "upgrade specodelic to a version that emits per-issue `file` fields",
        "upgrade_tool",
        "ah explain lint",
    )
}

/// An attributed issue for a file the walker did not surface (e.g. a dual-
/// format file the walker errored on): surfaced by path, never dropped.
fn unmatched_issue_finding(issue: &SpkIssue) -> LintFinding {
    LintFinding::warning(
        &format!("spk.{}", issue.rule_id),
        issue.file.as_deref().unwrap_or("<unknown>"),
        "",
        &issue.message,
        &suggestion_for(issue),
        "edit_spec",
        "ah explain lint",
    )
}

/// Test seam: per-file relay against an explicit spk program path.
#[cfg(test)]
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
