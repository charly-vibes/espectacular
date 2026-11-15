//! Human-readable lint report for `ah lint`.

use super::{LintOutput, Severity};

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
    println!(
        "lint: {} findings ({} warnings, {} errors)",
        output.findings.len(),
        count_severity(output, Severity::Warning),
        count_severity(output, Severity::Error)
    );
}

fn count_severity(output: &LintOutput, severity: Severity) -> usize {
    output
        .findings
        .iter()
        .filter(|f| f.severity == severity)
        .count()
}
