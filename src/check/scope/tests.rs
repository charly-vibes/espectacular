use crate::check::run_check;
use crate::check::test_support::{success_repo, write_executable};
use std::fs;

#[test]
fn run_check_missing_specs_dir_degrades_to_clean_zero_findings() {
    // GH#28.2 / espectacular-iq4: a repo with no deployed specs yet (changes
    // only) must not hard-fail with a bare ENOENT — ah doctor classifies a
    // missing specs dir as a missing-specs-dir finding, so check degrades
    // consistently (0.3.0 parity: clean, zero structural).
    let dir = success_repo();
    fs::remove_dir_all(dir.path().join("openspec/specs")).unwrap();
    fs::remove_dir_all(dir.path().join(".espectacular/compiler")).unwrap();

    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(output.findings.is_empty());
    assert_eq!(output.summary.passed, 0);
}

#[test]
pub(crate) fn run_check_reports_success_with_empty_findings() {
    let dir = success_repo();
    let output = run_check(dir.path(), &[], true).unwrap();
    assert!(output.findings.is_empty());
    assert_eq!(output.summary.passed, 1);
    assert!(output.summary.counts_by_kind.is_empty());
}

#[test]
pub(crate) fn run_check_with_change_adds_scenarios_to_scope() {
    let dir = success_repo();
    fs::create_dir_all(
        dir.path()
            .join("openspec/changes/add-parser/specs/compiler"),
    )
    .unwrap();
    fs::create_dir_all(dir.path().join(".espectacular/changes/add-parser/compiler")).unwrap();
    fs::write(
        dir.path().join("openspec/changes/add-parser/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Added path\n- **WHEN** change applies\n- **THEN** it passes\n",
    ).unwrap();
    fs::write(
        dir.path().join(".espectacular/changes/add-parser/compiler/added-path.toml"),
        "id = \"added-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
    ).unwrap();

    let output = run_check(dir.path(), &["add-parser".to_string()], true).unwrap();
    assert_eq!(output.scope.changes, vec!["add-parser"]);
    assert_eq!(output.summary.passed, 2);
}

#[test]
pub(crate) fn run_check_reports_overlay_conflict_for_duplicate_added_scenarios() {
    let dir = success_repo();
    for change in ["zeta", "alpha"] {
        fs::create_dir_all(
            dir.path()
                .join(format!("openspec/changes/{change}/specs/compiler")),
        )
        .unwrap();
        fs::create_dir_all(
            dir.path()
                .join(format!(".espectacular/changes/{change}/compiler")),
        )
        .unwrap();
        fs::write(
            dir.path().join(format!("openspec/changes/{change}/specs/compiler/spec.md")),
            "# Capability: compiler\n\n#### Scenario: Added path\n- **WHEN** change applies\n- **THEN** it passes\n",
        ).unwrap();
    }

    let output = run_check(dir.path(), &["zeta".to_string(), "alpha".to_string()], true).unwrap();
    assert_eq!(output.scope.changes, vec!["alpha", "zeta"]);
    assert!(output.findings.iter().any(|f| f.kind == "overlay-conflict"));
    assert_eq!(
        output.summary.counts_by_kind.get("overlay-conflict"),
        Some(&1)
    );
}

#[test]
pub(crate) fn run_check_overlay_modification_with_staged_contract_passes() {
    // RED (espectacular-hct / allow-overlay-scenario-modification):
    // a change delta that redefines a deployed scenario id WITH a staged
    // contract is a deliberate modification — the overlay text replaces
    // the base text, no overlay-conflict.
    let dir = success_repo();
    fs::create_dir_all(
        dir.path()
            .join("openspec/changes/mod-parser/specs/compiler"),
    )
    .unwrap();
    fs::create_dir_all(dir.path().join(".espectacular/changes/mod-parser/compiler")).unwrap();
    fs::write(
        dir.path().join("openspec/changes/mod-parser/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs modified\n- **THEN** it passes differently\n",
    ).unwrap();
    fs::write(
        dir.path().join(".espectacular/changes/mod-parser/compiler/green-path.toml"),
        "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
    ).unwrap();

    let output = run_check(dir.path(), &["mod-parser".to_string()], true).unwrap();
    assert!(
        !output.findings.iter().any(|f| f.kind == "overlay-conflict"),
        "staged-contract modification must not conflict; got: {:?}",
        output.findings
    );
}

#[test]
fn run_check_identical_body_carry_along_does_not_conflict() {
    // RED (espectacular-4u6): the gate spec pins overlay-conflict to
    // redefinitions "with different body text". openspec's MODIFIED-
    // wholesale rule forces verbatim carry-along of unchanged scenarios
    // into every MODIFIED delta, so a byte-identical (spec, id) without a
    // staged contract is a legitimate no-op — no overlay-conflict.
    let dir = success_repo();
    fs::create_dir_all(
        dir.path()
            .join("openspec/changes/carry-parser/specs/compiler"),
    )
    .unwrap();
    fs::write(
        dir.path().join("openspec/changes/carry-parser/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n",
    )
    .unwrap();

    let output = run_check(dir.path(), &["carry-parser".to_string()], true).unwrap();
    assert!(
        !output.findings.iter().any(|f| f.kind == "overlay-conflict"),
        "byte-identical carry-along must not conflict; got: {:?}",
        output.findings
    );
    assert_eq!(output.summary.passed, 1);
}

#[test]
pub(crate) fn run_check_unsignaled_scenario_redefinition_still_conflicts() {
    let dir = success_repo();
    fs::create_dir_all(
        dir.path()
            .join("openspec/changes/mod-parser/specs/compiler"),
    )
    .unwrap();
    fs::write(
        dir.path().join("openspec/changes/mod-parser/specs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs modified\n- **THEN** it passes differently\n",
    ).unwrap();

    let output = run_check(dir.path(), &["mod-parser".to_string()], true).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "overlay-conflict"),
        "redefinition without a staged contract must still conflict; got: {:?}",
        output.findings
    );
}

#[test]
pub(crate) fn run_check_conflicting_scenario_modifications_across_changes() {
    let dir = success_repo();
    for change in ["zeta", "alpha"] {
        fs::create_dir_all(
            dir.path()
                .join(format!("openspec/changes/{change}/specs/compiler")),
        )
        .unwrap();
        fs::create_dir_all(
            dir.path()
                .join(format!(".espectacular/changes/{change}/compiler")),
        )
        .unwrap();
        fs::write(
            dir.path().join(format!("openspec/changes/{change}/specs/compiler/spec.md")),
            "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs modified\n- **THEN** it passes differently\n",
        ).unwrap();
        fs::write(
            dir.path().join(format!(".espectacular/changes/{change}/compiler/green-path.toml")),
            "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
        ).unwrap();
    }

    let output = run_check(dir.path(), &["zeta".to_string(), "alpha".to_string()], true).unwrap();
    assert!(
        output.findings.iter().any(|f| f.kind == "overlay-conflict"),
        "two changes modifying the same deployed scenario must conflict; got: {:?}",
        output.findings
    );
}

#[test]
pub(crate) fn findings_carry_action_fields_and_scenario_prose() {
    let dir = success_repo();
    write_executable(&dir.path().join("runner.sh"), "printf 'boom' >&2\nexit 7");

    let output = run_check(dir.path(), &[], true).unwrap();
    let finding = output
        .findings
        .iter()
        .find(|f| f.kind == "test-failing")
        .unwrap();

    assert_eq!(finding.suggested_action, "edit_code_not_scenario");
    assert_eq!(
        finding.playbook_command,
        "ah explain edit_code_not_scenario"
    );
    assert_eq!(
        finding.scenario_prose.as_deref(),
        Some("- **WHEN** it runs\n- **THEN** it passes")
    );
}

#[test]
pub(crate) fn staged_superseded_contract_requires_replacement_in_scope() {
    let dir = success_repo();
    fs::create_dir_all(dir.path().join(".espectacular/changes/add-parser/compiler")).unwrap();
    fs::write(
        dir.path().join(".espectacular/changes/add-parser/compiler/green-path.toml"),
        "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"superseded\"\nsuperseded_by = \"missing\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
    ).unwrap();
    fs::create_dir_all(dir.path().join("openspec/changes/add-parser/specs")).unwrap();

    let output = run_check(dir.path(), &["add-parser".to_string()], true).unwrap();
    assert!(output
        .findings
        .iter()
        .any(|f| f.kind == "missing-replacement"));
}

#[test]
fn configured_specs_path_used_in_structural_findings() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    // Use a non-default specs root
    fs::create_dir_all(repo.join("myspecs/compiler")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
    fs::write(
        repo.join("myspecs/compiler/spec.md"),
        "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/config.toml"),
        "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"myspecs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n",
    )
    .unwrap();
    // No contract for this scenario — triggers a no-toml structural finding
    write_executable(&repo.join("runner.sh"), "exit 0");

    let output = run_check(dir.path(), &[], false).unwrap();
    let no_toml = output
        .findings
        .iter()
        .find(|f| f.kind == "no-toml")
        .expect("expected no-toml finding");
    assert!(
        no_toml.spec_path.contains("myspecs/"),
        "spec_path should use configured specs root 'myspecs', got: {}",
        no_toml.spec_path
    );
    assert!(
        !no_toml.spec_path.contains("openspec/specs"),
        "spec_path should not contain default 'openspec/specs', got: {}",
        no_toml.spec_path
    );
}
