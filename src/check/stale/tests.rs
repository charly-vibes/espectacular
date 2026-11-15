use super::contract_stale_findings;
use crate::check::run_check;
use crate::check::scope::{ResolvedScenario, ResolvedScope};
use crate::check::test_support::write_executable;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

// ---- contract-stale (derive-contracts-from-specodelic task 4.2) ----

const P_TOKEN_CELLS: &[(&str, &str)] = &[
    ("id", "P-token"),
    ("kind", "unit"),
    ("derives_from", "[[spec.C-token]]"),
    ("generator", "valid vs invalid tokens"),
    ("predicate", "invalid tokens rejected with 401"),
];

fn p_token_row() -> crate::derive::Row {
    crate::derive::Row {
        id: "P-token".to_string(),
        kind: Some("unit".to_string()),
        cells: P_TOKEN_CELLS
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

/// Fake spk: `parse <path> --json` emits an IR carrying exactly the
/// P-token row from `p_token_row`.
fn parse_shim(dir: &Path, name: &str) -> String {
    // Payload goes to a file and the shim `cat`s it. The earlier form fed
    // the JSON through printf's FORMAT string with \" escapes, which only
    // bash collapses — dash (CI's /bin/sh) prints them literally and the
    // parse result becomes invalid JSON (espectacular-pli CI failure).
    let mut cells = String::new();
    for (k, v) in P_TOKEN_CELLS {
        cells.push_str(&format!("\"{k}\":\"{v}\","));
    }
    cells.pop();
    let ir = format!(
        "{{\"ok\":true,\"data\":{{\"properties\":[{{\"id\":\"P-token\",\"kind\":\"unit\",\"cells\":{{{cells}}}}}],\"constraints\":[],\"states\":[],\"transitions\":[]}}}}"
    );
    let payload = dir.join(format!("{name}.json"));
    fs::write(&payload, &ir).unwrap();
    write_executable(
        &dir.join(name),
        &format!(
            "if [ \"$1\" = parse ]; then\n  cat '{}'\nelse\n  printf '%s' '{{\"ok\":true,\"data\":{{\"issues\":[]}}}}'\nfi",
            payload.display()
        ),
    );
    dir.join(name).to_string_lossy().into_owned()
}

fn stale_fixture(derived_from: &str) -> (tempfile::TempDir, ResolvedScope) {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/auth")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/auth")).unwrap();
    fs::write(
        repo.join("openspec/specs/auth/spec.md"),
        "# Capability: auth\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/auth/p-token.toml"),
        format!(
            "id = \"p-token\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.9.0\"\nfalsifiability_class = \"safety\"\nderived_from = \"{derived_from}\"\n\n[[tests.cargo]]\nflags = \"p_token::case1\"\n"
        ),
    )
    .unwrap();
    let scope = ResolvedScope {
        scenarios: Vec::new(),
        contract_files: vec![(
            "auth".to_string(),
            "p-token".to_string(),
            repo.join(".espectacular/auth/p-token.toml"),
        )],
        changes: Vec::new(),
        findings: Vec::new(),
        covered: BTreeMap::new(),
    };
    (dir, scope)
}

#[test]
pub(crate) fn contract_stale_emitted_when_row_hash_differs() {
    let (dir, scope) = stale_fixture("P-token@deadbeefcafe");
    let spk = parse_shim(dir.path(), "spk-stale");
    let findings = contract_stale_findings(&dir.path().join("openspec/specs"), &scope, &spk);
    assert!(
        findings
            .iter()
            .any(|f| f.kind == "contract-stale" && f.category == "structural"),
        "expected contract-stale; got: {:?}",
        findings
    );
}

#[test]
fn fresh_derived_contract_emits_no_contract_stale() {
    let fresh = crate::derive::canonical_hash(&p_token_row());
    let (dir, scope) = stale_fixture(&fresh);
    let spk = parse_shim(dir.path(), "spk-fresh");
    let findings = contract_stale_findings(&dir.path().join("openspec/specs"), &scope, &spk);
    assert!(
        !findings.iter().any(|f| f.kind == "contract-stale"),
        "unexpected contract-stale; got: {:?}",
        findings
    );
}

#[test]
fn derived_from_property_absent_from_ir_is_stale() {
    let (dir, scope) = stale_fixture("P-gone@deadbeefcafe");
    let spk = parse_shim(dir.path(), "spk-gone");
    let findings = contract_stale_findings(&dir.path().join("openspec/specs"), &scope, &spk);
    let stale = findings
        .iter()
        .find(|f| f.kind == "contract-stale")
        .expect("absent property must be stale");
    assert!(
        stale
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("P-gone"),
        "message must name the absent property; got: {:?}",
        stale.message
    );
}

#[test]
fn contract_stale_finding_attaches_to_covering_scenario() {
    let (dir, mut scope) = stale_fixture("P-token@deadbeefcafe");
    scope.covered.insert(
        ("auth".to_string(), "token-check".to_string()),
        vec!["p-token".to_string()],
    );
    scope.scenarios.push(ResolvedScenario {
        scenario: crate::openspec::Scenario {
            id: "token-check".to_string(),
            heading: "Token check".to_string(),
            spec_path: "auth".to_string(),
            source_line: 3,
            body: "- **VERIFIES** [[spec.P-token]]".to_string(),
        },
        contract_path: dir.path().join(".espectacular/auth/p-token.toml"),
    });
    let spk = parse_shim(dir.path(), "spk-covered");
    let findings = contract_stale_findings(&dir.path().join("openspec/specs"), &scope, &spk);
    let stale = findings
        .iter()
        .find(|f| f.kind == "contract-stale")
        .expect("stale finding");
    assert_eq!(stale.scenario.id, "token-check");
}

#[test]
fn contract_stale_degrades_without_spk() {
    let (dir, scope) = stale_fixture("P-token@deadbeefcafe");
    let findings = contract_stale_findings(
        &dir.path().join("openspec/specs"),
        &scope,
        "spk-definitely-not-on-path-xyz",
    );
    assert!(
        findings.is_empty(),
        "missing spk must degrade to no findings; got: {:?}",
        findings
    );
}

#[test]
fn run_check_does_not_fail_on_derived_contracts_without_spk_changes() {
    // Regression: a repo with derived contracts and no spk on PATH still
    // checks normally (stale detection silently degrades, design D2).
    let (dir, _scope) = stale_fixture("P-token@deadbeefcafe");
    fs::write(
        dir.path().join(".espectacular/config.toml"),
        "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
    )
    .unwrap();
    let output = run_check(dir.path(), &[], false).unwrap();
    assert!(
        !output.findings.iter().any(|f| f.kind == "contract-stale"),
        "stale must degrade without spk; got: {:?}",
        output.findings
    );
    // The structural pass still sees the contract as healthy.
    assert!(
        !output
            .findings
            .iter()
            .any(|f| f.kind == "malformed-contract"),
        "derived contract must load; got: {:?}",
        output.findings
    );
}
