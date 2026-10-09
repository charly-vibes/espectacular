//! Integration tests for the `ah sync` subcommand --json envelope semantics.
//!
//! Purpose: pin that `sync --check --json` emits `ok` agreeing with the
//! process exit code (espectacular-yr9, evallerina-54f drift class).
//! Responsibilities: fixture repo with a lint-clean dual-format spec and no
//! derived contracts (deterministic `--check` failure regardless of whether
//! spk is installed: missing contracts, or spk-unavailable — both exit 1).
//! Rationale: yr9's Meter — RED captures the ok:true+exit-1 pair; GREEN
//! flips ok to false so (ok, exit_code) is classifiable.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;

/// Deployed, rev-18 lint-clean dual-format spec — guarantees the sync path
/// reaches contract derivation whenever spk is available.
const DUAL_SPEC: &str = include_str!("../openspec/specs/spec-authoring/spec.md");

fn sync_check_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs/spec-authoring")).unwrap();
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
        root.join("openspec/specs/spec-authoring/spec.md"),
        DUAL_SPEC,
    )
    .unwrap();
    dir
}

#[test]
fn ah_sync_check_json_failure_flips_ok_false_with_exit_one() {
    // espectacular-yr9: `sync --check` with a missing derived contract exits
    // 1 while the envelope carried ok:true — the same unclassifiable
    // (ok, exit_code) pair fixed for `check` in evallerina-54f. With spk
    // present the failure is missing-contracts; without spk it is
    // spk-unavailable; both must exit 1 with ok:false.
    let repo = sync_check_repo();
    let out = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo.path())
        .args(["sync", "--check", "--json"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "failing --check must exit 1, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["ok"],
        Value::Bool(false),
        "ok must agree with exit code 1: {}",
        v
    );
    // The data payload still carries the outcome for machine consumers.
    let missing = v["data"]["missing"].as_array().unwrap();
    assert!(
        !missing.is_empty(),
        "expected missing-contract entries: {}",
        v
    );
}
