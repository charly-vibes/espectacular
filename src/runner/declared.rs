//! Declared-test execution loop (contract-test surface).
//!
//! Purpose: run a declared `test_type → entries` map sequentially in sorted
//! test-type order, which is the surface `gate/p-execution.toml` binds
//! (`p_execution`).
//!
//! Responsibilities: stable ordering, then compose + execute each entry.
//!
//! Rationale: production execution goes through `adapters::invoke` (incl.
//! batching); this loop stays as the directly testable reference order.

use super::{compose_command, execute_command, TestResult};
use crate::config::Config;
use crate::contracts::TestEntry;
use std::path::Path;

/// Sequential declared-test loop; kept as contract-test surface
/// (gate/p-execution.toml binds `p_execution`, whose helpers drive it).
/// Production execution goes through `adapters::invoke` (incl. batching).
#[allow(dead_code)]
pub fn run_declared_tests(
    repo_root: &Path,
    config: &Config,
    tests: &std::collections::HashMap<String, Vec<TestEntry>>,
) -> anyhow::Result<Vec<TestResult>> {
    let mut test_types: Vec<_> = tests.keys().cloned().collect();
    test_types.sort();

    let mut results = Vec::new();
    for test_type in test_types {
        let entries = &tests[&test_type];
        for entry in entries {
            let planned = compose_command(config, &test_type, entry)?;
            results.push(execute_command(repo_root, &planned)?);
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::run_declared_tests;
    use crate::config::{Config, Paths};
    use crate::runner::test_support::{flags_entry, write_helper};
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    /// Both test types point at the same helper, so the observed order is
    /// purely the loop's sort order.
    fn sorted_order_config(helper: &Path) -> Config {
        let script = helper.to_str().unwrap().to_string();
        Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::from([
                (
                    "pbt".to_string(),
                    vec!["/bin/sh".to_string(), script.clone()],
                ),
                ("unit".to_string(), vec!["/bin/sh".to_string(), script]),
            ]),
            quality: Default::default(),
            capabilities: Default::default(),
            lint: Default::default(),
        }
    }

    #[test]
    fn declared_tests_run_sequentially_in_sorted_test_type_order() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("order.txt");
        let helper = write_helper(
            dir.path(),
            &format!("printf '%s\n' \"$1\" >> {}", out.display()),
        );
        let config = sorted_order_config(&helper);
        let tests = HashMap::from([
            (
                "unit".to_string(),
                vec![flags_entry("u1"), flags_entry("u2")],
            ),
            ("pbt".to_string(), vec![flags_entry("p1")]),
        ]);

        let results = run_declared_tests(dir.path(), &config, &tests).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(fs::read_to_string(out).unwrap(), "p1\nu1\nu2\n");
    }
}
