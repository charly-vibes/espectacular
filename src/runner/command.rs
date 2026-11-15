//! Command composition: bindings → `PlannedCommand` argv/display/timeout.
//!
//! Purpose: one place that knows how each binding type becomes an argv, so
//! the shell form (`/bin/sh -c '<command>'`) and the configured-runner form
//! stay distinguishable in findings.
//!
//! Responsibilities: validate the positive timeout, then dispatch to the
//! shell or runner form.
//!
//! Rationale: flags stay ONE argv token (a binding may carry a whole test
//! filter), which composition — not the runner adapters — guarantees.

use super::{PlannedCommand, DEFAULT_TIMEOUT_SECONDS};
use crate::config::Config;
use crate::contracts::TestEntry;

pub fn compose_command(
    config: &Config,
    test_type: &str,
    entry: &TestEntry,
) -> anyhow::Result<PlannedCommand> {
    let timeout_seconds = entry.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS);
    anyhow::ensure!(timeout_seconds > 0, "timeout_seconds must be positive");

    if test_type == "shell" {
        return shell_command(entry, timeout_seconds);
    }
    runner_command(config, test_type, entry, timeout_seconds)
}

/// The shell binding form: the declared command as one `/bin/sh -c` argument.
fn shell_command(entry: &TestEntry, timeout_seconds: u64) -> anyhow::Result<PlannedCommand> {
    let command = entry
        .command
        .as_ref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("shell test entry must declare command"))?;
    Ok(PlannedCommand {
        test_type: "shell".to_string(),
        argv: vec!["/bin/sh".to_string(), "-c".to_string(), command.clone()],
        display: format!("/bin/sh -c {}", shell_escape(command)),
        timeout_seconds,
    })
}

/// The runner binding form: the configured argv plus the flags as one token.
fn runner_command(
    config: &Config,
    test_type: &str,
    entry: &TestEntry,
    timeout_seconds: u64,
) -> anyhow::Result<PlannedCommand> {
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

fn shell_escape(command: &str) -> String {
    format!("'{}'", command.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::compose_command;
    use crate::contracts::TestEntry;
    use crate::runner::execute_command;
    use crate::runner::test_support::{config_with_runner, write_helper};
    use std::fs;

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
    fn p_execution() {
        composes_non_shell_runner_with_flags_as_one_token();
        executes_shell_tests_via_bin_sh_c();
        captures_non_zero_exit_and_output_tails();
        timeouts_kill_long_running_commands();
    }
}
