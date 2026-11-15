//! Contract-property meta tests (the `p_*` group, DDL design D6).
//!
//! Purpose: every `p_*` test drives the behavioural tests bound to the same
//! property, so `ah check`'s `p_` filter sees all falsifying cases for a
//! property in one run.
//!
//! Responsibilities: only call the owning module's tests — no assertions of
//! their own live here.
//!
//! Rationale: the behavioural tests stay next to the code they exercise; the
//! property grouping is a thin, explicit list so a dropped case is visible.

use crate::check::scope::tests::{
    findings_carry_action_fields_and_scenario_prose,
    run_check_conflicting_scenario_modifications_across_changes,
    run_check_overlay_modification_with_staged_contract_passes,
    run_check_reports_overlay_conflict_for_duplicate_added_scenarios,
    run_check_reports_success_with_empty_findings,
    run_check_unsignaled_scenario_redefinition_still_conflicts,
    run_check_with_change_adds_scenarios_to_scope,
    staged_superseded_contract_requires_replacement_in_scope,
};
use crate::check::selection::tests::{
    all_tests_reports_all_tests_source, code_only_change_without_store_runs_everything,
    selection_absent_changed_files_run_everything,
    selection_change_overlay_spec_file_selects_capability,
    selection_changed_spec_file_runs_only_affected_capability,
    selection_zero_overlap_reports_bypassed_runs_everything,
};
use crate::check::stale::tests::contract_stale_emitted_when_row_hash_differs;
use crate::check::structural::tests::{
    findings_ordered_by_spec_scenario_kind, four_findings_fixture_emits_exactly_four,
    invalid_falsifiability_class_blocks_scenario_execution,
    invalid_falsifiability_class_emits_structural_finding, liveness_with_timeout_emits_no_warning,
    liveness_without_timeout_emits_warning_severity, missing_contract_finding_present,
    orphan_contract_finding_present, structural_findings_carry_error_severity,
};

// ---- property-derived contract bindings (DDL migration, design D6) ----

#[test]
fn p_correspondence() {
    missing_contract_finding_present();
    orphan_contract_finding_present();
    findings_carry_action_fields_and_scenario_prose();
}

#[test]
fn p_schema() {
    invalid_falsifiability_class_emits_structural_finding();
    invalid_falsifiability_class_blocks_scenario_execution();
    liveness_without_timeout_emits_warning_severity();
    liveness_with_timeout_emits_no_warning();
    structural_findings_carry_error_severity();
    contract_stale_emitted_when_row_hash_differs();
}

#[test]
fn p_json() {
    run_check_reports_success_with_empty_findings();
    findings_ordered_by_spec_scenario_kind();
    four_findings_fixture_emits_exactly_four();
    findings_carry_action_fields_and_scenario_prose();
}

#[test]
fn p_overlay() {
    run_check_with_change_adds_scenarios_to_scope();
    run_check_reports_overlay_conflict_for_duplicate_added_scenarios();
    run_check_overlay_modification_with_staged_contract_passes();
    run_check_unsignaled_scenario_redefinition_still_conflicts();
    run_check_conflicting_scenario_modifications_across_changes();
    staged_superseded_contract_requires_replacement_in_scope();
}

/// Selection is property-backed (review F6): selection reporting,
/// conservative fallbacks, and the escape hatch all carry the p_ group.
#[test]
fn p_selection() {
    selection_changed_spec_file_runs_only_affected_capability();
    selection_absent_changed_files_run_everything();
    all_tests_reports_all_tests_source();
    selection_zero_overlap_reports_bypassed_runs_everything();
    selection_change_overlay_spec_file_selects_capability();
    code_only_change_without_store_runs_everything();
}

#[test]
fn p_boundary() {
    findings_carry_action_fields_and_scenario_prose();
}

#[test]
fn p_agent_actions() {
    findings_carry_action_fields_and_scenario_prose();
    findings_ordered_by_spec_scenario_kind();
    four_findings_fixture_emits_exactly_four();
}

#[test]
fn p_apply_command() {
    findings_carry_action_fields_and_scenario_prose();
}
