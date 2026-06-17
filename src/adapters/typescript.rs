use super::DetectionSource;
use crate::config::Config;
use crate::contracts::TestEntry;
use crate::runner::{PlannedCommand, TestResult};
use std::fs;
use std::path::Path;

const DEFAULT_TIMEOUT_SECONDS: u64 = 60;

pub struct VitestAdapter;

impl super::Adapter for VitestAdapter {
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
    if config.runners.contains_key("vitest") {
        return Some(DetectionSource::Configured);
    }
    if has_vitest_in_package_json(repo_root) {
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
        .ok_or_else(|| anyhow::anyhow!("vitest test entry must declare flags"))?;

    let mut argv = if let Some(runner) = config.runners.get("vitest") {
        runner.clone()
    } else if let Some(source) = detect(repo_root, config) {
        anyhow::bail!(
            "vitest detected via {} but is not configured",
            detection_source_label(source)
        );
    } else {
        anyhow::bail!("missing adapter for vitest");
    };
    argv.push(flags.clone());

    Ok(PlannedCommand {
        test_type: "vitest".to_string(),
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
    if combined.contains("Transform failed")
        || combined.contains("Failed to transform")
        || combined.contains("SyntaxError")
        || combined.contains("transform error")
    {
        "vitest-transform-error"
    } else {
        "vitest"
    }
}

fn has_vitest_in_package_json(repo_root: &Path) -> bool {
    let package_json = repo_root.join("package.json");
    if let Ok(text) = fs::read_to_string(package_json) {
        return text.contains("\"vitest\"");
    }
    false
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
            capabilities: Default::default(),
        }
    }

    fn config_with_vitest(argv: Vec<&str>) -> Config {
        let mut runners = HashMap::new();
        runners.insert(
            "vitest".to_string(),
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

    // --- 5.1 RED: detection ---

    #[test]
    fn detects_vitest_via_package_json_devdependency() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies":{"vitest":"^1.0.0"}}"#,
        )
        .unwrap();

        assert_eq!(
            detect(dir.path(), &empty_config()),
            Some(DetectionSource::Manifest)
        );
    }

    #[test]
    fn detects_vitest_via_package_json_dependency() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"vitest":"^1.0.0"}}"#,
        )
        .unwrap();

        assert_eq!(
            detect(dir.path(), &empty_config()),
            Some(DetectionSource::Manifest)
        );
    }

    #[test]
    fn detects_vitest_via_configured_runner() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_with_vitest(vec!["npx", "vitest", "--reporter=json"]);

        assert_eq!(
            detect(dir.path(), &config),
            Some(DetectionSource::Configured)
        );
    }

    #[test]
    fn no_detection_without_vitest_in_package_json_or_config() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies":{"jest":"^29.0.0"}}"#,
        )
        .unwrap();

        assert_eq!(detect(dir.path(), &empty_config()), None);
    }

    #[test]
    fn no_detection_without_package_json() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(detect(dir.path(), &empty_config()), None);
    }

    #[test]
    fn configured_runner_takes_precedence_over_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies":{"vitest":"^1.0.0"}}"#,
        )
        .unwrap();
        let config = config_with_vitest(vec!["npx", "vitest"]);

        assert_eq!(
            detect(dir.path(), &config),
            Some(DetectionSource::Configured)
        );
    }

    // --- 5.3 RED: invocation and normalization ---

    #[test]
    fn compose_requires_configured_vitest_runner() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies":{"vitest":"^1.0.0"}}"#,
        )
        .unwrap();
        let entry = TestEntry {
            flags: Some("src/foo.test.ts".to_string()),
            command: None,
            timeout_seconds: Some(10),
        };

        let error = compose_command(dir.path(), &empty_config(), &entry).unwrap_err();
        assert!(error
            .to_string()
            .contains("vitest detected via manifest but is not configured"));
    }

    #[test]
    fn compose_appends_flags_to_configured_runner() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_with_vitest(vec!["npx", "vitest", "--reporter=json"]);
        let entry = TestEntry {
            flags: Some("src/foo.test.ts".to_string()),
            command: None,
            timeout_seconds: Some(10),
        };

        let planned = compose_command(dir.path(), &config, &entry).unwrap();
        assert_eq!(
            planned.argv,
            vec!["npx", "vitest", "--reporter=json", "src/foo.test.ts"]
        );
        assert_eq!(planned.timeout_seconds, 10);
        assert_eq!(planned.test_type, "vitest");
    }

    #[test]
    fn compose_uses_default_timeout_when_not_specified() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_with_vitest(vec!["npx", "vitest"]);
        let entry = TestEntry {
            flags: Some("src/foo.test.ts".to_string()),
            command: None,
            timeout_seconds: None,
        };

        let planned = compose_command(dir.path(), &config, &entry).unwrap();
        assert_eq!(planned.timeout_seconds, DEFAULT_TIMEOUT_SECONDS);
    }

    #[test]
    fn normalize_classifies_transform_errors() {
        let result = normalize(TestResult {
            test_type: "vitest".to_string(),
            command: "npx vitest src/foo.test.ts".to_string(),
            exit_code: Some(1),
            timed_out: false,
            stdout_tail: String::new(),
            stderr_tail: "Transform failed with 1 error:\nsrc/foo.ts:3:1: ERROR (transform error)"
                .to_string(),
        });

        assert_eq!(result.test_type, "vitest-transform-error");
    }

    #[test]
    fn normalize_classifies_syntax_errors_as_transform() {
        let result = normalize(TestResult {
            test_type: "vitest".to_string(),
            command: "npx vitest src/foo.test.ts".to_string(),
            exit_code: Some(1),
            timed_out: false,
            stdout_tail: String::new(),
            stderr_tail: "SyntaxError: Unexpected token 'export'".to_string(),
        });

        assert_eq!(result.test_type, "vitest-transform-error");
    }

    #[test]
    fn normalize_classifies_test_failures_without_transform_error() {
        let result = normalize(TestResult {
            test_type: "vitest".to_string(),
            command: "npx vitest src/foo.test.ts".to_string(),
            exit_code: Some(1),
            timed_out: false,
            stdout_tail: r#"{"numFailedTests":1,"numPassedTests":0}"#.to_string(),
            stderr_tail: String::new(),
        });

        assert_eq!(result.test_type, "vitest");
    }

    #[test]
    fn normalize_passes_through_successful_result() {
        let result = normalize(TestResult {
            test_type: "vitest".to_string(),
            command: "npx vitest src/foo.test.ts".to_string(),
            exit_code: Some(0),
            timed_out: false,
            stdout_tail: r#"{"numFailedTests":0,"numPassedTests":3}"#.to_string(),
            stderr_tail: String::new(),
        });

        assert_eq!(result.test_type, "vitest");
    }
}
