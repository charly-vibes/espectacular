//! Tests for `ah sync` (task groups 3.1–3.4 and the rev-18 id rule).
//!
//! Purpose: exercise the sync run end-to-end against fake `spk` shims — the
//! refuse-on-dirty/broken/unavailable rules, create-or-refresh field
//! ownership, and `--check` drift reporting.
//!
//! Rationale: the shims make every refusal path deterministic (the real
//! `spk` e2e case skips when the binary is absent).

use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// ---- fixture helpers ------------------------------------------------

const CONFIG_TOML: &str = "tool_version = \"0.9.0\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\npytest = [\"pytest\"]\ncargo = [\"cargo\", \"test\"]\n";

// Rev-18 naming law: the id derives from the parent directory (auth/ →
// `auth`), so the real-spk e2e lint stays clean.
const SPEC_MD: &str = "---\nid: auth\nkind: intent\nstatement: \"WHEN auth is exercised THE system SHALL reject bad tokens\"\n---\n\n## Constraints\n\n| id | kind | expr | traces_to |\n|----|------|------|-----------|\n| C-token | invariant | invalid tokens are rejected | [[auth]] |\n\n## Model\n\n### States\n\n- `checking`\n\n### Transitions\n\n| id | from | to | guard |\n|----|------|----|-------|\n| t-reject | checking | checking | [[auth.C-token]] |\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-token | unit | [[auth.C-token]] | valid vs invalid tokens | invalid tokens rejected with 401 |\n\n## Requirements\n\n### Requirement: Token check\n\nThe system SHALL reject invalid tokens.\n";

/// A real (non-`spec`) id under the rev-18 parent-dir naming law — the
/// sync gate must route it to spk exactly like the legacy `id: spec`.
const SPEC_MD_REAL_ID: &str = "---\nid: auth.tokens\nkind: intent\nstatement: \"WHEN auth is exercised THE system SHALL reject bad tokens\"\n---\n\n## Constraints\n\n| id | kind | expr | traces_to |\n|----|------|------|-----------|\n| C-token | invariant | invalid tokens are rejected | [[auth.tokens]] |\n\n## Model\n\n### States\n\n- `checking`\n\n### Transitions\n\n| id | from | to | guard |\n|----|------|----|-------|\n| t-reject | checking | checking | [[auth.tokens.C-token]] |\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-token | unit | [[auth.tokens.C-token]] | valid vs invalid tokens | invalid tokens rejected with 401 |\n\n## Requirements\n\n### Requirement: Token check\n\nThe system SHALL reject invalid tokens.\n";

fn write_executable(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// A fake spk whose `lint` reports clean and whose `parse` emits a valid
/// IR envelope for the auth fixture spec.
fn clean_spk_shim(dir: &Path) -> String {
    let parse_output = r#"{"ok":true,"data":{"properties":[{"id":"P-token","kind":"unit","cells":{"id":"P-token","kind":"unit","derives_from":"[[auth.C-token]]","generator":"valid vs invalid tokens","predicate":"invalid tokens rejected with 401"}}],"constraints":[{"id":"C-token","kind":"invariant","cells":{}}],"states":[],"transitions":[]}}"#;
    let body = format!(
        "#!/bin/sh\nif [ \"$1\" = lint ]; then\n  echo '{{\"ok\":true,\"data\":{{\"issues\":[]}}}}'\nelse\n  cat <<'JSON'\n{}\nJSON\nfi\n",
        parse_output
    );
    write_executable(dir, "spk-clean", &body);
    let path = dir.join("spk-clean");
    path.to_string_lossy().into_owned()
}

/// A fake spk whose `lint` reports a finding (lint-dirty spec).
fn dirty_spk_shim(dir: &Path) -> String {
    let body = "#!/bin/sh\nif [ \"$1\" = lint ]; then\n  echo '{\"ok\":true,\"data\":{\"issues\":[{\"rule_id\":\"vague-qualifier\",\"message\":\"unbound qualifier\"}]}}'\nelse\n  echo '{\"ok\":true,\"data\":null}'\nfi\n";
    write_executable(dir, "spk-dirty", body);
    dir.join("spk-dirty").to_string_lossy().into_owned()
}

/// A fake spk whose `lint` fails hard (broken invocation).
fn broken_spk_shim(dir: &Path) -> String {
    let body = "#!/bin/sh\nexit 3\n";
    write_executable(dir, "spk-broken", body);
    dir.join("spk-broken").to_string_lossy().into_owned()
}

fn repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join(".espectacular/auth")).unwrap();
    fs::create_dir_all(root.join("openspec/specs/auth")).unwrap();
    fs::write(root.join(".espectacular/config.toml"), CONFIG_TOML).unwrap();
    fs::write(root.join("openspec/specs/auth/spec.md"), SPEC_MD).unwrap();
    (dir, root)
}

fn contract_path(root: &Path) -> PathBuf {
    root.join(".espectacular/auth/p-token.toml")
}

// ---- 3.3: create-or-refresh -----------------------------------------

#[test]
fn slugify_lowercases_and_hyphenates() {
    assert_eq!(slugify_property_id("P-token"), "p-token");
    assert_eq!(slugify_property_id("P-weird.name/x"), "p-weird-name-x");
    assert_eq!(slugify_property_id("p_shipped"), "p-shipped");
}

#[test]
fn sync_creates_derived_contracts() {
    let (_dir, root) = repo();
    let spk = clean_spk_shim(_dir.path());
    let outcome = run_sync(&spk, &root, false).unwrap();
    assert!(outcome.is_ok(), "unexpected refusals: {outcome:?}");
    assert_eq!(outcome.wrote.len(), 1, "{outcome:?}");
    let toml_text = fs::read_to_string(contract_path(&root)).unwrap();
    assert!(toml_text.contains("id = \"p-token\""), "{toml_text}");
    assert!(toml_text.contains("invalid tokens rejected with 401"));
    assert!(toml_text.contains("archetype = \"PF\""), "{toml_text}");
    assert!(toml_text.contains("falsifiability_class = \"safety\""));
    assert!(
        toml_text.contains("derived_from = \"P-token@"),
        "{toml_text}"
    );
    assert!(toml_text.contains("status = \"active\""));
    assert!(
        !toml_text.contains("[[tests"),
        "fresh contract must not invent test bindings"
    );
}

#[test]
fn sync_refresh_preserves_human_owned_fields() {
    let (_dir, root) = repo();
    let spk = clean_spk_shim(_dir.path());
    fs::write(
        contract_path(&root),
        "id = \"p-token\"\ndescription = \"old\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\nfalsifiability_class = \"safety\"\nderived_from = \"P-token@000000000000\"\n\n[[tests.cargo]]\nflags = \"p_token::case1\"\n",
    )
    .unwrap();
    let outcome = run_sync(&spk, &root, false).unwrap();
    assert_eq!(outcome.wrote.len(), 1, "{outcome:?}");
    let toml_text = fs::read_to_string(contract_path(&root)).unwrap();
    assert!(
        toml_text.contains("invalid tokens rejected with 401"),
        "description refreshed: {toml_text}"
    );
    assert!(
        toml_text.contains("[[tests.cargo]]"),
        "human tests preserved"
    );
    assert!(toml_text.contains("flags = \"p_token::case1\""));
    assert!(
        toml_text.contains("authored_with = \"0.1.0\""),
        "provenance preserved"
    );
    assert!(
        !toml_text.contains("P-token@000000000000"),
        "stale hash replaced"
    );
}

// ---- 3.2: refusal rules ---------------------------------------------

#[test]
fn sync_refuses_lint_dirty_file_without_writing() {
    let (_dir, root) = repo();
    let spk = dirty_spk_shim(_dir.path());
    let outcome = run_sync(&spk, &root, false).unwrap();
    assert!(!outcome.is_ok());
    assert_eq!(outcome.refusals.len(), 1);
    assert!(
        outcome.refusals[0].0.contains("auth"),
        "names the file: {outcome:?}"
    );
    assert!(!contract_path(&root).exists(), "must write nothing");
}

#[test]
fn sync_refuses_broken_spk_invocation() {
    let (_dir, root) = repo();
    let spk = broken_spk_shim(_dir.path());
    let outcome = run_sync(&spk, &root, false).unwrap();
    assert!(!outcome.is_ok());
    assert!(!contract_path(&root).exists());
}

#[test]
fn sync_reports_spk_unavailable() {
    let (_dir, root) = repo();
    let outcome = run_sync("definitely-not-spk-xyz", &root, false).unwrap();
    assert!(outcome.spk_unavailable.is_some(), "{outcome:?}");
    assert!(!contract_path(&root).exists());
}

// ---- 3.4: --check mode ----------------------------------------------

#[test]
fn check_mode_reports_missing_contracts_without_writing() {
    let (_dir, root) = repo();
    let spk = clean_spk_shim(_dir.path());
    let outcome = run_sync(&spk, &root, true).unwrap();
    assert_eq!(outcome.missing.len(), 1, "{outcome:?}");
    assert!(outcome.wrote.is_empty(), "check mode writes nothing");
    assert!(!contract_path(&root).exists());
    assert!(!outcome.is_ok());
}

#[test]
fn check_mode_accepts_fresh_contracts() {
    let (_dir, root) = repo();
    let spk = clean_spk_shim(_dir.path());
    // First a write-mode sync to stage the contract, then check.
    run_sync(&spk, &root, false).unwrap();
    let outcome = run_sync(&spk, &root, true).unwrap();
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(outcome.fresh.len(), 1);
    assert!(outcome.missing.is_empty() && outcome.stale.is_empty());
}

#[test]
fn check_mode_reports_drifted_contracts() {
    let (_dir, root) = repo();
    let spk = clean_spk_shim(_dir.path());
    run_sync(&spk, &root, false).unwrap();
    // Hand-edit a derived field (D3: sync-owned) — the contract is stale.
    let text = fs::read_to_string(contract_path(&root)).unwrap();
    fs::write(contract_path(&root), text.replace("PF", "CE")).unwrap();
    let outcome = run_sync(&spk, &root, true).unwrap();
    assert_eq!(outcome.stale.len(), 1, "{outcome:?}");
    assert!(!outcome.is_ok());
    // And the file was not rewritten in check mode.
    assert!(fs::read_to_string(contract_path(&root))
        .unwrap()
        .contains("CE"));
}

// ---- 3.1: CLI registration (C-sync-registration / P-register) --------

#[test]
fn sync_appears_in_help_and_completions() {
    use clap::CommandFactory;

    // In-process: spawning the built binary needs `CARGO_BIN_EXE_ah`, which
    // cargo does not set for unit tests and coverage runners (tarpaulin) do
    // not set at all.
    let mut cmd = crate::cli::Cli::command();
    let help = cmd.render_long_help().to_string();
    assert!(help.contains("sync"), "sync missing from --help:\n{help}");

    let mut buf: Vec<u8> = Vec::new();
    clap_complete::generate(clap_complete::Shell::Bash, &mut cmd, "ah", &mut buf);
    let script = String::from_utf8(buf).expect("completion script is utf-8");
    assert!(
        script.contains("sync"),
        "sync missing from bash completions"
    );
}

// ---- real-spk end-to-end (skips gracefully without spk) --------------

#[test]
fn real_spk_end_to_end_if_installed() {
    if Command::new("spk").arg("--version").output().is_err() {
        eprintln!("spk not installed; skipping");
        return;
    }
    let (_dir, root) = repo();
    let outcome = run_sync("spk", &root, false).unwrap();
    assert!(outcome.is_ok(), "real spk refused: {outcome:?}");
    assert!(contract_path(&root).exists());
}

// ---- Revision 18: real (non-`spec`) ids must derive contracts too ----

#[test]
fn sync_derives_contracts_for_real_id_dual_format() {
    let (_dir, root) = repo();
    // Replace the legacy-id spec with a real-id one in its own directory.
    fs::remove_dir_all(root.join("openspec/specs/auth")).unwrap();
    fs::create_dir_all(root.join("openspec/specs/auth-tokens")).unwrap();
    fs::write(
        root.join("openspec/specs/auth-tokens/spec.md"),
        SPEC_MD_REAL_ID,
    )
    .unwrap();
    let spk = clean_spk_shim(&root);
    let outcome = run_sync(&spk, &root, false).unwrap();
    assert!(
        outcome.refusals.is_empty(),
        "real-id file must not be refused: {outcome:?}"
    );
    assert_eq!(outcome.wrote.len(), 1, "{outcome:?}");
    assert!(root.join(".espectacular/auth-tokens/p-token.toml").exists());
}
