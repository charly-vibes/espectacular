use crate::check::run_check;
use std::fs;

// ---- VERIFIES coverage (derive-contracts-from-specodelic task 4.1) ----

const LINKED_BODY: &str = "- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[spec.P-token]]\n";
const UNLINKED_BODY: &str =
    "- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n";

const DERIVED_CONTRACT: &str = "id = \"p-token\"\ndescription = \"invalid tokens rejected\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.9.0\"\nfalsifiability_class = \"safety\"\nderived_from = \"P-token@0123456789ab\"\n\n[[tests.cargo]]\nflags = \"p_token::case1\"\n";

const OTHER_DERIVED_CONTRACT: &str = "id = \"p-other\"\ndescription = \"second property covered by the same scenario\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.9.0\"\nfalsifiability_class = \"safety\"\nderived_from = \"P-other@0123456789ab\"\n\n[[tests.cargo]]\nflags = \"p_other::case1\"\n";

fn verifies_repo(scenario_bodies: &[&str], contracts: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/auth")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/auth")).unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
    ).unwrap();
    let mut spec = String::from("# Capability: auth\n");
    for (i, body) in scenario_bodies.iter().enumerate() {
        spec.push_str(&format!("\n#### Scenario: Token check {i}\n{body}\n"));
    }
    fs::write(repo.join("openspec/specs/auth/spec.md"), spec).unwrap();
    for (name, content) in contracts {
        fs::write(repo.join(".espectacular/auth").join(name), content).unwrap();
    }
    dir
}

#[test]
fn verifies_linked_scenario_is_covered_by_derived_contract() {
    let dir = verifies_repo(&[LINKED_BODY], &[("p-token.toml", DERIVED_CONTRACT)]);
    let output = run_check(dir.path(), &[], false).unwrap();
    for kind in ["no-toml", "no-tests-declared", "orphan-toml", "id-mismatch"] {
        assert!(
            !output.findings.iter().any(|f| f.kind == kind),
            "VERIFIES-linked scenario must not emit {kind}; got: {:?}",
            output.findings
        );
    }
}

#[test]
fn verifies_real_id_frontmatter_link_is_covered_by_derived_contract() {
    // Revision 18: the link is file-local under the real parent-dir id
    // (`auth.tokens.P-token`), not the legacy `spec.P-token` spelling.
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/auth-tokens")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/auth-tokens")).unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\n",
    )
    .unwrap();
    fs::write(
        repo.join("openspec/specs/auth-tokens/spec.md"),
        "---\nid: auth.tokens\nkind: intent\nstatement: \"WHEN checked THE system SHALL reject invalid tokens\"\n---\n\n# Capability: auth tokens\n\n#### Scenario: Token check 0\n- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[auth.tokens.P-token]]\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/auth-tokens/p-token.toml"),
        DERIVED_CONTRACT,
    )
    .unwrap();
    let output = run_check(repo, &[], false).unwrap();
    for kind in ["no-toml", "orphan-toml"] {
        assert!(
            !output.findings.iter().any(|f| f.kind == kind),
            "real-id VERIFIES link must resolve; got: {:?}",
            output.findings
        );
    }
}

#[test]
fn scenario_with_two_links_covers_both_derived_contracts() {
    // A scenario carrying two VERIFIES bullets asserts both properties;
    // both derived contracts are in use — neither may be orphan-toml.
    // (Full-corpus migration: P-handlers/P-selfheal share one scenario.)
    const TWO_LINKS: &str = "- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[spec.P-token]]\n- **VERIFIES** [[spec.P-other]]\n";
    let dir = verifies_repo(
        &[TWO_LINKS],
        &[
            ("p-token.toml", DERIVED_CONTRACT),
            ("p-other.toml", OTHER_DERIVED_CONTRACT),
        ],
    );
    let output = run_check(dir.path(), &[], false).unwrap();
    assert!(
        !output.findings.iter().any(|f| f.kind == "orphan-toml"),
        "both linked contracts are in use; got: {:?}",
        output.findings
    );
}

#[test]
fn covered_scenario_executes_derived_contract_tests() {
    let dir = verifies_repo(&[LINKED_BODY], &[("p-token.toml", DERIVED_CONTRACT)]);
    // Swap the derived contract's tests for a passing shell entry to prove
    // the redirect feeds the execution path, not just the structural pass.
    let contract = dir.path().join(".espectacular/auth/p-token.toml");
    let text = fs::read_to_string(&contract).unwrap();
    fs::write(
        &contract,
        text.replace(
            "[[tests.cargo]]\nflags = \"p_token::case1\"",
            "[[tests.shell]]\ncommand = \"exit 0\"",
        ),
    )
    .unwrap();
    let output = run_check(dir.path(), &[], true).unwrap();
    assert_eq!(
        output.summary.passed, 1,
        "derived contract tests must run; got: {:?}",
        output.findings
    );
}

#[test]
fn unlinked_scenario_still_requires_own_contract() {
    let dir = verifies_repo(&[UNLINKED_BODY], &[("p-token.toml", DERIVED_CONTRACT)]);
    let output = run_check(dir.path(), &[], false).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "no-toml"),
        "unlinked scenario must still emit no-toml; got: {:?}",
        output.findings
    );
}

#[test]
fn link_to_testless_derived_contract_still_flags_no_tests_declared() {
    // The link alone is not enough: the covering derived contract must
    // actually declare tests (C-verifies-covers covers the scenario; the
    // contract's own no-tests-declared check still applies).
    let empty = DERIVED_CONTRACT.replace("\n[[tests.cargo]]\nflags = \"p_token::case1\"", "");
    let dir = verifies_repo(&[LINKED_BODY], &[("p-token.toml", empty.as_str())]);
    let output = run_check(dir.path(), &[], false).unwrap();
    assert!(
        output
            .findings
            .iter()
            .any(|f| f.kind == "no-tests-declared"),
        "testless derived contract must still emit no-tests-declared; got: {:?}",
        output.findings
    );
}

#[test]
fn verifies_link_to_absent_property_keeps_own_contract_requirement() {
    let dir = verifies_repo(&[LINKED_BODY], &[]);
    let output = run_check(dir.path(), &[], false).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "no-toml"),
        "link without a derived contract must still emit no-toml; got: {:?}",
        output.findings
    );
}

// ---- Regression: plain + Properties-less specs unaffected (task 4.5) ----
// C-plain-unaffected: plain openspec specs and dual-format specs without
// Properties rows discover and check exactly as before this change.

#[test]
fn plain_spec_with_derived_named_contract_is_unaffected() {
    // Pre-change corpus shape: no VERIFIES bullet anywhere, but a
    // hand-written contract whose id matches no scenario → no-toml for
    // the scenario plus orphan-toml for the stray contract, exactly as
    // before coverage existed.
    let dir = verifies_repo(&[UNLINKED_BODY], &[("p-token.toml", DERIVED_CONTRACT)]);
    let output = run_check(dir.path(), &[], false).unwrap();
    assert!(output.findings.iter().any(|f| f.kind == "no-toml"));
    assert!(output
        .findings
        .iter()
        .any(|f| f.kind == "orphan-toml" && f.scenario.id == "p-token"));
}

#[test]
fn plain_spec_verifies_bullet_without_derived_contract_changes_nothing() {
    // A bullet in a plain (frontmatter-less) spec is inert when no
    // derived contract backs it: same single no-toml finding as without
    // the bullet.
    let with_link = verifies_repo(&[LINKED_BODY], &[]);
    let without_link = verifies_repo(&[UNLINKED_BODY], &[]);
    let extract = |output: &crate::check::CheckOutput| {
        let mut kinds: Vec<(String, String)> = output
            .findings
            .iter()
            .map(|f| (f.kind.clone(), f.scenario.id.clone()))
            .collect();
        kinds.sort();
        format!("{kinds:?}")
    };
    let a = run_check(with_link.path(), &[], false).unwrap();
    let b = run_check(without_link.path(), &[], false).unwrap();
    assert_eq!(
        extract(&a),
        extract(&b),
        "bullet must be inert without a derived contract"
    );
    assert!(a.findings.iter().any(|f| f.kind == "no-toml"));
}

#[test]
fn propertiesless_dual_format_spec_is_unaffected() {
    // Dual-format frontmatter (id: spec) but no Properties rows: the
    // bullet must not suppress the scenario's own contract requirement.
    let dir = verifies_repo(&[LINKED_BODY], &[]);
    fs::write(
        dir.path().join("openspec/specs/auth/spec.md"),
        "---\nid: spec\nkind: intent\nstatement: \"WHEN checked THE system SHALL reject invalid tokens\"\n---\n\n# Capability: auth\n\n#### Scenario: Token check 0\n- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[spec.P-token]]\n",
    )
    .unwrap();
    let output = run_check(dir.path(), &[], false).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "no-toml"),
        "Properties-less dual-format spec must keep the contract requirement; got: {:?}",
        output.findings
    );
}

#[test]
fn discovery_output_is_identical_for_bullet_bearing_scenarios() {
    // Discovery is text-faithful: a VERIFIES bullet is part of the body
    // and must not change scenario ids, headings, or line numbers.
    let dir = verifies_repo(&[LINKED_BODY], &[]);
    let scenarios =
        crate::openspec::discover_scenarios(dir.path().join("openspec/specs").to_str().unwrap())
            .unwrap();
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0].id, "token-check-0");
    assert_eq!(scenarios[0].heading, "Token check 0");
    assert!(scenarios[0].body.contains("**VERIFIES** [[spec.P-token]]"));
}
