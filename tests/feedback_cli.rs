use assert_cmd::Command;
use predicates::str::contains;
use std::fs;

fn ah() -> Command {
    Command::cargo_bin("ah").unwrap()
}

#[test]
fn ah_feedback_bug_dry_run_prints_body_and_gh_line() {
    ah().args(["feedback", "bug", "--dry-run"])
        .assert()
        .failure()
        .stderr(contains("DRY RUN"))
        .stderr(contains("gh issue create"))
        .stderr(contains("espectacular"))
        .stderr(contains("tool:"))
        .stderr(contains("ah"));
}

#[test]
fn ah_feedback_dry_run_redacts_git_remote() {
    ah().args(["feedback", "bug", "--dry-run"])
        .assert()
        .failure()
        .stderr(contains("git_remote"));
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

    ah().env("XDG_CACHE_HOME", dir.path())
        .args(["feedback", "bug", "--from-last-error", "--dry-run"])
        .assert()
        .failure()
        .stderr(contains("ah check"))
        .stderr(contains("Exit code"));
}
