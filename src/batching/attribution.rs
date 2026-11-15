//! Attribution: each binding's verdict from structured per-test output.
//!
//! Purpose: turn one batched invocation's vitest JSON report into per-binding
//! verdicts that are exactly as strong as per-binding execution.
//!
//! Responsibilities: parse the structured report (stdout, or the file named
//! by a reporter notice), re-match each binding's pattern against every
//! assertion, and classify it passed / test-failing / no-tests-ran.
//!
//! Rationale: the batched exit code alone cannot distinguish a binding whose
//! tests never ran from one that passed (C-batch-attribution,
//! C-batch-matched-zero), so attribution — not the exit code — decides.

use super::{BatchableBinding, MAX_PARSE_BYTES};
use crate::runner::TestResult;
use regex::Regex;
use serde::Deserialize;
use std::path::Path;

/// Per-binding verdict derived from a batched invocation's structured output.
#[derive(Debug, Clone)]
pub(crate) enum BatchedVerdict {
    /// Every test matched by the binding's pattern reported passed.
    Passed,
    /// At least one matched test reported failed.
    TestFailing(TestResult),
    /// The pattern matched nothing, or matched only skipped/pending/todo
    /// tests — falsification did not run, never covered by the exit code.
    NoTestsRan(TestResult),
    /// Attribution was impossible; the caller re-runs this binding
    /// per-contract with exit-code verdicts. The reason names the
    /// degenerate outcome (batch-runner-spawns task 2.2: named, non-silent
    /// fallback signals).
    Fallback(String),
}

/// Attribute each binding from the structured output (C-batch-attribution,
/// C-batch-matched-zero): a binding passes only when every test matched by
/// its pattern reports passed; any matched failed test → test-failing;
/// matched nothing or matched only skipped/pending/todo → no-tests-ran.
/// Returns None when the output is unparseable (caller falls back).
pub(super) fn attribute(
    bindings: &[BatchableBinding],
    result: &TestResult,
    stdout: &str,
    repo_root: &Path,
) -> Option<Vec<BatchedVerdict>> {
    let report = parse_report(stdout, repo_root)?;
    let assertions: Vec<&VitestAssertion> = report
        .test_results
        .iter()
        .flat_map(|suite| suite.assertion_results.iter())
        .collect();
    Some(
        bindings
            .iter()
            .map(|binding| verdict_for(binding, &assertions, result))
            .collect(),
    )
}

/// Parse stdout as the report itself, or — when stdout is only the reporter's
/// notice — as the file that notice names (GH#46).
fn parse_report(stdout: &str, repo_root: &Path) -> Option<VitestReport> {
    match serde_json::from_str(stdout) {
        Ok(report) => Some(report),
        Err(_) => report_from_notice(stdout, repo_root),
    }
}

/// Classify one binding against the parsed assertions.
fn verdict_for(
    binding: &BatchableBinding,
    assertions: &[&VitestAssertion],
    result: &TestResult,
) -> BatchedVerdict {
    let re = match Regex::new(&format!("(?:{})", binding.pattern)) {
        // The batched invocation already matched with the JS engine; if the
        // Rust engine cannot compile the same pattern the attribution view is
        // incomplete → per-binding fallback.
        Err(_) => {
            return BatchedVerdict::Fallback(
                "pattern not compilable by the attribution engine".to_string(),
            )
        }
        Ok(re) => re,
    };
    let mut any_matched = false;
    let mut any_failed = false;
    let mut all_passed = true;
    for assertion in assertions {
        if !re.is_match(&assertion.full_name) {
            continue;
        }
        any_matched = true;
        match assertion.status.as_str() {
            "passed" => {}
            "failed" => any_failed = true,
            // skipped/pending/todo: the test did not run.
            _ => all_passed = false,
        }
    }
    if any_failed {
        BatchedVerdict::TestFailing(result.clone())
    } else if any_matched && all_passed {
        BatchedVerdict::Passed
    } else {
        BatchedVerdict::NoTestsRan(result.clone())
    }
}

/// Vitest 5 routes the json reporter to a file instead of stdout and prints a
/// `JSON report written to <path>` notice line (GH#46). When stdout is not the
/// report itself, scan it for the notice and parse the file it names. Relative
/// paths resolve against the repo root (the invocation cwd). Oversized files
/// fall under the same parse cap as stdout. No file or unparseable content →
/// None (caller falls back).
fn report_from_notice(stdout: &str, repo_root: &Path) -> Option<VitestReport> {
    const NOTICE: &str = "JSON report written to";
    let path_text = stdout
        .lines()
        .find_map(|line| line.split_once(NOTICE).map(|(_, tail)| tail))?
        .trim()
        .trim_matches(|c| c == '\'' || c == '"' || c == '`')
        .to_string();
    if path_text.is_empty() {
        return None;
    }
    let path = if Path::new(&path_text).is_absolute() {
        std::path::PathBuf::from(&path_text)
    } else {
        repo_root.join(&path_text)
    };
    let bytes = std::fs::read(&path).ok()?;
    if bytes.len() > MAX_PARSE_BYTES {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}

/// Vitest JSON-reporter shape (jest-compatible). Only the fields attribution
/// needs are declared; unknown fields are ignored.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VitestReport {
    #[serde(default)]
    test_results: Vec<VitestSuite>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VitestSuite {
    #[serde(default)]
    assertion_results: Vec<VitestAssertion>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VitestAssertion {
    #[serde(default)]
    full_name: String,
    #[serde(default)]
    status: String,
}

#[cfg(test)]
mod tests {
    use super::{attribute, BatchedVerdict};
    use crate::batching::test_support::{batch_result, entry_with_pattern, report_json};
    use std::path::Path;

    // C-batch-attribution: every matched test passed → passed.
    #[test]
    fn attribution_passes_when_all_matched_tests_passed() {
        let bindings = vec![entry_with_pattern("p_a"), entry_with_pattern("p_b")];
        let stdout = report_json(&[("p_a", "passed"), ("p_b", "passed")]);

        let verdicts =
            attribute(&bindings, &batch_result(Some(0)), &stdout, Path::new(".")).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::Passed));
        assert!(matches!(verdicts[1], BatchedVerdict::Passed));
    }

    // C-batch-attribution: any matched test failed → test-failing.
    #[test]
    fn attribution_fails_contract_when_one_matched_test_failed() {
        let bindings = vec![entry_with_pattern("p_a")];
        let stdout = report_json(&[("p_a one", "passed"), ("p_a two", "failed")]);

        let verdicts =
            attribute(&bindings, &batch_result(Some(1)), &stdout, Path::new(".")).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::TestFailing(_)));
    }

    // C-batch-matched-zero: pattern matching nothing → no-tests-ran even
    // though the batched invocation exited zero.
    #[test]
    fn attribution_emits_no_tests_ran_for_matched_zero() {
        let bindings = vec![entry_with_pattern("p_missing")];
        let stdout = report_json(&[("p_a", "passed")]);

        let verdicts =
            attribute(&bindings, &batch_result(Some(0)), &stdout, Path::new(".")).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::NoTestsRan(_)));
    }

    // C-batch-attribution: skipped/pending/todo matched → no-tests-ran
    // (falsification did not run; a skipped test cannot cover a contract).
    #[test]
    fn attribution_emits_no_tests_ran_for_skipped_match() {
        let bindings = vec![entry_with_pattern("p_a")];
        let stdout = report_json(&[("p_a", "skipped")]);

        let verdicts =
            attribute(&bindings, &batch_result(Some(0)), &stdout, Path::new(".")).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::NoTestsRan(_)));
    }

    // failed dominates skipped when both match the same pattern.
    #[test]
    fn attribution_failure_dominates_skipped() {
        let bindings = vec![entry_with_pattern("p_a")];
        let stdout = report_json(&[("p_a one", "skipped"), ("p_a two", "failed")]);

        let verdicts =
            attribute(&bindings, &batch_result(Some(1)), &stdout, Path::new(".")).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::TestFailing(_)));
    }

    // Unparseable structured output → whole batch falls back.
    #[test]
    fn unparseable_output_falls_back_per_binding() {
        let bindings = vec![entry_with_pattern("p_a")];
        let verdicts = attribute(
            &bindings,
            &batch_result(Some(0)),
            "not json",
            Path::new("."),
        );
        assert!(verdicts.is_none());
    }

    // GH#46: vitest 5 routes the json reporter to a file and prints a
    // `JSON report written to <path>` notice on stdout. Attribution must
    // follow the notice and parse the file it names.
    #[test]
    fn notice_line_directs_attribution_to_report_file() {
        let dir = tempfile::tempdir().unwrap();
        let report_path = dir.path().join("output.json");
        std::fs::write(&report_path, report_json(&[("p_a", "passed")])).unwrap();
        let stdout = format!(
            "some runner noise\nJSON report written to {}\nmore noise\n",
            report_path.display()
        );
        let bindings = vec![entry_with_pattern("p_a")];

        let verdicts = attribute(&bindings, &batch_result(Some(0)), &stdout, dir.path()).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::Passed));
    }

    // GH#46: the notice's path can be repo-relative (config `outputFile:
    // "target/report.json"`) — resolve it against the invocation cwd.
    #[test]
    fn notice_line_relative_path_resolves_against_repo_root() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("target");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            nested.join("report.json"),
            report_json(&[("p_a", "passed")]),
        )
        .unwrap();
        let stdout = "JSON report written to target/report.json";
        let bindings = vec![entry_with_pattern("p_a")];

        let verdicts = attribute(&bindings, &batch_result(Some(0)), stdout, dir.path()).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::Passed));
    }

    // GH#46: notice names a file whose content is not the report → fallback
    // (no silent mis-attribution from a stale or foreign file).
    #[test]
    fn notice_file_with_unparseable_content_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let report_path = dir.path().join("output.json");
        std::fs::write(&report_path, "not the report").unwrap();
        let stdout = format!("JSON report written to {}", report_path.display());
        let bindings = vec![entry_with_pattern("p_a")];

        let verdicts = attribute(&bindings, &batch_result(Some(0)), &stdout, dir.path());

        assert!(verdicts.is_none());
    }

    // GH#46: unparseable stdout with NO notice line → still falls back.
    #[test]
    fn unparseable_stdout_without_notice_still_falls_back() {
        let bindings = vec![entry_with_pattern("p_a")];
        let verdicts = attribute(
            &bindings,
            &batch_result(Some(0)),
            "build failed\nexit 2",
            Path::new("."),
        );
        assert!(verdicts.is_none());
    }

    // JS-only pattern that Rust regex cannot compile → per-binding fallback.
    #[test]
    fn uncompilable_pattern_falls_back_for_that_binding() {
        let bindings = vec![entry_with_pattern("p_a"), entry_with_pattern("p_(?=b)")];
        let stdout = report_json(&[("p_b", "passed")]);

        let verdicts =
            attribute(&bindings, &batch_result(Some(0)), &stdout, Path::new(".")).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::NoTestsRan(_)));
        assert!(matches!(
            &verdicts[1],
            BatchedVerdict::Fallback(reason) if reason.contains("attribution engine")
        ));
    }

    // batch-runner-spawns task 2.3 (C-batch-attribution): attribution parses
    // the FULL captured stdout — a passing assertion beyond the 8 KiB
    // findings tail still reaches the parser; the TestResult tail that lands
    // in findings stays 8 KiB findings-only.
    #[test]
    fn attribution_parses_stdout_beyond_the_findings_tail() {
        let bindings = vec![entry_with_pattern("p_a")];
        // 16 KiB of padding before the assertion — far beyond OUTPUT_TAIL_BYTES.
        let padding = "x".repeat(16 * 1024);
        let stdout = format!(
            r#"{{"testResults": [{{"assertionResults": [{{"fullName": "{padding}", "status": "passed"}}, {{"fullName": "p_a", "status": "passed"}}]}}]}}"#
        );
        assert!(stdout.len() > 8 * 1024);

        let verdicts =
            attribute(&bindings, &batch_result(Some(0)), &stdout, Path::new(".")).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::Passed));
    }
}
