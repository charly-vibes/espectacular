//! `ah check` command handler: selection input resolution (files / commit
//! range / worktree diff), execution via `check::run_check_with_selection`,
//! and the human-readable report printer.

use crate::check;
use crate::cli::{to_json_envelope_with_ok, EnvelopeKind};
use std::io::Write;
use std::path::Path;

/// Handle `ah check`.
pub fn run_check_command(
    changes: Vec<String>,
    run_tests: bool,
    files: Vec<String>,
    all_tests: bool,
    base: Option<String>,
    head: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let repo_root = std::env::current_dir()?;
    // Selection input (espectacular-0k8): explicit --files wins;
    // then the --base commit range (C-range-diff); otherwise the
    // worktree git diff supplies the changed-file set. Degenerate
    // ranges fall back to the empty set, which runs everything.
    let changed_files = if !files.is_empty() || !run_tests || all_tests {
        files.clone()
    } else if let Some(base) = base.as_deref() {
        let head = head.as_deref().unwrap_or("HEAD");
        changed_files_from_range(&repo_root, base, head)
    } else {
        changed_files_from_git(&repo_root)
    };
    let report = check::run_check_with_selection(
        &repo_root,
        &changes,
        run_tests,
        &changed_files,
        all_tests,
    )?;
    if json {
        let has_blocking = has_blocking_findings(&report);
        // ok must agree with the exit code: blocking findings fail
        // the gate (exit 1), so the envelope reports ok:false —
        // see to_json_envelope_with_ok (evallerina-54f).
        println!(
            "{}",
            to_json_envelope_with_ok(EnvelopeKind::Check, &report, !has_blocking)
        );
        std::io::stdout().flush().unwrap_or_default();
        std::process::exit(if has_blocking { 1 } else { 0 });
    } else {
        print_check_report(&report, run_tests);
    }
    let has_blocking = has_blocking_findings(&report);
    std::io::stdout().flush().unwrap_or_default();
    std::process::exit(if has_blocking { 1 } else { 0 });
}

/// Structural and execution findings fail the gate (exit 1).
fn has_blocking_findings(report: &check::CheckOutput) -> bool {
    report
        .findings
        .iter()
        .any(|f| f.category == "structural" || f.category == "execution")
}

/// Changed files across the commit range <base>..<head> (two-dot snapshot
/// diff, testaruda-jdw5 / C-range-diff), repo-relative. Degenerate ranges —
/// an unresolvable ref or an empty diff — yield an empty set, which the
/// Layer-1 fallback runs as a full run (C-range-safe / design D2): the gate
/// must never pass with zero executed tests because of a range it cannot
/// distinguish from "nothing changed".
fn changed_files_from_range(repo_root: &Path, base: &str, head: &str) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", base, head])
        .current_dir(repo_root)
        .output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Worktree changed files (unstaged + staged vs HEAD), repo-relative.
fn changed_files_from_git(repo_root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(repo_root)
        .output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn print_check_report(report: &check::CheckOutput, run_tests: bool) {
    let total = report.findings.len() + report.quality_findings.len();

    if total == 0 {
        print_clean_verdict(report.summary.passed, run_tests);
        return;
    }

    println!("found {total} issue(s):");
    print_findings(&report.findings);
    print_quality_findings(&report.quality_findings);
    print_summary(report);
}

fn print_clean_verdict(passed: usize, run_tests: bool) {
    if passed == 0 && !run_tests {
        println!("no issues — 0 structural, 0 execution (contract tests skipped; run `ah check --run-tests` to execute)");
    } else if passed == 0 {
        println!("no scenarios to check — 0 passed, 0 findings");
    } else {
        println!("OK — {passed} passed, 0 findings");
    }
}

fn print_findings(findings: &[check::ReportFinding]) {
    for finding in findings {
        let spec = &finding.spec;
        let scenario_id = &finding.scenario.id;
        let kind = &finding.kind;
        let category = &finding.category;
        let msg = finding.message.as_deref().unwrap_or("");
        if msg.is_empty() {
            println!("{category}: {spec}/{scenario_id} — {kind}");
        } else {
            println!("{category}: {spec}/{scenario_id} — {kind}: {msg}");
        }
        if let Some(test) = &finding.test {
            print_tails(test);
        }
    }
}

fn print_tails(test: &crate::runner::TestResult) {
    for (label, tail) in [("stdout", &test.stdout_tail), ("stderr", &test.stderr_tail)] {
        let lines: Vec<&str> = tail.lines().collect();
        for line in lines.iter().take(5) {
            println!("  {label}: {line}");
        }
        if lines.len() > 5 {
            println!("  {label}: ... ({} more lines)", lines.len() - 5);
        }
    }
}

fn print_quality_findings(quality_findings: &[crate::quality::QualityFinding]) {
    if quality_findings.is_empty() {
        return;
    }
    println!();
    for qf in quality_findings {
        println!("quality: {} — {}", qf.kind, qf.message);
        if let Some(kr) = qf.kill_rate {
            println!("  kill_rate: {:.1}%", kr * 100.0);
        }
        if let Some(th) = qf.threshold {
            println!("  threshold: {:.1}%", th * 100.0);
        }
    }
}

fn print_summary(report: &check::CheckOutput) {
    let summary = &report.summary;
    println!();
    println!(
        "summary: {} passed, {} structural, {} execution, {} quality",
        summary.passed,
        summary.structural,
        summary.execution,
        report.quality_findings.len()
    );
    for (kind, count) in &summary.counts_by_kind {
        println!("  {kind}: {count}");
    }
    if let Some(sel) = &report.selection {
        let testaruda = sel
            .testaruda
            .as_ref()
            .map(|t| format!(", {} pruned by testaruda", t.pruned))
            .unwrap_or_default();
        println!(
            "selection: {} — {} selected, {} skipped{}",
            sel.source,
            sel.selected.len(),
            sel.skipped,
            testaruda
        );
    }
}
