use crate::config::Config;
use crate::contracts::TestEntry;
use crate::runner::{self, PlannedCommand, TestResult};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TAIL_BYTES: usize = 8 * 1024;

pub enum CustomRunnerResult {
    Passed,
    EnvelopeFindings(Vec<Value>),
    TestFailing(TestResult),
}

#[derive(Deserialize)]
struct Envelope {
    passed: bool,
    findings: Vec<Value>,
}

pub fn invoke(
    repo_root: &Path,
    config: &Config,
    entry: &TestEntry,
) -> anyhow::Result<CustomRunnerResult> {
    let planned = runner::compose_command(config, "custom", entry)?;
    let (result, raw_stdout) = run(repo_root, &planned)?;

    if result.timed_out || result.exit_code != Some(0) {
        return Ok(CustomRunnerResult::TestFailing(result));
    }

    match serde_json::from_str::<Envelope>(&raw_stdout) {
        Err(_) => Ok(CustomRunnerResult::TestFailing(result)),
        Ok(env) if env.passed && env.findings.is_empty() => Ok(CustomRunnerResult::Passed),
        Ok(env) => Ok(CustomRunnerResult::EnvelopeFindings(env.findings)),
    }
}

fn run(repo_root: &Path, planned: &PlannedCommand) -> anyhow::Result<(TestResult, String)> {
    let mut child = Command::new(&planned.argv[0])
        .args(&planned.argv[1..])
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let started = Instant::now();
    let timeout = Duration::from_secs(planned.timeout_seconds);
    let timed_out = loop {
        if child.try_wait()?.is_some() {
            break false;
        }
        if started.elapsed() >= timeout {
            child.kill()?;
            break true;
        }
        thread::sleep(Duration::from_millis(10));
    };

    let output = child.wait_with_output()?;
    let raw_stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let result = TestResult {
        test_type: planned.test_type.clone(),
        command: planned.display.clone(),
        exit_code: output.status.code(),
        timed_out,
        stdout_tail: tail(&output.stdout),
        stderr_tail: tail(&output.stderr),
    };

    Ok((result, raw_stdout))
}

fn tail(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Paths, QualityConfig};
    use crate::contracts::TestEntry;
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn config_with_custom(argv: Vec<&str>) -> Config {
        let mut runners = HashMap::new();
        runners.insert(
            "custom".to_string(),
            argv.into_iter().map(str::to_string).collect(),
        );
        Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners,
            quality: QualityConfig::default(),
        }
    }

    fn config_no_custom() -> Config {
        Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::new(),
            quality: QualityConfig::default(),
        }
    }

    fn flags_entry(flags: &str) -> TestEntry {
        TestEntry {
            flags: Some(flags.to_string()),
            command: None,
            timeout_seconds: None,
        }
    }

    fn write_runner(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("runner.sh");
        fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    fn write_envelope(dir: &Path, json: &str) -> std::path::PathBuf {
        let data = dir.join("envelope.json");
        fs::write(&data, json).unwrap();
        let script = dir.join("runner.sh");
        fs::write(&script, format!("#!/bin/sh\ncat {}\n", data.display())).unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
        script
    }

    // 6.1 RED: envelope parsing — passed=true, empty findings, exit 0 → Passed

    #[test]
    fn valid_passed_envelope_is_passed() {
        let dir = tempfile::tempdir().unwrap();
        let runner = write_envelope(dir.path(), r#"{"exit_code":0,"passed":true,"findings":[]}"#);
        let config = config_with_custom(vec![runner.to_str().unwrap()]);

        let result = invoke(dir.path(), &config, &flags_entry("s")).unwrap();
        assert!(matches!(result, CustomRunnerResult::Passed));
    }

    #[test]
    fn envelope_with_findings_produces_envelope_findings() {
        let dir = tempfile::tempdir().unwrap();
        let finding = serde_json::json!({
            "kind": "test-failing",
            "category": "execution",
            "spec": "foo",
            "spec_path": "openspec/specs/foo/spec.md",
            "scenario": {"id": "s", "title": "T", "body_markdown": "B"},
            "suggested_action": "edit_code_not_scenario",
            "playbook_command": "ah explain edit_code_not_scenario"
        });
        let envelope = serde_json::json!({
            "exit_code": 0,
            "passed": false,
            "findings": [finding]
        });
        let runner = write_envelope(dir.path(), &envelope.to_string());
        let config = config_with_custom(vec![runner.to_str().unwrap()]);

        let result = invoke(dir.path(), &config, &flags_entry("s")).unwrap();
        assert!(matches!(result, CustomRunnerResult::EnvelopeFindings(ref v) if v.len() == 1));
    }

    // 6.3 RED: non-zero exit without valid envelope → TestFailing

    #[test]
    fn non_zero_exit_produces_test_failing() {
        let dir = tempfile::tempdir().unwrap();
        let runner = write_runner(dir.path(), "printf 'boom' >&2; exit 1");
        let config = config_with_custom(vec![runner.to_str().unwrap()]);

        let result = invoke(dir.path(), &config, &flags_entry("s")).unwrap();
        assert!(matches!(result, CustomRunnerResult::TestFailing(_)));
    }

    #[test]
    fn non_zero_exit_stderr_captured_in_test_failing() {
        let dir = tempfile::tempdir().unwrap();
        let runner = write_runner(dir.path(), "printf 'error details' >&2; exit 2");
        let config = config_with_custom(vec![runner.to_str().unwrap()]);

        let result = invoke(dir.path(), &config, &flags_entry("s")).unwrap();
        if let CustomRunnerResult::TestFailing(r) = result {
            assert_eq!(r.stderr_tail, "error details");
        } else {
            panic!("expected TestFailing");
        }
    }

    #[test]
    fn invalid_envelope_json_on_zero_exit_produces_test_failing() {
        let dir = tempfile::tempdir().unwrap();
        let runner = write_runner(dir.path(), "printf 'not json at all'");
        let config = config_with_custom(vec![runner.to_str().unwrap()]);

        let result = invoke(dir.path(), &config, &flags_entry("s")).unwrap();
        assert!(matches!(result, CustomRunnerResult::TestFailing(_)));
    }

    // 6.5 RED: conflict precedence

    #[test]
    fn process_failure_overrides_valid_passed_envelope() {
        let dir = tempfile::tempdir().unwrap();
        // Runner exits non-zero even though stdout looks like a valid passed envelope
        let data = dir.path().join("envelope.json");
        fs::write(&data, r#"{"exit_code":0,"passed":true,"findings":[]}"#).unwrap();
        let runner = write_runner(dir.path(), &format!("cat {}; exit 1", data.display()));
        let config = config_with_custom(vec![runner.to_str().unwrap()]);

        let result = invoke(dir.path(), &config, &flags_entry("s")).unwrap();
        assert!(
            matches!(result, CustomRunnerResult::TestFailing(_)),
            "non-zero exit must override envelope success"
        );
    }

    #[test]
    fn envelope_failure_overrides_zero_exit() {
        let dir = tempfile::tempdir().unwrap();
        let finding = serde_json::json!({
            "kind": "test-failing",
            "category": "execution",
            "spec": "foo",
            "spec_path": "openspec/specs/foo/spec.md",
            "scenario": {"id": "s", "title": "T", "body_markdown": "B"},
            "suggested_action": "edit_code_not_scenario",
            "playbook_command": "ah explain edit_code_not_scenario"
        });
        let envelope = serde_json::json!({
            "exit_code": 0,
            "passed": false,
            "findings": [finding]
        });
        let runner = write_envelope(dir.path(), &envelope.to_string());
        let config = config_with_custom(vec![runner.to_str().unwrap()]);

        let result = invoke(dir.path(), &config, &flags_entry("s")).unwrap();
        assert!(
            matches!(result, CustomRunnerResult::EnvelopeFindings(_)),
            "envelope findings must be emitted even when process exits 0"
        );
    }

    // 6.7 RED: no custom runner without explicit config

    #[test]
    fn missing_runner_config_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = invoke(dir.path(), &config_no_custom(), &flags_entry("s"));
        assert!(
            result.is_err(),
            "must error when no custom runner is configured"
        );
    }
}
