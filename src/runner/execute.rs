//! Execution: spawn a planned command, supervise its deadline, capture output.
//!
//! Purpose: bounded, deadlock-free capture that both per-binding exit-code
//! verdicts and batched structured attribution consume.
//!
//! Responsibilities: process-group isolation, deadline kill of the whole
//! group (grandchildren included), reader threads draining both pipes while
//! the child runs, and 8 KiB findings tails from the full capture that retain
//! every runner result block (the zero-tests guard reasons over all of them,
//! not just the last 8 KiB).
//!
//! Rationale: a runner producing more output than the pipe buffer (~64 KiB)
//! would otherwise block on write() forever — the polling loop does not
//! read — and every oversized invocation would die on timeout instead of
//! completing (batch-runner-spawns task 2.3: batched JSON is expected to
//! reach megabytes).

use super::{PlannedCommand, TestResult};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const OUTPUT_TAIL_BYTES: usize = 8 * 1024;

/// Spawn helper with a bounded retry on `ETXTBSY` (Text file busy). Executing
/// a file that was just written (test shims in tempdirs) can transiently race
/// the kernel's write-fd accounting when the suite's many process-spawning
/// tests run in parallel (observed as os error 26); a short retry resolves it
/// without masking real failures (NotFound still surfaces to callers).
pub(crate) fn spawn_with_text_busy_retry(
    program: &str,
    args: &[&str],
) -> std::io::Result<std::process::Output> {
    const ATTEMPTS: usize = 4;
    let mut last_err = None;
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(25 * attempt as u64));
        }
        match Command::new(program).args(args).output() {
            Ok(output) => return Ok(output),
            Err(e) if e.raw_os_error() == Some(26) => last_err = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last_err.expect("retry loop exhausted with no error"))
}

pub fn execute_command(repo_root: &Path, planned: &PlannedCommand) -> anyhow::Result<TestResult> {
    execute_command_full(repo_root, planned).map(|(result, _)| result)
}

/// Like [`execute_command`] but also returns the full stdout bytes.
/// Batching (batch-runner-spawns) parses structured per-test output from the
/// complete captured stdout; tails stay 8 KiB findings-only.
pub(crate) fn execute_command_full(
    repo_root: &Path,
    planned: &PlannedCommand,
) -> anyhow::Result<(TestResult, Vec<u8>)> {
    let mut child = spawn_piped(repo_root, &planned.argv)?;
    let (stdout_reader, stderr_reader) = drain_pipes(&mut child);
    let timed_out = wait_with_deadline(&mut child, planned.timeout_seconds)?;

    let stdout = stdout_reader.join().expect("stdout reader thread")?;
    let stderr = stderr_reader.join().expect("stderr reader thread")?;
    let status = child.wait()?;

    let result = TestResult {
        test_type: planned.test_type.clone(),
        command: planned.display.clone(),
        exit_code: status.code(),
        timed_out,
        stdout_tail: tail_string(&stdout),
        stderr_tail: tail_string(&stderr),
    };
    Ok((result, stdout))
}

/// Spawn `argv` in `repo_root`: stdin closed, both outputs piped, isolated in
/// its own process group so a deadline kill can reach grandchildren.
fn spawn_piped(repo_root: &Path, argv: &[String]) -> anyhow::Result<Child> {
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        command.process_group(0); // isolate in its own process group
    }
    Ok(command.spawn()?)
}

type Reader = thread::JoinHandle<std::io::Result<Vec<u8>>>;

/// Drain stdout/stderr on reader threads WHILE the caller polls the child, so
/// a runner writing past the pipe buffer cannot deadlock the polling loop.
fn drain_pipes(child: &mut Child) -> (Reader, Reader) {
    use std::io::Read;

    let stdout_pipe = child.stdout.take().expect("stdout was piped");
    let stderr_pipe = child.stderr.take().expect("stderr was piped");
    let stdout_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut pipe = stdout_pipe;
        pipe.read_to_end(&mut buf).map(|_| buf)
    });
    let stderr_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut pipe = stderr_pipe;
        pipe.read_to_end(&mut buf).map(|_| buf)
    });
    (stdout_reader, stderr_reader)
}

/// Poll until the child exits or the deadline fires; on deadline, kill the
/// entire process group so grandchild processes holding inherited pipes also
/// die. Returns whether the deadline fired.
fn wait_with_deadline(child: &mut Child, timeout_seconds: u64) -> anyhow::Result<bool> {
    let started = Instant::now();
    let timeout = Duration::from_secs(timeout_seconds);
    loop {
        if child.try_wait()?.is_some() {
            return Ok(false);
        }
        if started.elapsed() >= timeout {
            // Kill the entire process group so grandchild processes
            // holding inherited pipes also die.
            #[cfg(unix)]
            {
                let pgid = child.id() as i32;
                unsafe { libc::killpg(pgid, libc::SIGTERM) };
            }
            child.kill()?;
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn tail_string(bytes: &[u8]) -> String {
    if bytes.len() <= OUTPUT_TAIL_BYTES {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    // Start the tail on a line boundary so every line below is whole; the
    // tail may exceed the cap by at most one line.
    let mut start = bytes.len() - OUTPUT_TAIL_BYTES;
    while start < bytes.len() && bytes[start] != b'\n' {
        start += 1;
    }
    let start = (start + 1).min(bytes.len());
    let mut kept = String::new();
    // Preserve result blocks from the omitted prefix: the zero-tests guard
    // reasons over every block, and a positive block from an earlier test
    // binary must survive truncation (catherine-j17).
    for line in String::from_utf8_lossy(&bytes[..start]).lines() {
        if line.starts_with("test result:") {
            kept.push_str(line);
            kept.push('\n');
        }
    }
    kept.push_str(&String::from_utf8_lossy(&bytes[start..]));
    kept
}

#[cfg(test)]
mod tests {
    use super::execute_command;
    use crate::contracts::TestEntry;
    use crate::runner::compose_command;
    use crate::runner::matched_zero_tests;
    use crate::runner::test_support::{config_with_runner, write_helper};
    use std::fs;

    #[test]
    fn timeout_kills_process_group_children() {
        let dir = tempfile::tempdir().unwrap();
        // Child runs a long command AND backgrounds a grandchild that holds
        // stdout open. Without process-group kill, child.kill() kills only
        // the direct child but wait_with_output blocks until the grandchild
        // closes the inherited pipe.
        let helper = write_helper(dir.path(), "(sleep 20) &\nsleep 10");
        let config = config_with_runner("unit", vec!["/bin/sh", helper.to_str().unwrap()]);
        let entry = TestEntry {
            flags: Some("flag".to_string()),
            command: None,
            timeout_seconds: Some(1),
        };

        let planned = compose_command(&config, "unit", &entry).unwrap();
        let started = std::time::Instant::now();
        let result = execute_command(dir.path(), &planned).unwrap();
        let elapsed = started.elapsed();

        assert!(result.timed_out, "expected timed_out=true");
        // Must return near the 1s deadline, not block for 10s
        assert!(
            elapsed < std::time::Duration::from_secs(3),
            "took {elapsed:?} — grandchild process not killed"
        );
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
    fn truncated_tail_keeps_earlier_result_blocks() {
        // A multi-crate `cargo test <filter>` run: the matching binary
        // reports one pass first, then thousands of bytes of "0 passed"
        // blocks from the remaining binaries push it past the 8 KiB tail.
        // Losing it makes the zero-tests guard flag a run that did execute
        // the selected test (catherine-j17).
        let dir = tempfile::tempdir().unwrap();
        let helper = write_helper(
            dir.path(),
            "printf 'test result: ok. 1 passed; 0 failed; 0 ignored\n'\n\
             i=0\n\
             while [ $i -lt 400 ]; do\n\
             \tprintf 'padding line for tail truncation padding padding padding\n'\n\
             \ti=$((i+1))\n\
             done\n\
             printf 'test result: ok. 0 passed; 0 failed; 1 filtered out\n'",
        );
        let config = config_with_runner("unit", vec!["/bin/sh", helper.to_str().unwrap()]);
        let entry = TestEntry {
            flags: Some("flag".to_string()),
            command: None,
            timeout_seconds: None,
        };

        let planned = compose_command(&config, "unit", &entry).unwrap();
        let result = execute_command(dir.path(), &planned).unwrap();

        assert!(
            result.stdout_tail.contains("test result: ok. 1 passed"),
            "earlier result block lost to truncation: {}",
            result.stdout_tail
        );
        assert!(
            !matched_zero_tests(&result),
            "zero-tests guard misfired on a run that executed a test"
        );
    }
}
