//! One RED→GREEN test per registry check kind: each asserts the kind fires on
//! the defective fixture and stays silent on the clean fixture's `auth` spec.
//!
//! These were `#[ignore]`d at tracer-bullet scope; each was un-ignored in the
//! ticket that implemented its check (rty: scenario-flow, aar: spec-shape).

use crate::lint::run_lint_query;
use crate::lint::LintFinding;
use std::path::PathBuf;

fn defective_specs() -> PathBuf {
    PathBuf::from("tests/fixtures/lint/defective/openspec/specs")
}

/// `(fires anywhere, fires on the clean `auth` spec)` for one check kind.
fn firing(kind: &str) -> (bool, bool) {
    let output = run_lint_query(&defective_specs(), None).unwrap();
    let matching: Vec<&LintFinding> = output.findings.iter().filter(|f| f.kind == kind).collect();
    let fires = !matching.is_empty();
    let on_auth = matching.iter().any(|f| f.spec_path == "auth");
    (fires, on_auth)
}

#[test]
fn vague_qualifier_fires_on_defective_fixture() {
    let (fires, on_auth) = firing("vague-qualifier");
    assert!(fires, "vague-qualifier must fire on the defective fixture");
    assert!(!on_auth, "vague-qualifier must stay silent on auth");
}

#[test]
fn imperative_step_fires_on_defective_fixture() {
    let (fires, on_auth) = firing("imperative-step");
    assert!(fires, "imperative-step must fire on the defective fixture");
    assert!(!on_auth, "imperative-step must stay silent on auth");
}

#[test]
fn conjunctive_bloat_fires_on_defective_fixture() {
    let (fires, on_auth) = firing("conjunctive-bloat");
    assert!(
        fires,
        "conjunctive-bloat must fire on the defective fixture"
    );
    assert!(!on_auth, "conjunctive-bloat must stay silent on auth");
}

#[test]
fn missing_negative_scenario_fires_on_defective_fixture() {
    let (fires, on_auth) = firing("missing-negative-scenario");
    assert!(
        fires,
        "missing-negative-scenario must fire on the defective fixture"
    );
    assert!(
        !on_auth,
        "missing-negative-scenario must stay silent on auth"
    );
}

#[test]
fn missing_non_goals_fires_on_defective_fixture() {
    let (fires, on_auth) = firing("missing-non-goals");
    assert!(
        fires,
        "missing-non-goals must fire on the defective fixture"
    );
    assert!(!on_auth, "missing-non-goals must stay silent on auth");
}

#[test]
fn unresolved_ambiguity_fires_on_defective_fixture() {
    let (fires, on_auth) = firing("unresolved-ambiguity");
    assert!(
        fires,
        "unresolved-ambiguity must fire on the defective fixture"
    );
    assert!(!on_auth, "unresolved-ambiguity must stay silent on auth");
}

#[test]
fn entangled_spec_fires_on_defective_fixture() {
    let (fires, on_auth) = firing("entangled-spec");
    assert!(fires, "entangled-spec must fire on the defective fixture");
    assert!(!on_auth, "entangled-spec must stay silent on auth");
}
