//! The `ah sync` run pipeline: lint-gated spec collection, table derivation,
//! and check/write staging.
//!
//! Purpose: implement one sync pass over the deployed specs scope without
//! touching the top-level result shape (see the parent module).
//!
//! Responsibilities:
//! - collect dual-format specs whose `spk lint` is clean (or refuse them)
//! - derive the sync-owned contract fields and merge them over the existing
//!   table so human-owned fields survive a refresh
//! - either report drift (`check`) or write the contract TOML
//!
//! Rationale: the run is split into one stage per design decision (D3 field
//! ownership, C-sync-refusal, C-sync-check) so each stage is independently
//! reviewable and every function stays small.

use super::{slugify_property_id, SyncOutcome};
use crate::derive::{canonical_hash, infer, parse_ir, Inferred, ParseData, Row};
use crate::lint::bridge::{invoke as spk_lint, Failure};
use crate::lint::walker;
use anyhow::anyhow;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Run sync over the deployed specs scope. `check` = CI mode (no writes).
pub(crate) fn run_sync(spk: &str, repo_root: &Path, check: bool) -> anyhow::Result<SyncOutcome> {
    let cfg = crate::config::load(repo_root)?;
    let specs_dir = repo_root.join(&cfg.paths.specs);
    let contracts_dir = repo_root.join(".espectacular");
    let mut outcome = SyncOutcome::default();

    // C-sync-refusal: lint every dual-format target first; derive from
    // lint-clean specs only. Missing spk = spk-unavailable, no writes.
    let parsed = collect_parsed(spk, &specs_dir, &mut outcome)?;
    if outcome.spk_unavailable.is_some() || !outcome.refusals.is_empty() {
        return Ok(outcome);
    }

    for (spec_path, ir) in &parsed {
        for property in &ir.properties {
            sync_property(&contracts_dir, spec_path, ir, property, check, &mut outcome)?;
        }
    }

    Ok(outcome)
}

/// Lint and parse every dual-format spec, recording refusals on `outcome`.
///
/// Stops early on an unavailable `spk` (C-sync-refusal); a refusal on any
/// other spec keeps collecting so the caller can report all of them.
fn collect_parsed(
    spk: &str,
    specs_dir: &Path,
    outcome: &mut SyncOutcome,
) -> anyhow::Result<Vec<(String, ParseData)>> {
    let (specs, _errors) = walker::walk_specs_with_errors(specs_dir)?;
    let mut parsed: Vec<(String, ParseData)> = Vec::new();
    for spec in &specs {
        if !crate::lint::bridge::is_dual_format(&spec.raw) {
            continue;
        }
        let Some(path) = spec.source_path.to_str() else {
            continue;
        };
        match lint_and_parse(spk, path, &spec.spec_path)? {
            Linted::Parsed(ir) => parsed.push((spec.spec_path.clone(), ir)),
            Linted::Refused(reason) => outcome.refusals.push((spec.spec_path.clone(), reason)),
            Linted::Unavailable(msg) => {
                outcome.spk_unavailable = Some(msg);
                return Ok(parsed);
            }
        }
    }
    Ok(parsed)
}

/// The tri-state result of linting one spec: parsed IR, a refusal, or a
/// missing binary.
enum Linted {
    Parsed(ParseData),
    Refused(String),
    Unavailable(String),
}

/// Lint one spec and, when clean, parse it into the derivation IR slice.
fn lint_and_parse(spk: &str, path: &str, spec_path: &str) -> anyhow::Result<Linted> {
    match spk_lint(spk, path) {
        Err(Failure::Unavailable(msg)) => Ok(Linted::Unavailable(msg)),
        Err(Failure::Broken(msg)) => Ok(Linted::Refused(format!("spk lint failed: {msg}"))),
        Ok(issues) if !issues.is_empty() => {
            let detail = issues
                .iter()
                .map(|i| i.rule_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Ok(Linted::Refused(format!("spk lint is not clean ({detail})")))
        }
        Ok(_) => Ok(Linted::Parsed(parse_spec(spk, path, spec_path)?)),
    }
}

/// Invoke `spk parse --json` for a lint-clean spec and deserialize the IR.
fn parse_spec(spk: &str, path: &str, spec_path: &str) -> anyhow::Result<ParseData> {
    let output = Command::new(spk)
        .args(["parse", path, "--json"])
        .output()
        .map_err(|e| anyhow!("failed to invoke spk parse: {e}"))?
        .stdout;
    parse_ir(&String::from_utf8_lossy(&output))
        .map_err(|e| anyhow!("spk parse failed for {spec_path}: {e}"))
}

/// Derive, then check or write, the contract for one property row.
fn sync_property(
    contracts_dir: &Path,
    spec_path: &str,
    ir: &ParseData,
    property: &Row,
    check: bool,
    outcome: &mut SyncOutcome,
) -> anyhow::Result<()> {
    let contract_id = slugify_property_id(&property.id);
    let path = contracts_dir
        .join(spec_path)
        .join(format!("{contract_id}.toml"));
    let path_str = path.to_string_lossy().into_owned();
    let existing = load_table(&path);
    let inferred = infer(ir, property);
    let hash = canonical_hash(property);
    let table = build_table(existing.as_ref(), &contract_id, property, &inferred, &hash);

    if check {
        record_check(existing.as_ref(), &table, path_str, outcome);
    } else {
        write_contract(&path, &table, path_str, outcome)?;
    }
    Ok(())
}

/// Merge the sync-owned fields (D3) over the existing table, adding the
/// provenance fields only for a brand-new contract.
fn build_table(
    existing: Option<&toml::Table>,
    contract_id: &str,
    property: &Row,
    inferred: &Inferred,
    hash: &str,
) -> toml::Table {
    let mut table = existing.cloned().unwrap_or_default();
    insert_derived_fields(&mut table, contract_id, property, inferred);
    table.insert("derived_from".into(), hash.to_owned().into());
    if existing.is_none() {
        insert_provenance(&mut table);
    }
    table
}

/// Insert the four sync-owned fields other than `derived_from` (D3).
fn insert_derived_fields(
    table: &mut toml::Table,
    contract_id: &str,
    property: &Row,
    inferred: &Inferred,
) {
    table.insert("id".into(), contract_id.to_owned().into());
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
}

/// Seed the provenance fields a brand-new contract needs.
fn insert_provenance(table: &mut toml::Table) {
    table.insert("status".into(), "active".into());
    table.insert("superseded_by".into(), "".into());
    table.insert("authored_with".into(), env!("CARGO_PKG_VERSION").into());
}

/// Record a `--check` verdict for one contract: missing, drifted, or fresh.
fn record_check(
    existing: Option<&toml::Table>,
    table: &toml::Table,
    path_str: String,
    outcome: &mut SyncOutcome,
) {
    match existing {
        None => outcome.missing.push(path_str),
        Some(old) if derived_fingerprint(old) != derived_fingerprint(table) => {
            outcome.stale.push(path_str);
        }
        Some(_) => outcome.fresh.push(path_str),
    }
}

/// Render and write one contract, recording it as written.
fn write_contract(
    path: &Path,
    table: &toml::Table,
    path_str: String,
    outcome: &mut SyncOutcome,
) -> anyhow::Result<()> {
    let rendered = toml::to_string_pretty(table)?;
    fs::create_dir_all(path.parent().expect("contract parent"))?;
    fs::write(path, rendered)?;
    outcome.wrote.push(path_str);
    Ok(())
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
