use assert_cmd::Command;
use std::fs;

use crate::git_commit_all;

fn make_healthy_doctor_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    fs::write(
        root.join(".espectacular/config.toml"),
        concat!(
            "tool_version = \"",
            env!("CARGO_PKG_VERSION"),
            "\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join("AGENTS.md"),
        "# P\n\n<!-- ah:managed:start -->\n<!-- ah:managed:end -->\n",
    )
    .unwrap();
    fs::write(
        root.join("CLAUDE.md"),
        "# P\n\n<!-- ah:managed:start -->\n<!-- ah:managed:end -->\n",
    )
    .unwrap();
    fs::write(
        root.join("lefthook.yml"),
        "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\npre-push:\n  commands:\n    ah-check:\n      run: ah check\n",
    )
    .unwrap();
    git_commit_all(root);
    dir
}

#[test]
fn ah_doctor_healthy_repo_exits_zero() {
    let repo = make_healthy_doctor_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("healthy"));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("healthy"),
        "doctor must report the repo as healthy: {stdout}"
    );
}

#[test]
fn ah_doctor_bad_config_exits_nonzero() {
    let repo = make_healthy_doctor_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        "tool_version = \"\"\n[paths]\nspecs = \"\"\nchanges = \"\"\n[runners]\n",
    )
    .unwrap();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicates::str::contains("config: bad config"));
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("config: bad config"),
        "bad config must be named in the diagnostic: {stderr}"
    );
}
