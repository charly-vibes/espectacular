use crate::config::Config;
use crate::contracts::TestEntry;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_SECONDS: u64 = 60;
const OUTPUT_TAIL_BYTES: usize = 8 * 1024;

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

pub fn compose_command(
    config: &Config,
    test_type: &str,
    entry: &TestEntry,
) -> anyhow::Result<PlannedCommand> {
    let timeout_seconds = entry.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS);
    anyhow::ensure!(timeout_seconds > 0, "timeout_seconds must be positive");

    if test_type == "shell" {
        let command = entry
            .command
            .as_ref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("shell test entry must declare command"))?;
        return Ok(PlannedCommand {
            test_type: test_type.to_string(),
            argv: vec!["/bin/sh".to_string(), "-c".to_string(), command.clone()],
            display: format!("/bin/sh -c {}", shell_escape(command)),
            timeout_seconds,
        });
    }

    let flags = entry
        .flags
        .as_ref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("non-shell test entry must declare flags"))?;
    let runner = config
        .runners
        .get(test_type)
        .ok_or_else(|| anyhow::anyhow!("missing runner for {test_type}"))?;

    let mut argv = runner.clone();
    argv.push(flags.clone());

    Ok(PlannedCommand {
        test_type: test_type.to_string(),
        display: argv.join(" "),
        argv,
        timeout_seconds,
    })
}

pub fn execute_command(repo_root: &Path, planned: &PlannedCommand) -> anyhow::Result<TestResult> {
    let mut command = Command::new(&planned.argv[0]);
    command
        .args(&planned.argv[1..])
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;
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

    Ok(TestResult {
        test_type: planned.test_type.clone(),
        command: planned.display.clone(),
        exit_code: output.status.code(),
        timed_out,
        stdout_tail: tail_string(&output.stdout),
        stderr_tail: tail_string(&output.stderr),
    })
}

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

fn tail_string(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(OUTPUT_TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn shell_escape(command: &str) -> String {
    format!("'{}'", command.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Paths};
    use crate::contracts::TestEntry;
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn config_with_runner(test_type: &str, runner: Vec<&str>) -> Config {
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
        }
    }

    fn write_helper(repo_root: &Path, body: &str) -> std::path::PathBuf {
        let path = repo_root.join("helper.sh");
        fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    #[test]
    fn composes_non_shell_runner_with_flags_as_one_token() {
        let config = config_with_runner("unit", vec!["/bin/sh", "runner.sh", "out.txt"]);
        let entry = TestEntry {
            flags: Some("tests/foo::bar baz".to_string()),
            command: None,
            timeout_seconds: Some(7),
        };

        let planned = compose_command(&config, "unit", &entry).unwrap();
        assert_eq!(planned.argv[3], "tests/foo::bar baz");
        assert_eq!(planned.timeout_seconds, 7);
    }

    #[test]
    fn executes_shell_tests_via_bin_sh_c() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("shell.txt");
        let config = config_with_runner("unit", vec!["ignored"]);
        let entry = TestEntry {
            flags: None,
            command: Some(format!("printf shell > {}", output.display())),
            timeout_seconds: None,
        };

        let planned = compose_command(&config, "shell", &entry).unwrap();
        let result = execute_command(dir.path(), &planned).unwrap();

        assert_eq!(result.exit_code, Some(0));
        assert_eq!(fs::read_to_string(output).unwrap(), "shell");
        assert_eq!(planned.argv[..2], ["/bin/sh", "-c"]);
    }

    #[test]
    fn captures_non_zero_exit_and_output_tails() {
        let dir = tempfile::tempdir().unwrap();
        let helper = write_helper(dir.path(), "printf 'ok'\nprintf 'bad' >&2\nexit 9");
        let config = config_with_runner("unit", vec!["/bin/sh", helper.to_str().unwrap()]);
        let entry = TestEntry {
            flags: Some("flag".to_string()),
            command: None,
            timeout_seconds: None,
        };

        let planned = compose_command(&config, "unit", &entry).unwrap();
        let result = execute_command(dir.path(), &planned).unwrap();

        assert_eq!(result.exit_code, Some(9));
        assert!(!result.timed_out);
        assert_eq!(result.stdout_tail, "ok");
        assert_eq!(result.stderr_tail, "bad");
    }

    #[test]
    fn timeouts_kill_long_running_commands() {
        let dir = tempfile::tempdir().unwrap();
        let helper = write_helper(dir.path(), "sleep 2");
        let config = config_with_runner("unit", vec!["/bin/sh", helper.to_str().unwrap()]);
        let entry = TestEntry {
            flags: Some("flag".to_string()),
            command: None,
            timeout_seconds: Some(1),
        };

        let planned = compose_command(&config, "unit", &entry).unwrap();
        let result = execute_command(dir.path(), &planned).unwrap();

        assert!(result.timed_out);
    }

    #[test]
    fn commands_run_from_repo_root_with_inherited_environment() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("env.txt");
        let helper = write_helper(
            dir.path(),
            &format!(
                "pwd > {}\nprintf '\n%s' \"$SPECIAL_VALUE\" >> {}",
                out.display(),
                out.display()
            ),
        );
        let config = config_with_runner("unit", vec!["/bin/sh", helper.to_str().unwrap()]);
        let entry = TestEntry {
            flags: Some("flag".to_string()),
            command: None,
            timeout_seconds: None,
        };

        std::env::set_var("SPECIAL_VALUE", "present");
        let planned = compose_command(&config, "unit", &entry).unwrap();
        let result = execute_command(dir.path(), &planned).unwrap();
        std::env::remove_var("SPECIAL_VALUE");

        assert_eq!(result.exit_code, Some(0));
        let captured = fs::read_to_string(out).unwrap();
        assert!(captured.contains(&dir.path().display().to_string()));
        assert!(captured.contains("present"));
    }

    #[test]
    fn declared_tests_run_sequentially_in_sorted_test_type_order() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("order.txt");
        let helper = write_helper(
            dir.path(),
            &format!("printf '%s\n' \"$1\" >> {}", out.display()),
        );
        let config = Config {
            tool_version: "0.1.0".to_string(),
            paths: Paths {
                specs: "openspec/specs".to_string(),
                changes: "openspec/changes".to_string(),
            },
            runners: HashMap::from([
                (
                    "pbt".to_string(),
                    vec!["/bin/sh".to_string(), helper.to_str().unwrap().to_string()],
                ),
                (
                    "unit".to_string(),
                    vec!["/bin/sh".to_string(), helper.to_str().unwrap().to_string()],
                ),
            ]),
            quality: Default::default(),
            capabilities: Default::default(),
        };
        let tests = HashMap::from([
            (
                "unit".to_string(),
                vec![
                    TestEntry {
                        flags: Some("u1".to_string()),
                        command: None,
                        timeout_seconds: None,
                    },
                    TestEntry {
                        flags: Some("u2".to_string()),
                        command: None,
                        timeout_seconds: None,
                    },
                ],
            ),
            (
                "pbt".to_string(),
                vec![TestEntry {
                    flags: Some("p1".to_string()),
                    command: None,
                    timeout_seconds: None,
                }],
            ),
        ]);

        let results = run_declared_tests(dir.path(), &config, &tests).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(fs::read_to_string(out).unwrap(), "p1\nu1\nu2\n");
    }
}
