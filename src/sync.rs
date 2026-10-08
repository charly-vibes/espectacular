//! `ah sync` — create or refresh property-derived contracts (task group 3 of
//! derive-contracts-from-specodelic).
//!
//! Purpose: bridge the specodelic half (Properties rows, read via
//! `spk parse --json`) and the gate (contract TOMLs). Refuses to derive from
//! a file whose `spk lint` is not clean or whose `spk` binary is missing
//! (C-sync-refusal). `--check` writes nothing and reports drift/missing
//! contracts for CI (C-sync-check).
//!
//! Field ownership (design D3): sync owns `id`, `description`, `archetype`,
//! `falsifiability_class`, `derived_from`; humans own `tests`, `status`,
//! `superseded_by` (and `authored_with` provenance, preserved on refresh).

use crate::derive::{canonical_hash, infer, parse_ir, ParseData};
use crate::lint::bridge::{invoke as spk_lint, Failure};
use crate::lint::walker;
use anyhow::anyhow;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Result of a sync run; the CLI layer maps it to an exit code.
#[derive(Debug, Default, PartialEq, serde::Serialize)]
pub(crate) struct SyncOutcome {
    /// Contract paths created or refreshed (write mode only).
    pub wrote: Vec<String>,
    /// Contracts that already matched their source row (--check only).
    pub fresh: Vec<String>,
    /// Properties with no contract file (--check only).
    pub missing: Vec<String>,
    /// Contracts whose derived fields drifted from the source row (--check).
    pub stale: Vec<String>,
    /// Refusals: (spec file, reason) — lint-dirty or parse-broken files.
    pub refusals: Vec<(String, String)>,
    /// Set when the spk binary is missing (C-sync-refusal, spk-unavailable).
    pub spk_unavailable: Option<String>,
}

impl SyncOutcome {
    pub(crate) fn is_ok(&self) -> bool {
        self.spk_unavailable.is_none()
            && self.refusals.is_empty()
            && self.missing.is_empty()
            && self.stale.is_empty()
    }
}

/// Lowercase and hyphenate a property id for use as a contract id.
pub(crate) fn slugify_property_id(id: &str) -> String {
    let slug: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let mut out = String::new();
    for c in slug.chars() {
        if c == '-' && out.ends_with('-') {
            continue;
        }
        out.push(c);
    }
    out.trim_matches('-').to_ascii_lowercase()
}

/// Load a contract TOML as a raw table without schema validation — sync must
/// be able to read partially-staged contracts (e.g. no test bindings yet).
fn load_table(path: &Path) -> Option<toml::Table> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.parse().ok())
}

/// The derived-field fingerprint of a contract table: all five sync-owned
/// fields (D3). Any hand-edit of these makes the contract stale under
/// `--check` (Open question 3 resolution).
fn derived_fingerprint(table: &toml::Table) -> [String; 5] {
    let get = |k: &str| {
        table
            .get(k)
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    [
        get("id"),
        get("description"),
        get("archetype"),
        get("falsifiability_class"),
        get("derived_from"),
    ]
}

/// Run sync over the deployed specs scope. `check` = CI mode (no writes).
pub(crate) fn run_sync(spk: &str, repo_root: &Path, check: bool) -> anyhow::Result<SyncOutcome> {
    let cfg = crate::config::load(repo_root)?;
    let specs_dir = repo_root.join(&cfg.paths.specs);
    let contracts_dir = repo_root.join(".espectacular");
    let mut outcome = SyncOutcome::default();

    // C-sync-refusal: lint every dual-format target first; derive from
    // lint-clean specs only. Missing spk = spk-unavailable, no writes.
    let (specs, _errors) = walker::walk_specs_with_errors(&specs_dir)?;
    let mut parsed: Vec<(String, ParseData)> = Vec::new();
    for spec in &specs {
        if !crate::lint::bridge::is_dual_format(&spec.raw) {
            continue;
        }
        let Some(path) = spec.source_path.to_str() else {
            continue;
        };
        match spk_lint(spk, path) {
            Err(Failure::Unavailable(msg)) => {
                outcome.spk_unavailable = Some(msg);
                return Ok(outcome);
            }
            Err(Failure::Broken(msg)) => {
                outcome
                    .refusals
                    .push((spec.spec_path.clone(), format!("spk lint failed: {msg}")));
                continue;
            }
            Ok(issues) if !issues.is_empty() => {
                let detail = issues
                    .iter()
                    .map(|i| i.rule_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                outcome.refusals.push((
                    spec.spec_path.clone(),
                    format!("spk lint is not clean ({detail})"),
                ));
                continue;
            }
            Ok(_) => {
                let output = Command::new(spk)
                    .args(["parse", path, "--json"])
                    .output()
                    .map_err(|e| anyhow!("failed to invoke spk parse: {e}"))?
                    .stdout;
                let ir = parse_ir(&String::from_utf8_lossy(&output))
                    .map_err(|e| anyhow!("spk parse failed for {}: {e}", spec.spec_path))?;
                parsed.push((spec.spec_path.clone(), ir));
            }
        }
    }

    if !outcome.refusals.is_empty() {
        return Ok(outcome);
    }

    for (spec_path, ir) in &parsed {
        for property in &ir.properties {
            let contract_id = slugify_property_id(&property.id);
            let path = contracts_dir
                .join(spec_path)
                .join(format!("{contract_id}.toml"));
            let path_str = path.to_string_lossy().into_owned();
            let inferred = infer(ir, property);
            let hash = canonical_hash(property);

            let existing = load_table(&path);
            let mut table = existing.clone().unwrap_or_default();
            let is_new = existing.is_none();
            table.insert("id".into(), contract_id.clone().into());
            table.insert(
                "description".into(),
                property
                    .cells
                    .get("predicate")
                    .cloned()
                    .unwrap_or_default()
                    .into(),
            );
            table.insert("archetype".into(), inferred.archetype.into());
            table.insert(
                "falsifiability_class".into(),
                inferred.falsifiability_class.into(),
            );
            table.insert("derived_from".into(), hash.into());
            if is_new {
                table.insert("status".into(), "active".into());
                table.insert("superseded_by".into(), "".into());
                table.insert("authored_with".into(), env!("CARGO_PKG_VERSION").into());
            }

            if check {
                match &existing {
                    None => outcome.missing.push(path_str),
                    Some(old) => {
                        if derived_fingerprint(old) != derived_fingerprint(&table) {
                            outcome.stale.push(path_str);
                        } else {
                            outcome.fresh.push(path_str);
                        }
                    }
                }
            } else {
                let rendered = toml::to_string_pretty(&table)?;
                fs::create_dir_all(path.parent().expect("contract parent"))?;
                fs::write(&path, rendered)?;
                outcome.wrote.push(path_str);
            }
        }
    }

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
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
        use assert_cmd::Command as AssertCommand;
        AssertCommand::cargo_bin("ah")
            .unwrap()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicates::str::contains("sync"));
        AssertCommand::cargo_bin("ah")
            .unwrap()
            .args(["completions", "bash"])
            .assert()
            .success()
            .stdout(predicates::str::contains("sync"));
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
}
