//! Batched invocation: composition, execution, and degeneration to fallback.
//!
//! Purpose: run every eligible binding as ONE OR-joined invocation and hand
//! the captured output to attribution, never letting the batched exit code
//! produce a verdict by itself.
//!
//! Responsibilities: compose the runner argv (structured reporter, OR-joined
//! pattern, max-entry timeout) and map any degenerate outcome to named
//! per-binding fallbacks.
//!
//! Rationale: batching only pays off when its result is exactly as strict as
//! per-binding execution; anything doubtful must re-run per binding.

use super::attribution::{attribute, BatchedVerdict};
use super::{BatchableBinding, MAX_PARSE_BYTES};
use crate::config::Config;
use crate::runner::{execute_command_full, PlannedCommand, TestResult, DEFAULT_TIMEOUT_SECONDS};
use std::path::Path;

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
        Ok(captured) => captured,
        Err(error) => return fallback_all(format!("batched invocation failed: {error}")),
    };
    batched_verdicts(bindings, &result, &stdout, repo_root).unwrap_or_else(fallback_all)
}

/// Attribute the captured batched invocation. Every degenerate outcome
/// becomes a named reason (batch-runner-spawns task 2.2: non-silent fallback
/// signals) instead of a verdict.
fn batched_verdicts(
    bindings: &[BatchableBinding],
    result: &TestResult,
    stdout: &[u8],
    repo_root: &Path,
) -> Result<Vec<BatchedVerdict>, String> {
    if result.timed_out {
        return Err(
            "batched invocation timed out; per-binding re-runs carry the verdicts".to_string(),
        );
    }
    if stdout.len() > MAX_PARSE_BYTES {
        return Err(
            "batched structured output exceeds the parse cap; per-binding re-runs carry the verdicts"
                .to_string(),
        );
    }
    let text = String::from_utf8_lossy(stdout);
    attribute(bindings, result, &text, repo_root).ok_or_else(|| {
        "batched structured output unparseable; per-binding re-runs carry the verdicts".to_string()
    })
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

#[cfg(test)]
mod tests {
    use super::{plan, run_batch};
    use crate::batching::test_support::{entry_with_pattern, script_config, vitest_config};
    use crate::batching::BatchedVerdict;

    // batch-runner-spawns task 2.2 (C-batch-fallback): a batched invocation
    // that degenerates (here: timed out) yields per-binding Fallback verdicts
    // carrying a named reason so the caller can re-run per-contract and
    // surface the degradation non-silently.
    #[test]
    fn run_batch_reports_fallback_with_reason_on_timed_out_batch() {
        let dir = tempfile::tempdir().unwrap();
        let config = script_config(dir.path(), "sleepy.sh", "sleep 3");
        let mut first = entry_with_pattern("p_a");
        first.entry.timeout_seconds = Some(1);
        let mut second = entry_with_pattern("p_b");
        second.entry.timeout_seconds = Some(1);

        let verdicts = run_batch(dir.path(), &config, &[first, second]);

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

    // batch-runner-spawns task 2.3: a batched invocation whose structured
    // output exceeds the parse cap degrades to per-binding fallback instead
    // of ballooning memory or mis-attributing.
    #[test]
    fn oversized_structured_output_falls_back_per_binding() {
        let dir = tempfile::tempdir().unwrap();
        // ~65 MiB of output, above the 64 MiB parse cap.
        let config = script_config(dir.path(), "bulky.sh", "head -c 68157440 /dev/zero");
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
        let mut short = entry_with_pattern("p_a");
        short.entry.timeout_seconds = Some(10);
        let mut long = entry_with_pattern("p_b");
        long.entry.timeout_seconds = Some(30);

        let planned = plan(&vitest_config(), &[short, long]).unwrap();

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
