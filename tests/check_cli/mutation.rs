use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::data_from_envelope;
use crate::git_commit_all;
use crate::schema::assert_schema_valid;

// 8.7/8.8: quality findings do not cause non-zero exit

fn make_mutation_repo() -> (tempfile::TempDir, tempfile::TempDir) {
    let runner_dir = tempfile::tempdir().unwrap();
    let script = runner_dir.path().join("mutation-runner.sh");
    fs::write(&script, "#!/bin/sh\nprintf '{\"kill_rate\": 0.50}'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    fs::write(
        root.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{version}\"\n\
             [paths]\n\
             specs = \"openspec/specs\"\n\
             changes = \"openspec/changes\"\n\
             [runners]\n\
             [quality.mutation]\n\
             enabled = true\n\
             threshold = 0.80\n\
             command = [\"{script}\"]\n",
            version = env!("CARGO_PKG_VERSION"),
            script = script.to_string_lossy()
        ),
    )
    .unwrap();
    git_commit_all(root);
    (dir, runner_dir)
}

#[test]
fn ah_check_quality_mutation_finding_exits_zero() {
    let (repo, _runner) = make_mutation_repo();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();
}

#[test]
fn ah_check_quality_mutation_finding_present_in_output() {
    let (repo, _runner) = make_mutation_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "--run-tests", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    // Schema must validate even when quality_findings are present
    assert_schema_valid(&output);
    let qf = &data["quality_findings"];
    assert!(qf.is_array(), "quality_findings must be an array");
    assert_eq!(qf.as_array().unwrap().len(), 1);
    assert_eq!(qf[0]["kind"], "quality-mutation");
    assert_eq!(qf[0]["category"], "quality");
    assert!(qf[0]["kill_rate"].as_f64().is_some());
}

#[test]
fn ah_check_mutation_skipped_in_precommit_scope() {
    let (repo, _runner) = make_mutation_repo();
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .env("AH_SCOPE", "pre-commit")
        .args(["check", "--json"])
        .assert()
        .success();

    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let data = data_from_envelope(&output);
    assert!(
        output.get("quality_findings").is_none()
            || data["quality_findings"].as_array().unwrap().is_empty(),
        "quality findings must be empty in pre-commit scope"
    );
}
