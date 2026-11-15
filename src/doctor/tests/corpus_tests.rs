//! Contract-corpus diagnostics: collisions, orphan contracts, archetypes.

use super::super::runner::run_doctor;
use super::super::types::DoctorOutcome;
use super::fixtures::{has_issue, issues, make_healthy_repo};
use std::fs;
use std::path::Path;

#[test]
fn scenario_collision_emits_collision_diagnostic() {
    let repo = make_healthy_repo();
    let spec_dir = repo.path().join("openspec/specs/compiler");
    fs::create_dir_all(&spec_dir).unwrap();
    // adopt-dual-format-specs: collisions now mean same id AND different
    // bodies; identical bodies are section-sync mirrors and dedupe.
    let content = "# Capability: compiler\n\n## DEPLOYED Requirements\n\n### Requirement: R\n\n#### Scenario: Empty input rejected\n- **GIVEN** x\n- **WHEN** y\n- **THEN** z\n\n#### Scenario: Empty input rejected\n- **GIVEN** null bytes\n- **WHEN** y\n- **THEN** z\n";
    fs::write(spec_dir.join("spec.md"), content).unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "scenario-collisions"),
        "duplicate scenario headings must emit scenario-collisions diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn orphan_contract_emits_orphan_diagnostic() {
    let repo = make_healthy_repo();
    let contract_dir = repo.path().join(".espectacular/compiler");
    fs::create_dir_all(&contract_dir).unwrap();
    fs::write(
        contract_dir.join("ghost-scenario.toml"),
        "id = \"ghost-scenario\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "orphan-contracts"),
        "orphan contract must emit orphan-contracts diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn derived_contract_referenced_by_verifies_link_is_not_orphan() {
    // Full-corpus migration: derived p-* contracts are referenced by
    // VERIFIES bullets, not by scenario-slug filenames — doctor must not
    // flag them as orphans (same suppression as the ah check gate).
    let repo = make_healthy_repo();
    let spec_dir = repo.path().join("openspec/specs/auth");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        "---\nid: spec\nkind: intent\nstatement: \"WHEN checked THE system SHALL reject invalid tokens\"\n---\n\n# Capability: auth\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-token | unit | [[spec.C-a]] | input | output |\n\n## Requirements\n\n### Requirement: Token check\nThe system SHALL reject invalid tokens.\n\n#### Scenario: Token rejected\n- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[spec.P-token]]\n",
    )
    .unwrap();
    let contract_dir = repo.path().join(".espectacular/auth");
    fs::create_dir_all(&contract_dir).unwrap();
    fs::write(
        contract_dir.join("p-token.toml"),
        "id = \"p-token\"\ndescription = \"invalid tokens rejected\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        !has_issue(&report, "orphan-contracts"),
        "VERIFIES-referenced derived contract must not be orphan; got: {:?}",
        issues(&report)
    );
}

#[test]
fn rev18_verifies_link_with_real_file_id_is_not_orphan() {
    // rev-18 trees carry real capability ids in VERIFIES links
    // ([[auth.P-token]], [[lint.P-vague]], ...) instead of the legacy
    // [[spec.P-x]] shape. Doctor's orphan suppression must parse the
    // open-ended link shape (same as ah check) or every derived
    // contract is reported as an orphan.
    let repo = make_healthy_repo();
    let spec_dir = repo.path().join("openspec/specs/auth");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        "---\nid: auth\nkind: intent\nstatement: \"WHEN checked THE system SHALL reject invalid tokens\"\n---\n\n# Capability: auth\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-token | unit | [[auth.C-a]] | input | output |\n\n## Requirements\n\n### Requirement: Token check\nThe system SHALL reject invalid tokens.\n\n#### Scenario: Token rejected\n- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[auth.P-token]]\n",
    )
    .unwrap();
    let contract_dir = repo.path().join(".espectacular/auth");
    fs::create_dir_all(&contract_dir).unwrap();
    fs::write(
        contract_dir.join("p-token.toml"),
        "id = \"p-token\"\ndescription = \"invalid tokens rejected\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(
        !has_issue(&report, "orphan-contracts"),
        "rev-18 VERIFIES-referenced derived contract must not be orphan; got: {:?}",
        issues(&report)
    );
}

#[test]
fn deployed_only_corpus_orphan_matching_is_selective() {
    // wanna-vk3 corpus shape: deployed-only layout (all specs live under
    // openspec/specs, no top-level specs/) with rev-18 real-id VERIFIES
    // links. One fixture pins all three matching classes together so the
    // suppression cannot regress in either direction: the scenario-slug
    // contract matches by heading slug, the VERIFIES-derived contract is
    // suppressed, and the unlinked contract still fires — matching must
    // stay selective, never vacuously allow-all.
    let repo = make_healthy_repo();
    write_deployed_only_corpus(repo.path());
    let report = run_doctor(repo.path()).unwrap();
    let orphan_findings = orphan_contract_findings(&report);
    assert_eq!(
        orphan_findings.len(),
        1,
        "deployed-only corpus must flag exactly the unlinked contract; got: {:?}",
        orphan_findings
            .iter()
            .map(|c| c.message.clone())
            .collect::<Vec<_>>()
    );
    assert!(
        orphan_findings[0].message.contains("ghost-contract.toml"),
        "the single orphan finding must name the unlinked contract; got: {}",
        orphan_findings[0].message
    );
}

/// The wanna-vk3 deployed-only corpus: one matching scenario-slug contract,
/// one VERIFIES-referenced derived contract, and one unlinked contract.
fn write_deployed_only_corpus(root: &Path) {
    let spec_dir = root.join("openspec/specs/auth");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join("spec.md"), DEPLOYED_ONLY_SPEC).unwrap();
    let contract_dir = root.join(".espectacular/auth");
    fs::create_dir_all(&contract_dir).unwrap();
    let contract = |id: &str| {
        format!(
            "id = \"{id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n"
        )
    };
    for id in ["token-rejected", "p-token", "ghost-contract"] {
        fs::write(contract_dir.join(format!("{id}.toml")), contract(id)).unwrap();
    }
}

fn orphan_contract_findings(outcome: &DoctorOutcome) -> Vec<&genesis::doctor::CheckEntry> {
    issues(outcome)
        .into_iter()
        .filter(|c| c.name == "orphan-contracts")
        .collect()
}

const DEPLOYED_ONLY_SPEC: &str = "---\nid: auth\nkind: intent\nstatement: \"WHEN checked THE system SHALL reject invalid tokens\"\n---\n\n# Capability: auth\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-token | unit | [[auth.C-a]] | input | output |\n\n## Requirements\n\n### Requirement: Token check\nThe system SHALL reject invalid tokens.\n\n#### Scenario: Token rejected\n- **WHEN** a token is checked\n- **THEN** invalid tokens are rejected\n- **VERIFIES** [[auth.P-token]]\n";

#[test]
fn unknown_archetype_emits_diagnostic() {
    let repo = make_healthy_repo();
    let spec_dir = repo.path().join("openspec/specs/compiler");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_content = "# Capability: compiler\n\n## DEPLOYED Requirements\n\n### Requirement: R\n\n#### Scenario: Empty input rejected\n- **GIVEN** x\n- **WHEN** y\n- **THEN** z\n";
    fs::write(spec_dir.join("spec.md"), spec_content).unwrap();
    let contract_dir = repo.path().join(".espectacular/compiler");
    fs::create_dir_all(&contract_dir).unwrap();
    fs::write(
        contract_dir.join("empty-input-rejected.toml"),
        "id = \"empty-input-rejected\"\ndescription = \"\"\narchetype = \"UNKNOWN_TYPE\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    assert!(!report.genesis_report.is_healthy());
    assert!(
        has_issue(&report, "unknown-archetypes"),
        "unknown archetype must emit unknown-archetypes diagnostic; got: {:?}",
        issues(&report)
    );
}

#[test]
fn each_problem_emits_exactly_one_diagnostic_kind() {
    let repo = make_healthy_repo();
    fs::write(
        repo.path().join(".espectacular/config.toml"),
        "tool_version = \"\"\n[paths]\nspecs = \"\"\nchanges = \"\"\n[runners]\n",
    )
    .unwrap();
    let report = run_doctor(repo.path()).unwrap();
    let config_count = issues(&report)
        .iter()
        .filter(|c| c.name == "config")
        .count();
    assert_eq!(config_count, 1, "should emit exactly one config diagnostic");
}
