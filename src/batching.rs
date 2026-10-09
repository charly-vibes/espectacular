//! Batched runner execution (batch-runner-spawns).
//!
//! Purpose: cut runner startup cost from one process per contract to one
//! process per eligible runner type per run while keeping per-contract
//! falsification semantics exactly as strong as per-binding execution.
//!
//! Responsibilities: binding eligibility (scenario-scoped vitest
//! `--testNamePattern` bindings), batch composition (OR-joined pattern +
//! structured reporter, max-entry timeout), attribution of each binding's
//! verdict from the structured per-test output, and conservative fallback
//! to per-binding execution on any degenerate batched outcome.
//!
//! Rationale: measured corpora (tambor: 658 bindings, 603–835s per push,
//! ~93% of the gate) are dominated by ~1s vitest startup per spawn; the
//! bindings carry no file notion, only test-name patterns (design Q1–Q3).

use crate::config::Config;
use crate::contracts::TestEntry;
use crate::runner::{execute_command_full, PlannedCommand, TestResult, DEFAULT_TIMEOUT_SECONDS};
use regex::Regex;
use serde::Deserialize;
use std::path::Path;

/// Bindings at or below this count keep per-binding spawns; above it, one
/// runner-type batch replaces them. Design: ~1s vitest startup per spawn
/// makes 8 the amortization crossover on measured corpora.
pub(crate) const BATCH_THRESHOLD: usize = 8;

/// Sanity cap on the parse buffer for structured output (design: batched
/// JSON can be megabytes; a runaway runner must degrade to fallback, not
/// balloon memory).
const MAX_PARSE_BYTES: usize = 64 * 1024 * 1024;

/// A binding eligible for batching: a vitest binding whose flags are exactly
/// a scenario-scoped test-name pattern.
#[derive(Debug, Clone)]
pub(crate) struct BatchableBinding {
    pub pattern: String,
    pub entry: TestEntry,
}

/// A vitest binding is batchable when its flags are exactly
/// `--testNamePattern=<regex>` and the pattern carries no JS-only regex
/// constructs (lookaround, backreferences) — ah re-matches patterns against
/// the structured output with the Rust regex engine, which cannot compile
/// those, so such bindings keep per-binding spawns (batch-runner-spawns
/// task 1.4; design eligibility guard).
pub(crate) fn batchable_pattern(entry: &TestEntry) -> Option<String> {
    let flags = entry.flags.as_deref()?;
    let pattern = flags.strip_prefix("--testNamePattern=")?;
    if pattern.is_empty() || has_js_only_constructs(pattern) {
        None
    } else {
        Some(pattern.to_string())
    }
}

/// Detect regex constructs the Rust regex engine cannot compile: lookaround
/// (`(?=` `(?!` `(?<=` `(?<!`) and backreferences (`\1`–`\9`, `\k<name>`).
/// Escape-aware: `\(?=` is a literal paren, not a lookahead; `\n` is a
/// newline escape, not a backreference; `(?<name>` is a named group, not a
/// lookbehind. Conservative in the safe direction — a false positive costs
/// only a per-binding spawn, a false negative would break attribution.
fn has_js_only_constructs(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if i + 1 < bytes.len() {
                    let next = bytes[i + 1];
                    // Backreference: \1–\9 (not \0, the NUL escape) and
                    // named backreference \k.
                    if (next.is_ascii_digit() && next != b'0') || next == b'k' {
                        return true;
                    }
                }
                i += 2; // skip the escaped character
            }
            b'(' if i + 2 < bytes.len() && bytes[i + 1] == b'?' => {
                let lookaround = match bytes[i + 2] {
                    b'=' | b'!' => true, // (?= / (?! lookaheads
                    b'<' => i + 3 < bytes.len() && (bytes[i + 3] == b'=' || bytes[i + 3] == b'!'), // (?<= / (?<! lookbehinds; (?<name> is a named group
                    _ => false, // (?P<name>, (?:, (?flags — dual-engine
                };
                if lookaround {
                    return true;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    false
}

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

/// Run all bindings as one OR-joined batched invocation and attribute each
/// binding's verdict from the structured per-test output. Any degenerate
/// outcome (composition failure, invocation error, timeout, unparseable or
/// oversized output) degrades every binding to [`BatchedVerdict::Fallback`]
/// so the caller re-runs them per-binding with exit-code verdicts — the
/// batched invocation's exit code never produces a verdict by itself.
pub(crate) fn run_batch(
    repo_root: &Path,
    config: &Config,
    bindings: &[BatchableBinding],
) -> Vec<BatchedVerdict> {
    let fallback_all = |reason: String| vec![BatchedVerdict::Fallback(reason); bindings.len()];

    let planned = match plan(config, bindings) {
        Ok(planned) => planned,
        Err(error) => {
            return fallback_all(format!("batched invocation composition failed: {error}"))
        }
    };
    let (result, stdout) = match execute_command_full(repo_root, &planned) {
        Ok((result, stdout)) => (result, stdout),
        Err(error) => return fallback_all(format!("batched invocation failed: {error}")),
    };
    if result.timed_out {
        return fallback_all(
            "batched invocation timed out; per-binding re-runs carry the verdicts".to_string(),
        );
    }
    if stdout.len() > MAX_PARSE_BYTES {
        return fallback_all(
            "batched structured output exceeds the parse cap; per-binding re-runs carry the verdicts"
                .to_string(),
        );
    }
    match attribute(bindings, &result, &String::from_utf8_lossy(&stdout)) {
        Some(verdicts) => verdicts,
        None => fallback_all(
            "batched structured output unparseable; per-binding re-runs carry the verdicts"
                .to_string(),
        ),
    }
}

/// Compose the single batched invocation: the runner argv plus the structured
/// reporter and an OR-joined `--testNamePattern`, under the maximum
/// timeout_seconds across the batched entries (C-batch-timeout).
fn plan(config: &Config, bindings: &[BatchableBinding]) -> anyhow::Result<PlannedCommand> {
    let runner = config
        .runners
        .get("vitest")
        .ok_or_else(|| anyhow::anyhow!("missing runner for vitest"))?;

    // Each pattern is wrapped in a non-capturing group so anchors and `|`
    // inside one pattern cannot bleed into the join; an unbalanced pattern
    // surfaces as a Rust-regex compile failure in attribution → fallback.
    let joined = bindings
        .iter()
        .map(|b| format!("(?:{})", b.pattern))
        .collect::<Vec<_>>()
        .join("|");

    let mut argv = runner.clone();
    argv.push("--reporter=json".to_string());
    argv.push(format!("--testNamePattern=({joined})"));

    let timeout_seconds = bindings
        .iter()
        .map(|b| b.entry.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS))
        .max()
        .unwrap_or(DEFAULT_TIMEOUT_SECONDS);
    anyhow::ensure!(timeout_seconds > 0, "timeout_seconds must be positive");

    Ok(PlannedCommand {
        test_type: "vitest".to_string(),
        display: argv.join(" "),
        argv,
        timeout_seconds,
    })
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

/// Attribute each binding from the structured output (C-batch-attribution,
/// C-batch-matched-zero): a binding passes only when every test matched by
/// its pattern reports passed; any matched failed test → test-failing;
/// matched nothing or matched only skipped/pending/todo → no-tests-ran.
/// Returns None when the output is unparseable (caller falls back).
fn attribute(
    bindings: &[BatchableBinding],
    result: &TestResult,
    stdout: &str,
) -> Option<Vec<BatchedVerdict>> {
    let report: VitestReport = serde_json::from_str(stdout).ok()?;
    let assertions: Vec<&VitestAssertion> = report
        .test_results
        .iter()
        .flat_map(|suite| suite.assertion_results.iter())
        .collect();

    let mut verdicts = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let verdict = match Regex::new(&format!("(?:{})", binding.pattern)) {
            // The batched invocation already matched with the JS engine;
            // if the Rust engine cannot compile the same pattern the
            // attribution view is incomplete → per-binding fallback.
            Err(_) => BatchedVerdict::Fallback(
                "pattern not compilable by the attribution engine".to_string(),
            ),
            Ok(re) => {
                let mut any_matched = false;
                let mut any_failed = false;
                let mut all_passed = true;
                for assertion in &assertions {
                    if re.is_match(&assertion.full_name) {
                        any_matched = true;
                        match assertion.status.as_str() {
                            "passed" => {}
                            "failed" => any_failed = true,
                            // skipped/pending/todo: the test did not run.
                            _ => all_passed = false,
                        }
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
        };
        verdicts.push(verdict);
    }
    Some(verdicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use std::collections::HashMap;

    fn vitest_config() -> Config {
        Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::from([(
                "vitest".to_string(),
                vec!["/bin/sh".to_string(), "vitest.sh".to_string()],
            )]),
            quality: Default::default(),
            capabilities: Default::default(),
            lint: Default::default(),
        }
    }

    fn entry_with_pattern(pattern: &str) -> BatchableBinding {
        BatchableBinding {
            pattern: pattern.to_string(),
            entry: TestEntry {
                flags: Some(format!("--testNamePattern={pattern}")),
                command: None,
                timeout_seconds: None,
            },
        }
    }

    fn batch_result(exit_code: Option<i32>) -> TestResult {
        TestResult {
            test_type: "vitest".to_string(),
            command: "vitest run --reporter=json".to_string(),
            exit_code,
            timed_out: false,
            stdout_tail: String::new(),
            stderr_tail: String::new(),
        }
    }

    fn report_json(assertions: &[(&str, &str)]) -> String {
        let items = assertions
            .iter()
            .map(|(name, status)| format!(r#"{{"fullName": "{name}", "status": "{status}"}}"#))
            .collect::<Vec<_>>()
            .join(", ");
        format!(r#"{{"testResults": [{{"assertionResults": [{items}]}}]}}"#)
    }

    // C-batch-attribution: every matched test passed → passed.
    #[test]
    fn attribution_passes_when_all_matched_tests_passed() {
        let bindings = vec![entry_with_pattern("p_a"), entry_with_pattern("p_b")];
        let stdout = report_json(&[("p_a", "passed"), ("p_b", "passed")]);

        let verdicts = attribute(&bindings, &batch_result(Some(0)), &stdout).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::Passed));
        assert!(matches!(verdicts[1], BatchedVerdict::Passed));
    }

    // C-batch-attribution: any matched test failed → test-failing.
    #[test]
    fn attribution_fails_contract_when_one_matched_test_failed() {
        let bindings = vec![entry_with_pattern("p_a")];
        let stdout = report_json(&[("p_a one", "passed"), ("p_a two", "failed")]);

        let verdicts = attribute(&bindings, &batch_result(Some(1)), &stdout).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::TestFailing(_)));
    }

    // C-batch-matched-zero: pattern matching nothing → no-tests-ran even
    // though the batched invocation exited zero.
    #[test]
    fn attribution_emits_no_tests_ran_for_matched_zero() {
        let bindings = vec![entry_with_pattern("p_missing")];
        let stdout = report_json(&[("p_a", "passed")]);

        let verdicts = attribute(&bindings, &batch_result(Some(0)), &stdout).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::NoTestsRan(_)));
    }

    // C-batch-attribution: skipped/pending/todo matched → no-tests-ran
    // (falsification did not run; a skipped test cannot cover a contract).
    #[test]
    fn attribution_emits_no_tests_ran_for_skipped_match() {
        let bindings = vec![entry_with_pattern("p_a")];
        let stdout = report_json(&[("p_a", "skipped")]);

        let verdicts = attribute(&bindings, &batch_result(Some(0)), &stdout).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::NoTestsRan(_)));
    }

    // failed dominates skipped when both match the same pattern.
    #[test]
    fn attribution_failure_dominates_skipped() {
        let bindings = vec![entry_with_pattern("p_a")];
        let stdout = report_json(&[("p_a one", "skipped"), ("p_a two", "failed")]);

        let verdicts = attribute(&bindings, &batch_result(Some(1)), &stdout).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::TestFailing(_)));
    }

    // Unparseable structured output → whole batch falls back.
    #[test]
    fn unparseable_output_falls_back_per_binding() {
        let bindings = vec![entry_with_pattern("p_a")];
        let verdicts = attribute(&bindings, &batch_result(Some(0)), "not json");
        assert!(verdicts.is_none());
    }

    // JS-only pattern that Rust regex cannot compile → per-binding fallback.
    #[test]
    fn uncompilable_pattern_falls_back_for_that_binding() {
        let bindings = vec![entry_with_pattern("p_a"), entry_with_pattern("p_(?=b)")];
        let stdout = report_json(&[("p_b", "passed")]);

        let verdicts = attribute(&bindings, &batch_result(Some(0)), &stdout).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::NoTestsRan(_)));
        assert!(matches!(
            verdicts[1],
            BatchedVerdict::Fallback(ref reason) if reason.contains("attribution engine")
        ));
    }

    // Eligibility: only exact `--testNamePattern=<regex>` flags batch.
    #[test]
    fn batchable_pattern_recognizes_only_pattern_scoped_flags() {
        assert_eq!(
            batchable_pattern(&TestEntry {
                flags: Some("--testNamePattern=p_a".to_string()),
                command: None,
                timeout_seconds: None,
            })
            .unwrap(),
            "p_a"
        );
        assert!(batchable_pattern(&TestEntry {
            flags: Some("src/ui.test.ts".to_string()),
            command: None,
            timeout_seconds: None,
        })
        .is_none());
        assert!(batchable_pattern(&TestEntry {
            flags: Some("--testNamePattern=".to_string()),
            command: None,
            timeout_seconds: None,
        })
        .is_none());
        assert!(batchable_pattern(&TestEntry {
            flags: None,
            command: None,
            timeout_seconds: None,
        })
        .is_none());
    }

    // batch-runner-spawns task 1.4: JS-only regex constructs (lookaround,
    // backreferences) cannot compile under the Rust regex engine ah uses for
    // attribution — such bindings are excluded from batching at eligibility
    // time and keep per-binding spawns instead of paying a batched run plus
    // an attribution fallback re-run.
    #[test]
    fn batchable_pattern_rejects_js_only_regex_constructs() {
        let js_only = [
            "p_(?=b)",     // lookahead
            "p_(?!b)",     // negative lookahead
            "(?<=b)p",     // lookbehind
            "(?<!b)p",     // negative lookbehind
            "(p_a)\\1",    // backreference
            "(p_a)\\k<g>", // named backreference
            "p_\\9",       // high backreference
        ];
        for pattern in js_only {
            assert!(
                batchable_pattern(&TestEntry {
                    flags: Some(format!("--testNamePattern={pattern}")),
                    command: None,
                    timeout_seconds: None,
                })
                .is_none(),
                "JS-only pattern must not batch: {pattern}"
            );
        }
    }

    // Constructs that LOOK JS-only but are legal in both engines (or escaped
    // literals) must stay batchable — the guard is conservative but not
    // overbroad.
    #[test]
    fn batchable_pattern_keeps_dual_engine_constructs() {
        let batchable = [
            "p_a|p_b",       // alternation
            "p_\\d+",        // digit class
            "p_\\n",         // escaped letter (not a backref)
            "p_\\0",         // NUL escape, not backreference \0
            "(p_a)(p_b)",    // capturing groups
            "(?P<name>p_a)", // Rust named group
            "(?<name>p_a)",  // named group (not lookbehind)
            "\\(\\?=",       // escaped parens + equals: literal text
        ];
        for pattern in batchable {
            assert!(
                batchable_pattern(&TestEntry {
                    flags: Some(format!("--testNamePattern={pattern}")),
                    command: None,
                    timeout_seconds: None,
                })
                .is_some(),
                "dual-engine pattern must stay batchable: {pattern}"
            );
        }
    }

    // batch-runner-spawns task 2.2 (C-batch-fallback): a batched invocation
    // that degenerates (here: timed out) yields per-binding Fallback verdicts
    // carrying a named reason so the caller can re-run per-contract and
    // surface the degradation non-silently.
    #[test]
    fn run_batch_reports_fallback_with_reason_on_timed_out_batch() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("sleepy.sh");
        std::fs::write(&helper, "#!/bin/sh\nsleep 3\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&helper).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&helper, perms).unwrap();
        }
        let config = Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::from([(
                "vitest".to_string(),
                vec!["/bin/sh".to_string(), helper.to_str().unwrap().to_string()],
            )]),
            quality: Default::default(),
            capabilities: Default::default(),
            lint: Default::default(),
        };
        let bindings = vec![
            BatchableBinding {
                pattern: "p_a".to_string(),
                entry: TestEntry {
                    flags: Some("--testNamePattern=p_a".to_string()),
                    command: None,
                    timeout_seconds: Some(1),
                },
            },
            BatchableBinding {
                pattern: "p_b".to_string(),
                entry: TestEntry {
                    flags: Some("--testNamePattern=p_b".to_string()),
                    command: None,
                    timeout_seconds: Some(1),
                },
            },
        ];

        let verdicts = run_batch(dir.path(), &config, &bindings);

        assert_eq!(verdicts.len(), 2);
        for verdict in &verdicts {
            match verdict {
                BatchedVerdict::Fallback(reason) => {
                    assert!(
                        reason.contains("timed out"),
                        "fallback reason must be named: {reason}"
                    );
                }
                other => panic!("expected Fallback, got {other:?}"),
            }
        }
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

        let verdicts = attribute(&bindings, &batch_result(Some(0)), &stdout).unwrap();

        assert!(matches!(verdicts[0], BatchedVerdict::Passed));
    }

    // batch-runner-spawns task 2.3: a batched invocation whose structured
    // output exceeds the parse cap degrades to per-binding fallback instead
    // of ballooning memory or mis-attributing.
    #[test]
    fn oversized_structured_output_falls_back_per_binding() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("bulky.sh");
        // ~65 MiB of output, above the 64 MiB parse cap.
        std::fs::write(&helper, "#!/bin/sh\nhead -c 68157440 /dev/zero\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&helper).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&helper, perms).unwrap();
        }
        let config = Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::from([(
                "vitest".to_string(),
                vec!["/bin/sh".to_string(), helper.to_str().unwrap().to_string()],
            )]),
            quality: Default::default(),
            capabilities: Default::default(),
            lint: Default::default(),
        };
        let bindings = vec![entry_with_pattern("p_a"), entry_with_pattern("p_b")];

        let verdicts = run_batch(dir.path(), &config, &bindings);

        assert_eq!(verdicts.len(), 2);
        for verdict in &verdicts {
            match verdict {
                BatchedVerdict::Fallback(reason) => {
                    assert!(
                        reason.contains("parse cap"),
                        "oversized output must name the cap: {reason}"
                    );
                }
                other => panic!("expected Fallback, got {other:?}"),
            }
        }
    }

    // C-batch-timeout: the batched invocation runs under the maximum
    // timeout_seconds across its entries.
    #[test]
    fn plan_uses_max_timeout_across_entries() {
        let bindings = vec![
            BatchableBinding {
                pattern: "p_a".to_string(),
                entry: TestEntry {
                    flags: Some("--testNamePattern=p_a".to_string()),
                    command: None,
                    timeout_seconds: Some(10),
                },
            },
            BatchableBinding {
                pattern: "p_b".to_string(),
                entry: TestEntry {
                    flags: Some("--testNamePattern=p_b".to_string()),
                    command: None,
                    timeout_seconds: Some(30),
                },
            },
        ];

        let planned = plan(&vitest_config(), &bindings).unwrap();

        assert_eq!(planned.timeout_seconds, 30);
        assert_eq!(
            planned.argv[2..],
            [
                "--reporter=json".to_string(),
                "--testNamePattern=((?:p_a)|(?:p_b))".to_string()
            ]
        );
    }
}
