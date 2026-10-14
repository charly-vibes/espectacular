use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::data_from_envelope;
use crate::schema::assert_schema_valid;
use crate::write_executable;

/// Fixture (espectacular-pta, gate P-range-*): a git repo with two
/// capabilities, each owning two contracts (unit + shell), fully committed
/// so the worktree is clean. Returns the dir and the base commit sha.
fn git_range_repo() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join("openspec/specs/other")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/other")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Shell path\n- **WHEN** shell command runs\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(
        repo.join("openspec/specs/other/spec.md"),
        "# Capability: other\n\n#### Scenario: Other green\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Other shell\n- **WHEN** shell command runs\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        concat!(
            "tool_version = \"",
            env!("CARGO_PKG_VERSION"),
            "\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n"
        ),
    )
    .unwrap();
    for (cap, unit_id, shell_id) in [
        ("compiler", "green-path", "shell-path"),
        ("other", "other-green", "other-shell"),
    ] {
        fs::write(
            repo.join(format!(".espectacular/{cap}/{unit_id}.toml")),
            format!(
                "id = \"{unit_id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n"
            ),
        )
        .unwrap();
        fs::write(
            repo.join(format!(".espectacular/{cap}/{shell_id}.toml")),
            format!(
                "id = \"{shell_id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.shell]]\ncommand = \"printf shell\"\n"
            ),
        )
        .unwrap();
    }
    write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");

    for args in [
        vec!["init"],
        vec!["add", "."],
        vec![
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "base",
        ],
    ] {
        let out = Command::new("git")
            .args(&args)
            .current_dir(repo)
            .output()
            .expect("git seed step failed");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    let base = String::from_utf8_lossy(&sha.stdout).trim().to_string();
    (dir, base)
}

fn git_commit(repo: &Path, message: &str) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            message,
        ])
        .current_dir(repo)
        .output()
        .expect("git commit failed");
    assert!(
        out.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Range selection (espectacular-pta, gate P-range-files / C-range-diff):
/// at pre-push time the worktree is clean, so the pre-change worktree-diff
/// source supplies no signal and every contract runs on every push. With a
/// commit range that touches only one capability's spec, only that
/// capability's contracts run and the untouched capability is skipped in
/// the selection report. The report shape itself is unchanged
/// (C-range-unchanged / design D4), so the source stays "spec-provenance".
#[test]
fn ah_check_range_selection_runs_only_affected_capability() {
    let (dir, base) = git_range_repo();
    let repo = dir.path();

    // Second commit: touch the compiler spec only.
    let spec = repo.join("openspec/specs/compiler/spec.md");
    let original = fs::read_to_string(&spec).unwrap();
    fs::write(&spec, format!("{original}\n<!-- touch -->")).unwrap();
    let out = Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .expect("git add failed");
    assert!(out.status.success());
    git_commit(repo, "touch compiler spec");

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args([
            "check",
            "--run-tests",
            "--base",
            &base,
            "--head",
            "HEAD",
            "--json",
        ])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let data = data_from_envelope(&output);
    let selection = data
        .get("selection")
        .expect("range mode must report a selection");
    assert_eq!(
        selection["source"], "spec-provenance",
        "selection report shape unchanged (C-range-unchanged)"
    );
    assert_eq!(
        selection["selected"],
        serde_json::json!(["compiler"]),
        "only the changed capability is selected; other absent from the report"
    );
    assert_eq!(
        selection["skipped"], 2,
        "untouched capability's scenarios are skipped"
    );
    assert_eq!(
        data["summary"]["passed"], 2,
        "only the affected capability's contracts ran"
    );
}

/// Range precedence (espectacular-pta, gate P-range-precedence /
/// C-range-precedence): the changed-file source order is total —
/// explicit --files wins over --base, --all-tests bypasses selection
/// entirely, and --base without --run-tests is inert (structural gate
/// unchanged, no selection report).
#[test]
fn ah_check_range_precedence_files_all_tests_inert() {
    let (dir, base) = git_range_repo();
    let repo = dir.path();

    // Second commit: touch the compiler spec only — a non-degenerate
    // range that would select compiler if it were consulted.
    let spec = repo.join("openspec/specs/compiler/spec.md");
    let original = fs::read_to_string(&spec).unwrap();
    fs::write(&spec, format!("{original}\n<!-- touch -->")).unwrap();
    let out = Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .expect("git add failed");
    assert!(out.status.success());
    git_commit(repo, "touch compiler spec");

    // --files beats --base: other selected despite the range pointing
    // at compiler.
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args([
            "check",
            "--run-tests",
            "--files",
            "openspec/specs/other/spec.md",
            "--base",
            &base,
            "--json",
        ])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let data = data_from_envelope(&output);
    assert_eq!(
        data["selection"]["selected"],
        serde_json::json!(["other"]),
        "explicit --files wins over --base"
    );

    // --all-tests beats --base: everything runs, bypass reported.
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args([
            "check",
            "--run-tests",
            "--all-tests",
            "--base",
            &base,
            "--json",
        ])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let data = data_from_envelope(&output);
    assert_eq!(
        data["summary"]["passed"], 4,
        "everything ran despite the range"
    );

    // --base without --run-tests is inert: structural gate, no tests,
    // no selection report.
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--base", &base, "--json"])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let data = data_from_envelope(&output);
    assert!(
        data.get("selection").is_none(),
        "range mode must be inert without --run-tests"
    );
    assert_eq!(data["summary"]["passed"], 0, "structural run ran no tests");
}

/// D6 (espectacular-pta): --head without --base is meaningless — clap
/// rejects it at parse (exit 2), matching the invalid-subcommand
/// convention.
#[test]
fn ah_check_head_requires_base() {
    let (dir, _base) = git_range_repo();
    let repo = dir.path();
    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--head", "HEAD"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--base <BASE>"))
        .code(2);
}

/// Degenerate range fallback (espectacular-pta, gate P-range-safe /
/// design D2): an empty range (base == HEAD) and an unresolvable ref both
/// look identical to git — empty diff output — so both must run every
/// declared contract. The gate never exits green with zero tests executed
/// because of a degenerate input it cannot distinguish.
#[test]
fn ah_check_degenerate_range_runs_everything() {
    let (dir, _base) = git_range_repo();
    let repo = dir.path();

    for base_ref in ["HEAD", "not-a-ref"] {
        let assert = Command::cargo_bin("ah")
            .unwrap()
            .current_dir(repo)
            .args([
                "check",
                "--run-tests",
                "--base",
                base_ref,
                "--head",
                "HEAD",
                "--json",
            ])
            .assert()
            .success();
        let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
        assert_schema_valid(&output);
        let data = data_from_envelope(&output);
        assert!(
            data.get("selection").is_none(),
            "degenerate range supplies no selection signal (base={base_ref})"
        );
        assert_eq!(
            data["summary"]["passed"], 4,
            "conservative fallback: every declared contract ran (base={base_ref})"
        );
    }
}
