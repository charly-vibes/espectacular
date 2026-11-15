use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;

fn ah() -> Command {
    Command::cargo_bin("ah").unwrap()
}

#[test]
fn ah_feedback_bug_dry_run_prints_body_and_gh_line() {
    let output = ah()
        .args(["feedback", "bug", "--dry-run"])
        .assert()
        .failure()
        .stderr(contains("DRY RUN"))
        .stderr(contains("gh issue create"))
        .stderr(contains("espectacular"))
        .stderr(contains("tool:"))
        .stderr(contains("ah"))
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).unwrap();
    let gh_line = stderr
        .lines()
        .find(|line| line.contains("gh issue create"))
        .unwrap_or("");
    assert!(
        gh_line.contains("DRY RUN: would run: gh issue create --repo "),
        "the dry run must preview the exact gh command; got:\n{stderr}"
    );
}

#[test]
fn ah_feedback_dry_run_redacts_git_remote() {
    let output = ah()
        .args(["feedback", "bug", "--dry-run"])
        .assert()
        .failure()
        .stderr(contains("git_remote"))
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).unwrap();
    let remote_line = stderr
        .lines()
        .find(|line| line.contains("git_remote"))
        .unwrap_or("");
    assert!(
        !remote_line.is_empty() && !remote_line.contains("://") && !remote_line.contains('@'),
        "git_remote must be reduced to host/path; got: {remote_line:?}"
    );
}

// Regression for espectacular-qyu (GH#28 item 3): feedback must not
// hard-require Cargo.toml — non-Rust git repos (e.g. babashka/Clojure)
// need to file bug reports too. Repo metadata should degrade to the
// default target (charly-vibes/espectacular) instead of erroring.
#[test]
fn ah_feedback_bug_works_in_non_rust_git_repo() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("clojure-project");
    fs::create_dir_all(&repo).unwrap();
    Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .assert()
        .success();
    // deliberately NO Cargo.toml here

    let assert = ah()
        .current_dir(&repo)
        .args(["feedback", "bug", "--dry-run"])
        .assert()
        .failure() // dry-run convention: prints body, exits non-zero
        .stderr(contains("DRY RUN"))
        .stderr(contains("gh issue create"))
        .stderr(contains("tool:"))
        .stderr(contains("ah"))
        .stderr(contains("gh issue create --repo charly-vibes/espectacular"))
        .stderr(contains("cannot read Cargo.toml").not());
    assert_eq!(
        assert.get_output().status.code(),
        Some(1),
        "a non-Rust repo dry run must still exit non-zero"
    );
}

#[test]
fn ah_feedback_from_last_error_prints_error_context() {
    let dir = tempfile::tempdir().unwrap();
    let cache_dir = dir.path().join("ah");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(
        cache_dir.join("errors.jsonl"),
        r#"{"ts":"2026-07-28T00:00:00Z","argv":["ah","check"],"exit":1,"footer":"→ Run: ah doctor","kind":"Fix"}"#,
    )
    .unwrap();

    let output = ah()
        .env("XDG_CACHE_HOME", dir.path())
        .args(["feedback", "bug", "--from-last-error", "--dry-run"])
        .assert()
        .failure()
        .stderr(contains("ah check"))
        .stderr(contains("Exit code"))
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).unwrap();
    assert!(
        stderr.contains("**Exit code:** 1"),
        "the recorded exit code must be rendered; got:\n{stderr}"
    );
}
