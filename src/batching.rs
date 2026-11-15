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
//!
//! Module map: `eligibility` decides which bindings may batch, `execute`
//! composes and runs the single invocation, `attribution` derives each
//! binding's verdict from the structured report. This root owns the shared
//! constants and re-exports the crate-visible surface.

mod attribution;
mod eligibility;
mod execute;

pub(crate) use attribution::BatchedVerdict;
pub(crate) use eligibility::{batchable_pattern, BatchableBinding};
pub(crate) use execute::run_batch;

/// Bindings at or below this count keep per-binding spawns; above it, one
/// runner-type batch replaces them. Design: ~1s vitest startup per spawn
/// makes 8 the amortization crossover on measured corpora.
pub(crate) const BATCH_THRESHOLD: usize = 8;

/// Sanity cap on the parse buffer for structured output (design: batched
/// JSON can be megabytes; a runaway runner must degrade to fallback, not
/// balloon memory).
const MAX_PARSE_BYTES: usize = 64 * 1024 * 1024;

/// Fixtures shared by the module tests: configs, bindings, and report JSON.
#[cfg(test)]
pub(crate) mod test_support {
    use super::BatchableBinding;
    use crate::config::{Config, Paths};
    use crate::contracts::TestEntry;
    use crate::runner::TestResult;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    pub(crate) fn vitest_config() -> Config {
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

    pub(crate) fn entry_with_pattern(pattern: &str) -> BatchableBinding {
        BatchableBinding {
            pattern: pattern.to_string(),
            entry: TestEntry {
                flags: Some(format!("--testNamePattern={pattern}")),
                command: None,
                timeout_seconds: None,
            },
        }
    }

    pub(crate) fn batch_result(exit_code: Option<i32>) -> TestResult {
        TestResult {
            test_type: "vitest".to_string(),
            command: "vitest run --reporter=json".to_string(),
            exit_code,
            timed_out: false,
            stdout_tail: String::new(),
            stderr_tail: String::new(),
        }
    }

    pub(crate) fn report_json(assertions: &[(&str, &str)]) -> String {
        let items = assertions
            .iter()
            .map(|(name, status)| format!(r#"{{"fullName": "{name}", "status": "{status}"}}"#))
            .collect::<Vec<_>>()
            .join(", ");
        format!(r#"{{"testResults": [{{"assertionResults": [{items}]}}]}}"#)
    }

    /// Write an executable `name` script running `body` into `dir` and return
    /// a vitest config that invokes it through `/bin/sh` (stands in for a
    /// real runner while keeping the invocation shape identical).
    pub(crate) fn script_config(dir: &Path, name: &str, body: &str) -> Config {
        let helper = write_script(dir, name, body);
        Config {
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
        }
    }

    fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let helper = dir.join(name);
        std::fs::write(&helper, format!("#!/bin/sh\n{body}\n")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&helper).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&helper, perms).unwrap();
        }
        helper
    }
}
