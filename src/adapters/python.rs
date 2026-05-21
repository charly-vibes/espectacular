use super::DetectionSource;
use crate::config::Config;
use crate::contracts::TestEntry;
use crate::runner::{PlannedCommand, TestResult};
use std::env;
use std::fs;
use std::path::Path;

const DEFAULT_TIMEOUT_SECONDS: u64 = 60;
const IGNORED_DIRS: &[&str] = &[".git", "node_modules", "dist", "build", ".venv", "target"];

pub struct PytestAdapter;

impl super::Adapter for PytestAdapter {
    fn detect(repo_root: &Path, config: &Config) -> Option<DetectionSource> {
        detect(repo_root, config)
    }

    fn compose_command(
        repo_root: &Path,
        config: &Config,
        entry: &TestEntry,
    ) -> anyhow::Result<PlannedCommand> {
        compose_command(repo_root, config, entry)
    }

    fn normalize(result: TestResult) -> TestResult {
        normalize(result)
    }
}

pub fn detect(repo_root: &Path, config: &Config) -> Option<DetectionSource> {
    detect_with_path(repo_root, config, env::var_os("PATH"))
}

fn detect_with_path(
    repo_root: &Path,
    config: &Config,
    path_override: Option<std::ffi::OsString>,
) -> Option<DetectionSource> {
    if config.runners.contains_key("pytest") {
        return Some(DetectionSource::Configured);
    }
    if has_manifest_signal(repo_root) {
        return Some(DetectionSource::Manifest);
    }
    if pytest_on_path(path_override) {
        return Some(DetectionSource::Environment);
    }
    if has_source_import_signal(repo_root) {
        return Some(DetectionSource::SourceImport);
    }
    None
}

pub fn compose_command(
    repo_root: &Path,
    config: &Config,
    entry: &TestEntry,
) -> anyhow::Result<PlannedCommand> {
    let timeout_seconds = entry.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS);
    anyhow::ensure!(timeout_seconds > 0, "timeout_seconds must be positive");

    let flags = entry
        .flags
        .as_ref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("pytest test entry must declare flags"))?;

    let mut argv = if let Some(runner) = config.runners.get("pytest") {
        runner.clone()
    } else if let Some(source) = detect(repo_root, config) {
        anyhow::bail!(
            "pytest detected via {} but is not configured",
            detection_source_label(source)
        );
    } else {
        anyhow::bail!("missing adapter for pytest");
    };
    argv.push(flags.clone());

    Ok(PlannedCommand {
        test_type: "pytest".to_string(),
        display: argv.join(" "),
        argv,
        timeout_seconds,
    })
}

pub fn normalize(mut result: TestResult) -> TestResult {
    if result.timed_out || result.exit_code != Some(0) {
        result.test_type = classify_failure(&result.stdout_tail, &result.stderr_tail).to_string();
    }
    result
}

fn classify_failure(stdout: &str, stderr: &str) -> &'static str {
    if let Some(json_kind) = classify_failure_from_json(stdout) {
        return json_kind;
    }

    let combined = format!("{stdout}\n{stderr}");
    if combined.contains("ImportError") {
        "pytest-import-error"
    } else if combined.contains("fixture") && combined.contains("not found") {
        "pytest-fixture-error"
    } else if combined.contains("ERROR collecting")
        || combined.contains("collected 0 items / 1 error")
    {
        "pytest-collection-error"
    } else {
        "pytest"
    }
}

fn classify_failure_from_json(stdout: &str) -> Option<&'static str> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    if json_contains(&value, "ImportError") {
        return Some("pytest-import-error");
    }
    if json_contains(&value, "fixture") && json_contains(&value, "not found") {
        return Some("pytest-fixture-error");
    }
    if json_contains(&value, "ERROR collecting")
        || json_contains(&value, "collected 0 items / 1 error")
    {
        return Some("pytest-collection-error");
    }
    None
}

fn json_contains(value: &serde_json::Value, needle: &str) -> bool {
    match value {
        serde_json::Value::String(text) => text.contains(needle),
        serde_json::Value::Array(items) => items.iter().any(|item| json_contains(item, needle)),
        serde_json::Value::Object(map) => map.values().any(|item| json_contains(item, needle)),
        _ => false,
    }
}

fn detection_source_label(source: DetectionSource) -> &'static str {
    match source {
        DetectionSource::Configured => "configured",
        DetectionSource::Manifest => "manifest",
        DetectionSource::Environment => "environment",
        DetectionSource::SourceImport => "source_import",
    }
}

fn has_manifest_signal(repo_root: &Path) -> bool {
    let pyproject = repo_root.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(text) = fs::read_to_string(pyproject) {
            if text.contains("[tool.pytest.ini_options]") || text.contains("pytest") {
                return true;
            }
        }
    }

    let pytest_ini = repo_root.join("pytest.ini");
    if pytest_ini.exists() {
        return true;
    }

    let setup_cfg = repo_root.join("setup.cfg");
    if setup_cfg.exists() {
        if let Ok(text) = fs::read_to_string(setup_cfg) {
            if text.contains("[tool:pytest]") {
                return true;
            }
        }
    }

    false
}

fn pytest_on_path(path_override: Option<std::ffi::OsString>) -> bool {
    let Some(path) = path_override else {
        return false;
    };
    env::split_paths(&path).any(|dir| {
        let candidate = dir.join("pytest");
        candidate.is_file()
    })
}

fn has_source_import_signal(repo_root: &Path) -> bool {
    scan_dir_for_pytest_import(repo_root)
}

fn scan_dir_for_pytest_import(path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };

    for entry in entries.flatten() {
        let child = entry.path();
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if IGNORED_DIRS.contains(&name.as_ref()) {
                continue;
            }
            if scan_dir_for_pytest_import(&child) {
                return true;
            }
            continue;
        }

        let Some(name) = child.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !(name.starts_with("test_") || name.ends_with("_test.py")) {
            continue;
        }
        if child.extension().and_then(|value| value.to_str()) != Some("py") {
            continue;
        }
        if let Ok(text) = fs::read_to_string(&child) {
            if text.contains("import pytest") || text.contains("from pytest import") {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use std::collections::HashMap;
    use std::os::unix::fs::PermissionsExt;

    fn empty_config() -> Config {
        Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::new(),
        }
    }

    fn write_executable(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn detects_pytest_via_pyproject() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("pyproject.toml"),
            "[tool.pytest.ini_options]\n",
        )
        .unwrap();

        assert_eq!(
            detect_with_path(dir.path(), &empty_config(), None),
            Some(DetectionSource::Manifest)
        );
    }

    #[test]
    fn detects_pytest_via_pytest_ini() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pytest.ini"), "[pytest]\n").unwrap();

        assert_eq!(
            detect_with_path(dir.path(), &empty_config(), None),
            Some(DetectionSource::Manifest)
        );
    }

    #[test]
    fn detects_pytest_via_environment() {
        let dir = tempfile::tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        write_executable(&bin_dir.join("pytest"), "exit 0");
        let joined = env::join_paths(std::iter::once(bin_dir.clone())).unwrap();

        let detected = detect_with_path(dir.path(), &empty_config(), Some(joined));
        assert_eq!(detected, Some(DetectionSource::Environment));
    }

    #[test]
    fn detects_pytest_via_source_import() {
        let dir = tempfile::tempdir().unwrap();
        let tests_dir = dir.path().join("tests");
        fs::create_dir_all(&tests_dir).unwrap();
        fs::write(
            tests_dir.join("test_demo.py"),
            "import pytest\n\ndef test_ok():\n    assert True\n",
        )
        .unwrap();

        let detected = detect_with_path(dir.path(), &empty_config(), None);
        assert_eq!(detected, Some(DetectionSource::SourceImport));
    }

    #[test]
    fn detects_pytest_via_setup_cfg() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("setup.cfg"), "[tool:pytest]\n").unwrap();

        assert_eq!(
            detect_with_path(dir.path(), &empty_config(), None),
            Some(DetectionSource::Manifest)
        );
    }

    #[test]
    fn compose_requires_configured_pytest_runner() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pytest.ini"), "[pytest]\n").unwrap();
        let entry = TestEntry {
            flags: Some("tests/test_demo.py::test_ok".to_string()),
            command: None,
            timeout_seconds: Some(5),
        };

        let error = compose_command(dir.path(), &empty_config(), &entry).unwrap_err();
        assert!(error
            .to_string()
            .contains("pytest detected via manifest but is not configured"));
    }

    #[test]
    fn normalize_classifies_import_errors_from_json_report() {
        let result = normalize(TestResult {
            test_type: "pytest".to_string(),
            command: "pytest tests/test_demo.py::test_import".to_string(),
            exit_code: Some(2),
            timed_out: false,
            stdout_tail:
                r#"{"collectors":[{"longrepr":"ImportError: cannot import name 'boom'"}]}"#
                    .to_string(),
            stderr_tail: String::new(),
        });

        assert_eq!(result.test_type, "pytest-import-error");
    }

    #[test]
    fn normalize_classifies_fixture_errors_from_json_report() {
        let result = normalize(TestResult {
            test_type: "pytest".to_string(),
            command: "pytest tests/test_demo.py::test_fixture".to_string(),
            exit_code: Some(1),
            timed_out: false,
            stdout_tail: r#"{"tests":[{"setup":{"crash":{"message":"fixture 'db' not found"}}}]}"#
                .to_string(),
            stderr_tail: String::new(),
        });

        assert_eq!(result.test_type, "pytest-fixture-error");
    }

    #[test]
    fn normalize_classifies_collection_errors_from_json_report() {
        let result = normalize(TestResult {
            test_type: "pytest".to_string(),
            command: "pytest tests/test_demo.py::test_collect".to_string(),
            exit_code: Some(2),
            timed_out: false,
            stdout_tail: r#"{"collectors":[{"longrepr":"ERROR collecting tests/test_demo.py"}]}"#
                .to_string(),
            stderr_tail: String::new(),
        });

        assert_eq!(result.test_type, "pytest-collection-error");
    }
}
