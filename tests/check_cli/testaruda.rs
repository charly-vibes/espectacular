use assert_cmd::Command;
use serde_json::Value;
use std::fs;

use crate::data_from_envelope;
use crate::schema::assert_schema_valid;
use crate::write_executable;

#[test]
fn p_testaruda_l2() {
    ah_check_testaruda_refinement_prunes_unaffected_bindings();
    ah_check_testaruda_empty_selection_runs_everything();
}

// ===== Layer 2: testaruda-store refinement (espectacular-0k8) =====

fn testaruda_available() -> bool {
    Command::new("testaruda")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One capability (compiler) with two unit contracts whose flags are
/// distinctive test names, plus a seeded testaruda store binding
/// src/parser.rs -> parses_input and src/other.rs -> other_thing.
/// The store has passing run history for both tests so neither lands in
/// the always_run fallback (SAFE-007) and selection stays precise.
fn testaruda_l2_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(
        repo.join("openspec/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n\n#### Scenario: Shell path\n- **WHEN** shell command runs\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/green-path.toml"),
        "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"src::parser::tests::parses_input\"\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/compiler/shell-path.toml"),
        "id = \"shell-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"src::other::tests::other_thing\"\n",
    )
    .unwrap();
    write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");
    fs::write(repo.join("src/parser.rs"), "pub fn p() {}\n").unwrap();
    fs::write(repo.join("src/other.rs"), "pub fn o() {}\n").unwrap();

    // Seed the testaruda store: init, import a deterministic graph, then
    // refresh fingerprints from disk so a later in-place edit registers
    // as a change.
    let graph = serde_json::json!({
        "format": "testaruda-graph-v1",
        "content_units": [
            {"id": 1, "kind": "source", "path": "src/parser.rs", "symbol": null, "fingerprint": "unknown", "component": "default"},
            {"id": 2, "kind": "source", "path": "src/other.rs", "symbol": null, "fingerprint": "unknown", "component": "default"}
        ],
        "test_items": [
            {"id": 1, "node_id": "src::parser::tests::parses_input(Test)", "adapter": "rust-adapter", "component": "default", "quarantined": false},
            {"id": 2, "node_id": "src::other::tests::other_thing(Test)", "adapter": "rust-adapter", "component": "default", "quarantined": false}
        ],
        "edges": [
            {"from": 1, "to": 1, "from_node_id": "src::parser::tests::parses_input(Test)", "to_path": "src/parser.rs", "origin": "static", "k": 1000000, "environment": "default"},
            {"from": 2, "to": 2, "from_node_id": "src::other::tests::other_thing(Test)", "to_path": "src/other.rs", "origin": "static", "k": 1000000, "environment": "default"}
        ],
        "run_history": [
            {"run_id": "r1", "node_id": "src::parser::tests::parses_input(Test)", "test_item_id": 1, "outcome": "passed", "environment": "default", "duration_ms": 10},
            {"run_id": "r2", "node_id": "src::other::tests::other_thing(Test)", "test_item_id": 2, "outcome": "passed", "environment": "default", "duration_ms": 10}
        ]
    });
    fs::write(
        repo.join("graph.json"),
        serde_json::to_string(&graph).unwrap(),
    )
    .unwrap();
    // testaruda resolves the project root by walking up for .git — the git
    // repo must exist before any testaruda step, otherwise the store lands
    // in a parent directory and `select` fails with "not initialized".
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
            "init",
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
    // Seed the testaruda store: init, import a deterministic graph, then
    // refresh fingerprints from disk so a later in-place edit registers
    // as a change.
    for args in [
        vec!["init", "-q"],
        vec!["import", "graph.json", "-q"],
        vec!["fingerprint", "-q"],
    ] {
        let out = Command::new("testaruda")
            .args(&args)
            .current_dir(repo)
            .output()
            .expect("testaruda seed step failed");
        assert!(
            out.status.success(),
            "testaruda {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fs::remove_file(repo.join("graph.json")).unwrap();
    dir
}

/// Layer 2 red: seeded store + changed code file -> ah prunes the contract
/// binding whose underlying test testaruda did not select, and reports the
/// refinement in the JSON selection report.
#[test]
fn ah_check_testaruda_refinement_prunes_unaffected_bindings() {
    if !testaruda_available() {
        eprintln!("skipping: testaruda binary not on PATH");
        return;
    }
    let dir = testaruda_l2_repo();
    let repo = dir.path();
    fs::write(repo.join("src/parser.rs"), "pub fn p() { /* v2 */ }\n").unwrap();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--files", "src/parser.rs", "--json"])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let data = data_from_envelope(&output);
    assert_eq!(
        data["selection"]["testaruda"]["pruned"], 1,
        "other_thing binding must be pruned by testaruda refinement"
    );
    assert_eq!(
        data["summary"]["passed"], 1,
        "only the parses_input contract ran"
    );
}

/// Layer 2 exit-20 semantics: EMPTY testaruda selection means "no code
/// affected" — skip the refinement only; the Layer 1 set (here a
/// conservative bypassed run-all for a code-only change) still runs.
#[test]
fn ah_check_testaruda_empty_selection_runs_everything() {
    if !testaruda_available() {
        eprintln!("skipping: testaruda binary not on PATH");
        return;
    }
    let dir = testaruda_l2_repo();
    let repo = dir.path();
    fs::write(repo.join("README.md"), "# docs\n").unwrap();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(repo)
        .args(["check", "--run-tests", "--files", "README.md", "--json"])
        .assert()
        .success();
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_schema_valid(&output);
    let data = data_from_envelope(&output);
    assert_eq!(
        data["selection"]["testaruda"]["pruned"], 0,
        "EMPTY selection must not prune anything"
    );
    assert_eq!(
        data["summary"]["passed"], 2,
        "conservative: both contracts ran"
    );
}
