//! Batched runner execution for eligible vitest bindings.
//!
//! Purpose: execute many pattern-scoped vitest bindings as ONE OR-joined
//! invocation when their count exceeds `batching::BATCH_THRESHOLD`, and fall
//! back to per-binding runs otherwise.
//!
//! Responsibilities: the batch/per-binding split, per-binding attribution of
//! batched verdicts, and the named degradation warning when a batched
//! invocation cannot attribute results (C-batch-fallback).
//!
//! Rationale: runner startup dominates push-gate time, but batching must
//! never weaken falsification — a degenerate batch degrades to per-binding
//! exit-code verdicts and says so in a warning-severity finding.

use super::execute::{run_binding_fallback, ExecutionState};
use super::report::{batch_fallback_report, execution_report, no_tests_ran_report};
use super::scope::ResolvedScenario;
use crate::batching;
use crate::config;
use std::path::Path;

/// Batched runner execution (batch-runner-spawns): above the threshold, the
/// collected eligible bindings run as ONE OR-joined invocation with
/// per-binding attribution from structured output; at or below it they keep
/// per-binding spawns.
pub(crate) fn run_batched(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    sorted_scenarios: &[&ResolvedScenario],
    run: &mut ExecutionState,
) {
    let count = run.batchable.len();
    if count == 0 {
        return;
    }
    if count > batching::BATCH_THRESHOLD {
        run_batched_invocation(repo_root, cfg, specs_root, sorted_scenarios, run);
    } else {
        run_each_binding(repo_root, cfg, specs_root, sorted_scenarios, run);
    }
}

fn run_batched_invocation(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    sorted_scenarios: &[&ResolvedScenario],
    run: &mut ExecutionState,
) {
    let bindings: Vec<batching::BatchableBinding> = run
        .batchable
        .iter()
        .map(|(_, binding)| binding.clone())
        .collect();
    let verdicts = batching::run_batch(repo_root, cfg, &bindings);
    let batchable = std::mem::take(&mut run.batchable);
    for ((scenario_index, binding), verdict) in batchable.into_iter().zip(verdicts) {
        let scenario = &sorted_scenarios[scenario_index].scenario;
        match verdict {
            batching::BatchedVerdict::Passed => run.passed += 1,
            batching::BatchedVerdict::TestFailing(result) => {
                let finding = execution_report(scenario, specs_root, result);
                run.findings.push(finding);
            }
            batching::BatchedVerdict::NoTestsRan(result) => {
                let finding = no_tests_ran_report(scenario, specs_root, result);
                run.findings.push(finding);
            }
            batching::BatchedVerdict::Fallback(reason) => {
                // batch-runner-spawns task 2.2 (C-batch-fallback): the
                // degradation is named, non-silently surfaced as a
                // warning-severity finding, and the per-binding re-run
                // carries the authoritative verdict.
                let finding = batch_fallback_report(scenario, specs_root, &reason);
                run.findings.push(finding);
                run_binding_fallback(repo_root, cfg, specs_root, scenario, &binding, run);
            }
        }
    }
}

fn run_each_binding(
    repo_root: &Path,
    cfg: &config::Config,
    specs_root: &Path,
    sorted_scenarios: &[&ResolvedScenario],
    run: &mut ExecutionState,
) {
    let batchable = std::mem::take(&mut run.batchable);
    for (scenario_index, binding) in batchable {
        let scenario = &sorted_scenarios[scenario_index].scenario;
        run_binding_fallback(repo_root, cfg, specs_root, scenario, &binding, run);
    }
}
