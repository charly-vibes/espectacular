pub mod custom;
pub mod python;
pub mod rust;
pub mod typescript;

use crate::config::Config;
use crate::contracts::TestEntry;
use crate::runner::{self, PlannedCommand, TestResult};
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
    fn normalize(result: TestResult) -> TestResult {
        result
    }
    fn invoke(repo_root: &Path, config: &Config, entry: &TestEntry) -> anyhow::Result<TestResult> {
        let planned = Self::compose_command(repo_root, config, entry)?;
        let result = runner::execute_command(repo_root, &planned)?;
        Ok(Self::normalize(result))
    }
}

pub fn detection_source_label(source: DetectionSource) -> &'static str {
    match source {
        DetectionSource::Configured => "configured",
        DetectionSource::Manifest => "manifest",
        DetectionSource::Environment => "environment",
        DetectionSource::SourceImport => "source_import",
    }
}

pub fn detect(repo_root: &Path, config: &Config, test_type: &str) -> Option<DetectionSource> {
    match test_type {
        "pytest" => python::PytestAdapter::detect(repo_root, config),
        "cargo" => rust::CargoAdapter::detect(repo_root, config),
        "vitest" => typescript::VitestAdapter::detect(repo_root, config),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn compose_command(
    repo_root: &Path,
    config: &Config,
    test_type: &str,
    entry: &TestEntry,
) -> anyhow::Result<PlannedCommand> {
    match test_type {
        "pytest" => python::PytestAdapter::compose_command(repo_root, config, entry),
        "cargo" => rust::CargoAdapter::compose_command(repo_root, config, entry),
        "vitest" => typescript::VitestAdapter::compose_command(repo_root, config, entry),
        _ => runner::compose_command(config, test_type, entry),
    }
}

pub fn invoke(
    repo_root: &Path,
    config: &Config,
    test_type: &str,
    entry: &TestEntry,
) -> anyhow::Result<TestResult> {
    match test_type {
        "pytest" => python::PytestAdapter::invoke(repo_root, config, entry),
        "cargo" => rust::CargoAdapter::invoke(repo_root, config, entry),
        "vitest" => typescript::VitestAdapter::invoke(repo_root, config, entry),
        _ => {
            let planned = runner::compose_command(config, test_type, entry)?;
            runner::execute_command(repo_root, &planned)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

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
            quality: Default::default(),
            capabilities: Default::default(),
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

    fn write_executable(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn detect_dispatch_reports_pytest_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("tests")).unwrap();
        fs::write(repo.join("pyproject.toml"), "[tool.pytest.ini_options]\n").unwrap();
        fs::write(
            repo.join("tests/test_demo.py"),
            "import pytest\n\ndef test_ok():\n    assert True\n",
        )
        .unwrap();

        let env_dir = repo.join("bin");
        fs::create_dir_all(&env_dir).unwrap();
        write_executable(&env_dir.join("pytest"), "exit 0");
        let path = std::env::join_paths([env_dir]).unwrap();
        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", path);

        let empty = Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::new(),
            quality: Default::default(),
            capabilities: Default::default(),
        };
        let configured = config_with_runner("pytest", vec!["pytest"]);

        assert_eq!(
            detect(repo, &configured, "pytest"),
            Some(DetectionSource::Configured)
        );
        assert_eq!(
            detect(repo, &empty, "pytest"),
            Some(DetectionSource::Manifest)
        );

        fs::remove_file(repo.join("pyproject.toml")).unwrap();
        assert_eq!(
            detect(repo, &empty, "pytest"),
            Some(DetectionSource::Environment)
        );

        std::env::remove_var("PATH");
        assert_eq!(
            detect(repo, &empty, "pytest"),
            Some(DetectionSource::SourceImport)
        );

        if let Some(path) = original_path {
            std::env::set_var("PATH", path);
        }
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
