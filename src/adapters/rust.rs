use super::DetectionSource;
use crate::config::Config;
use crate::contracts::TestEntry;
use crate::runner::{PlannedCommand, TestResult};
use std::fs;
use std::path::Path;

const DEFAULT_TIMEOUT_SECONDS: u64 = 120;

pub struct CargoAdapter;

impl super::Adapter for CargoAdapter {
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
    if config.runners.contains_key("cargo") {
        return Some(DetectionSource::Configured);
    }
    if repo_root.join("Cargo.toml").exists() {
        return Some(DetectionSource::Manifest);
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
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow::anyhow!("cargo test entry must declare flags"))?;

    let mut argv = if let Some(runner) = config.runners.get("cargo") {
        runner.clone()
    } else if let Some(source) = detect(repo_root, config) {
        anyhow::bail!(
            "cargo detected via {} but is not configured",
            detection_source_label(source)
        );
    } else {
        anyhow::bail!("missing adapter for cargo");
    };
    argv.push(flags.clone());

    Ok(PlannedCommand {
        test_type: "cargo".to_string(),
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
    let combined = format!("{stdout}\n{stderr}");
    if combined.contains("error[E") || combined.contains("error: could not compile") {
        "cargo-build-error"
    } else {
        "cargo-test"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use std::collections::HashMap;

    fn empty_config() -> Config {
        Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::new(),
            quality: Default::default(),
        }
    }

    fn config_with_cargo(argv: Vec<&str>) -> Config {
        let mut runners = HashMap::new();
        runners.insert(
            "cargo".to_string(),
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
        }
    }

    // --- 4.1 RED: detection ---

    #[test]
    fn detects_cargo_via_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"foo\"\n").unwrap();

        assert_eq!(
            detect(dir.path(), &empty_config()),
            Some(DetectionSource::Manifest)
        );
    }

    #[test]
    fn detects_cargo_via_configured_runner() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_with_cargo(vec!["cargo", "test"]);

        assert_eq!(
            detect(dir.path(), &config),
            Some(DetectionSource::Configured)
        );
    }

    #[test]
    fn no_detection_without_cargo_toml_or_config() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(detect(dir.path(), &empty_config()), None);
    }

    #[test]
    fn configured_runner_takes_precedence_over_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"foo\"\n").unwrap();
        let config = config_with_cargo(vec!["cargo", "test"]);

        assert_eq!(
            detect(dir.path(), &config),
            Some(DetectionSource::Configured)
        );
    }

    // --- 4.3 RED: invocation and normalization ---

    #[test]
    fn compose_requires_configured_cargo_runner() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"foo\"\n").unwrap();
        let entry = TestEntry {
            flags: Some("some_module::tests::it_works".to_string()),
            command: None,
            timeout_seconds: Some(30),
        };

        let error = compose_command(dir.path(), &empty_config(), &entry).unwrap_err();
        assert!(error
            .to_string()
            .contains("cargo detected via manifest but is not configured"));
    }

    #[test]
    fn compose_appends_flags_to_configured_runner() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_with_cargo(vec!["cargo", "test"]);
        let entry = TestEntry {
            flags: Some("some_module::tests::it_works".to_string()),
            command: None,
            timeout_seconds: Some(30),
        };

        let planned = compose_command(dir.path(), &config, &entry).unwrap();
        assert_eq!(
            planned.argv,
            vec!["cargo", "test", "some_module::tests::it_works"]
        );
        assert_eq!(planned.timeout_seconds, 30);
        assert_eq!(planned.test_type, "cargo");
    }

    #[test]
    fn compose_uses_default_timeout_when_not_specified() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_with_cargo(vec!["cargo", "test"]);
        let entry = TestEntry {
            flags: Some("tests::works".to_string()),
            command: None,
            timeout_seconds: None,
        };

        let planned = compose_command(dir.path(), &config, &entry).unwrap();
        assert_eq!(planned.timeout_seconds, DEFAULT_TIMEOUT_SECONDS);
    }

    #[test]
    fn normalize_classifies_compile_errors() {
        let result = normalize(TestResult {
            test_type: "cargo".to_string(),
            command: "cargo test some::test".to_string(),
            exit_code: Some(101),
            timed_out: false,
            stdout_tail: String::new(),
            stderr_tail: "error[E0308]: mismatched types\nerror: could not compile `foo`"
                .to_string(),
        });

        assert_eq!(result.test_type, "cargo-build-error");
    }

    #[test]
    fn normalize_classifies_test_failures_without_compile_error() {
        let result = normalize(TestResult {
            test_type: "cargo".to_string(),
            command: "cargo test some::test".to_string(),
            exit_code: Some(101),
            timed_out: false,
            stdout_tail: "test some_test ... FAILED\ntest result: FAILED. 0 passed; 1 failed"
                .to_string(),
            stderr_tail: String::new(),
        });

        assert_eq!(result.test_type, "cargo-test");
    }

    #[test]
    fn normalize_passes_through_successful_result() {
        let result = normalize(TestResult {
            test_type: "cargo".to_string(),
            command: "cargo test some::test".to_string(),
            exit_code: Some(0),
            timed_out: false,
            stdout_tail: "test result: ok. 1 passed".to_string(),
            stderr_tail: String::new(),
        });

        assert_eq!(result.test_type, "cargo");
    }
}
