use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
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

    ah().current_dir(&repo)
        .args(["feedback", "bug", "--dry-run"])
        .assert()
        .failure() // dry-run convention: prints body, exits non-zero
        .stderr(contains("DRY RUN"))
        .stderr(contains("gh issue create"))
        .stderr(contains("tool:"))
        .stderr(contains("ah"))
        .stderr(contains("espectacular")) // falls back to default target repo
        .stderr(contains("cannot read Cargo.toml").not());
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
