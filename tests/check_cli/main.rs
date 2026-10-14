// Integration tests for `ah check`, `ah doctor`, `ah report` and the
// selection layers (testaruda L2, range selection). Shared repo fixtures
// live here; each submodule owns one responsibility.
use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

mod batch_attribution;
mod batching;
mod check;
mod doctor;
mod e2e;
mod mutation;
mod pytest;
mod range;
mod report;
mod schema;
mod testaruda;

/// Extract the inner data payload from a genesis envelope.
pub fn data_from_envelope(output: &Value) -> &Value {
    &output["data"]
}

/// Make `root` a git repository with a clean initial commit.
///
/// `ah check --run-tests` enumerates worktree changes against HEAD since
/// espectacular-o77; a non-repo directory is now an explicit error, and a
/// dirty tree activates test selection. A clean committed fixture keeps
/// the changed set empty (conservative run-all, pre-migration behavior).
pub fn git_commit_all(root: &std::path::Path) {
    let run = |args: &[&str]| {
        Command::new("git")
            .args(["-c", "user.email=test@example.com", "-c", "user.name=test"])
            .args(args)
            .current_dir(root)
            .assert()
            .success();
    };
    run(&["init"]);
    run(&["add", "."]);
    run(&["commit", "--no-gpg-sign", "-m", "fixture"]);
}

pub fn write_executable(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

pub fn base_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Shell path\n- **WHEN** shell command runs\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n"),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/green-path.toml"),
        "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/shell-path.toml"),
        "id = \"shell-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.shell]]\ncommand = \"printf shell\"\n",
    )
    .unwrap();
    write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");
    git_commit_all(repo);
    dir
}
