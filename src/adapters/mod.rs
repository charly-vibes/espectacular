pub mod python;

use crate::config::Config;
use crate::contracts::TestEntry;
use crate::runner::{self, PlannedCommand};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionSource {
    Configured,
    Manifest,
    Environment,
    SourceImport,
}

pub trait Adapter {
    fn detect(repo_root: &Path, config: &Config) -> Option<DetectionSource>;
    fn compose_command(
        repo_root: &Path,
        config: &Config,
        entry: &TestEntry,
    ) -> anyhow::Result<PlannedCommand>;
}

pub fn compose_command(
    repo_root: &Path,
    config: &Config,
    test_type: &str,
    entry: &TestEntry,
) -> anyhow::Result<PlannedCommand> {
    match test_type {
        "pytest" => python::PytestAdapter::compose_command(repo_root, config, entry),
        _ => runner::compose_command(config, test_type, entry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use std::collections::HashMap;
    use std::fs;

    fn config_with_runner(name: &str, argv: Vec<&str>) -> Config {
        let mut runners = HashMap::new();
        runners.insert(
            name.to_string(),
            argv.into_iter().map(str::to_string).collect(),
        );
        Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners,
        }
    }

    #[test]
    fn pytest_dispatch_prefers_explicit_config() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("pyproject.toml"),
            "[tool.pytest.ini_options]\n",
        )
        .unwrap();
        let config = config_with_runner("pytest", vec!["custom-pytest"]);
        let entry = TestEntry {
            flags: Some("tests/test_demo.py::test_ok".to_string()),
            command: None,
            timeout_seconds: None,
        };

        let planned = compose_command(dir.path(), &config, "pytest", &entry).unwrap();
        assert_eq!(
            planned.argv,
            vec!["custom-pytest", "tests/test_demo.py::test_ok"]
        );
    }

    #[test]
    fn non_pytest_dispatch_falls_back_to_generic_runner() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_with_runner("unit", vec!["cargo", "test"]);
        let entry = TestEntry {
            flags: Some("crate::tests::works".to_string()),
            command: None,
            timeout_seconds: Some(9),
        };

        let planned = compose_command(dir.path(), &config, "unit", &entry).unwrap();
        assert_eq!(planned.argv, vec!["cargo", "test", "crate::tests::works"]);
        assert_eq!(planned.timeout_seconds, 9);
    }
}
