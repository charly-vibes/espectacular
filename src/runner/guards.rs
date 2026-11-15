//! Zero-tests-ran guard: detect invocations that ran no passing test.
//!
//! Purpose: a runner filter that matches nothing exits 0 while printing
//! `test result: ok. 0 passed; N filtered out`, so an exit-code-only check
//! would count a matched-nothing binding as passed.
//!
//! Responsibilities: recognize cargo-style zero-passed result lines and let
//! any positive block in the same output (multiple test binaries) win.
//!
//! Rationale: derive-contracts-from-specodelic task 4.3 (DDL 4.3 guard);
//! applies to every runner type, not only shells.

use super::TestResult;

/// Zero-tests-ran guard (derive-contracts-from-specodelic task 4.3):
/// detect a runner invocation whose output reports zero passing tests.
/// `cargo test -- <filter>` with a filter matching nothing exits 0 while
/// printing `test result: ok. 0 passed; N filtered out`, so an exit-code-only
/// check would count a matched-nothing binding as passed. Applies to every
/// runner type (previously shell-only); shells that emit cargo-style result
/// lines are covered by the same detection.
pub(crate) fn matched_zero_tests(result: &TestResult) -> bool {
    let has_zero_line = result.stdout_tail.contains("test result: ok. 0 passed")
        || result.stderr_tail.contains("test result: ok. 0 passed");
    if !has_zero_line {
        return false;
    }
    // Check that no "N passed" (N > 0) line appears anywhere in the output.
    // This handles `cargo test` running multiple binaries — the last binary
    // may print "0 passed" even though earlier binaries ran tests successfully.
    !has_positive_passed(&result.stdout_tail) && !has_positive_passed(&result.stderr_tail)
}

fn has_positive_passed(output: &str) -> bool {
    output.lines().any(|line| {
        if let Some(rest) = line.strip_prefix("test result: ok. ") {
            if let Some(num_str) = rest.split(' ').next() {
                if let Ok(n) = num_str.parse::<u64>() {
                    return n > 0;
                }
            }
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::matched_zero_tests;
    use crate::runner::TestResult;

    // Zero-tests-ran guard (derive-contracts-from-specodelic task 4.3):
    // a `flags` binding whose runner output reports zero passing tests
    // (e.g. `cargo test` with a filter matching nothing exits 0 with
    // "0 passed; 1 filtered out") must be detectable so the gate can fail
    // it instead of counting it as passed.

    fn result_with_output(stdout: &str) -> TestResult {
        TestResult {
            test_type: "cargo".to_string(),
            command: "cargo test -- p_token".to_string(),
            exit_code: Some(0),
            timed_out: false,
            stdout_tail: stdout.to_string(),
            stderr_tail: String::new(),
        }
    }

    #[test]
    fn matched_zero_tests_detects_cargo_zero_passed() {
        let result = result_with_output(
            "running 1 test\ntest result: ok. 0 passed; 1 filtered out; 0 finished\n",
        );
        assert!(matched_zero_tests(&result));
    }

    #[test]
    fn matched_zero_tests_false_when_any_binary_reported_passes() {
        // cargo runs one result block per test binary; a trailing zero-passed
        // block must not mask earlier positive blocks.
        let result = result_with_output(
            "test result: ok. 3 passed; 0 failed\ntest result: ok. 0 passed; 0 failed\n",
        );
        assert!(!matched_zero_tests(&result));
    }

    #[test]
    fn matched_zero_tests_false_for_positive_output() {
        let result = result_with_output("test result: ok. 2 passed; 0 failed\n");
        assert!(!matched_zero_tests(&result));
    }

    #[test]
    fn matched_zero_tests_false_for_unrelated_output() {
        let result = result_with_output("all targets up to date\n");
        assert!(!matched_zero_tests(&result));
    }

    #[test]
    fn matched_zero_tests_checks_stderr_tail_too() {
        let mut result = result_with_output("");
        result.stderr_tail = "test result: ok. 0 passed; 1 filtered out\n".to_string();
        assert!(matched_zero_tests(&result));
    }
}
