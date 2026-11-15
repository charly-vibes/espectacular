//! Runner execution: planned commands, spawn + timeout supervision, and
//! structured-output capture for batched attribution.
//!
//! Purpose: turn contract test bindings into executed, bounded, captured
//! invocations whose results feed both exit-code verdicts (per-binding) and
//! structured per-test attribution (batched, src/batching.rs).
//!
//! Responsibilities: compose `PlannedCommand`s per binding type (shell via
//! `/bin/sh -c`, runners via configured argv + flags), execute with process-
//! group isolation and deadline kill, drain stdout/stderr on reader threads
//! while polling (oversized output must not deadlock), keep 8 KiB findings
//! tails, and detect matched-zero runs (DDL 4.3 guard).
//!
//! Rationale: push-gate time is dominated by runner startup (GH#40); bounded,
//! deadlock-free capture is the precondition for batching many contracts
//! into one invocation without weakening per-contract falsification.
//!
//! Module map: `command` composes argv, `execute` spawns and supervises,
//! `guards` holds the matched-zero guard, `declared` keeps the declared-test
//! contract surface. This root owns the shared types and constants.

mod command;
mod declared;
mod execute;
mod guards;

pub use command::compose_command;
pub use execute::execute_command;
pub(crate) use execute::{execute_command_full, spawn_with_text_busy_retry};
pub(crate) use guards::matched_zero_tests;

use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_TIMEOUT_SECONDS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedCommand {
    pub test_type: String,
    pub argv: Vec<String>,
    pub display: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestResult {
    #[serde(rename = "type")]
    pub test_type: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

/// Fixtures shared by the module tests: configs, entries, helper scripts.
#[cfg(test)]
pub(crate) mod test_support {
    use crate::config::{Config, Paths};
    use crate::contracts::TestEntry;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    pub(crate) fn config_with_runner(test_type: &str, runner: Vec<&str>) -> Config {
        let mut runners = HashMap::new();
        runners.insert(
            test_type.to_string(),
            runner.into_iter().map(str::to_string).collect(),
        );
        Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners,
            quality: Default::default(),
            capabilities: Default::default(),
            lint: Default::default(),
        }
    }

    pub(crate) fn flags_entry(flags: &str) -> TestEntry {
        TestEntry {
            flags: Some(flags.to_string()),
            command: None,
            timeout_seconds: None,
        }
    }

    pub(crate) fn write_helper(repo_root: &Path, body: &str) -> PathBuf {
        let path = repo_root.join("helper.sh");
        fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).unwrap();
        }
        path
    }
}
