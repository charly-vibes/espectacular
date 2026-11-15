use crate::check::run_check_with_selection;
use crate::check::test_support::success_repo;
use std::fs;

// ---- test selection (espectacular-0k8, Layer 1) ----

/// Layer 1 selection (espectacular-0k8): with an explicit changed-file
/// set, only the capability owning the changed spec file runs its
/// contract tests; the untouched capability's contract tests are
/// skipped, and the selection is reported in the JSON output.
#[test]
pub(crate) fn selection_changed_spec_file_runs_only_affected_capability() {
    let dir = two_capability_repo();

    // sanity: without a changed-file set, both run (current behavior)
    let output = run_check_with_selection(dir.path(), &[], true, &[], false).unwrap();
    assert_eq!(output.summary.passed, 2);

    let output = run_check_with_selection(
        dir.path(),
        &[],
        true,
        &["openspec/specs/compiler/spec.md".to_string()],
        false,
    )
    .unwrap();
    let selection = output
        .selection
        .as_ref()
        .expect("selection must be reported");
    assert_eq!(selection.selected.len(), 1, "one capability selected");
    assert_eq!(selection.selected[0], "compiler");
    assert_eq!(selection.skipped, 1, "untouched capability skipped");
    assert_eq!(selection.source, "spec-provenance");
    assert_eq!(selection.unmapped_files, 0, "spec file maps directly");
    assert_eq!(output.summary.passed, 1, "only the affected contract ran");
}

/// No changed-file set (or empty diff) means no selection signal —
/// everything runs, selection reporting is omitted.
#[test]
pub(crate) fn selection_absent_changed_files_run_everything() {
    let dir = two_capability_repo();
    let output = run_check_with_selection(dir.path(), &[], true, &[], false).unwrap();
    assert!(
        output.selection.is_none(),
        "no selection without changed files"
    );
    assert_eq!(output.summary.passed, 2);
}

/// Escape hatch (espectacular-0k8): --all-tests must bypass selection
/// and run every declared contract test even when a changed-file set
/// is supplied.
#[test]
pub(crate) fn all_tests_reports_all_tests_source() {
    let dir = two_capability_repo();
    let output = run_check_with_selection(
        dir.path(),
        &[],
        true,
        &["openspec/specs/compiler/spec.md".to_string()],
        true,
    )
    .unwrap();
    let selection = output
        .selection
        .as_ref()
        .expect("--all-tests must report its bypass");
    assert_eq!(selection.source, "all-tests");
    assert_eq!(selection.skipped, 0, "everything ran");
    assert_eq!(output.summary.passed, 2);
}

/// A code file (neither spec nor contract) in the changed set with no
/// testaruda store must NOT deselect anything — conservative fallback
/// avoids false greens (espectacular-0k8 exit-20 correction).
#[test]
pub(crate) fn code_only_change_without_store_runs_everything() {
    let dir = two_capability_repo();
    let output =
        run_check_with_selection(dir.path(), &[], true, &["src/parser.rs".to_string()], false)
            .unwrap();
    assert_eq!(output.summary.passed, 2, "conservative: all contracts run");
}

/// Zero-overlap guard (review F1): a changed set that maps to
/// capabilities owning NO in-scope scenario (e.g. .espectacular/
/// config.toml -> bogus "config") must NOT deselect everything — the
/// run is conservative (all pass through) and the bypass is reported
/// in the JSON, never a hidden green. unmapped_files counts changed
/// files that map to no capability at all (review F9).
#[test]
pub(crate) fn selection_zero_overlap_reports_bypassed_runs_everything() {
    let dir = two_capability_repo();
    let output = run_check_with_selection(
        dir.path(),
        &[],
        true,
        &[
            ".espectacular/config.toml".to_string(),
            "src/parser.rs".to_string(),
        ],
        false,
    )
    .unwrap();
    let selection = output
        .selection
        .as_ref()
        .expect("bypass must be reported, not silent");
    assert_eq!(selection.source, "bypassed");
    assert!(selection.selected.is_empty(), "nothing was selected");
    assert_eq!(selection.skipped, 0, "everything ran");
    assert_eq!(
        selection.unmapped_files, 1,
        "src/parser.rs maps to no capability"
    );
    assert_eq!(output.summary.passed, 2, "conservative: all contracts ran");
}

fn two_capability_repo() -> tempfile::TempDir {
    let dir = success_repo();
    let repo = dir.path();
    fs::create_dir_all(repo.join("openspec/specs/parser")).unwrap();
    fs::create_dir_all(repo.join(".espectacular/parser")).unwrap();
    fs::write(
        repo.join("openspec/specs/parser/spec.md"),
        "# Capability: parser\n\n#### Scenario: Parse path\n- **WHEN** it parses\n- **THEN** it passes\n",
    )
    .unwrap();
    fs::write(
        repo.join(".espectacular/parser/parse-path.toml"),
        "id = \"parse-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
    )
    .unwrap();
    dir
}

/// Overlay mapping (review F3): a changed file under
/// openspec/changes/<change>/specs/<capability>/ must select the
/// owning capability — change-overlay spec edits previously selected
/// nothing.
#[test]
pub(crate) fn selection_change_overlay_spec_file_selects_capability() {
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

    let output = run_check_with_selection(
        dir.path(),
        &["add-parser".to_string()],
        true,
        &["openspec/changes/add-parser/specs/compiler/spec.md".to_string()],
        false,
    )
    .unwrap();
    let selection = output
        .selection
        .as_ref()
        .expect("overlay spec edit must select its capability");
    assert_eq!(selection.selected, vec!["compiler"]);
    assert_eq!(output.summary.passed, 2, "deployed + staged contract ran");
}
