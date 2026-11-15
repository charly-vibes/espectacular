//! Drift guard between the lint check registry and the explain topics.
//!
//! Purpose: keep `ah lint` check kinds and `ah explain` topics in lockstep.
//!
//! Responsibilities:
//! - resolve every registry check kind to a topic with the same slug
//! - keep the `lint` playbook topic and the vague-qualifier envelope intact
//!
//! Rationale: the registry is the source of truth for lint kinds; a new check
//! without a topic must fail here rather than surface as an unknown-topic 404.

use super::registry::lookup;

// espectacular-eia (task 6.1): one explain topic per lint finding kind.

#[test]
fn every_lint_check_kind_has_an_explain_topic() {
    // Drift guard: the live lint registry is the source of truth. Any
    // check kind added to the registry without an explain topic fails
    // here (and vice versa is enforced by the LintCheckKind exhaustive
    // match).
    let checks = crate::lint::select_checks(&crate::config::LintConfig::default(), None).unwrap();
    for kind in checks.iter().map(|c| c.kind()) {
        let topic = lookup(kind);
        assert!(topic.is_some(), "lint kind '{kind}' has no explain topic");
        assert_eq!(topic.unwrap().slug, kind);
    }
}

#[test]
fn lint_bridge_playbook_topic_exists() {
    // Bridge findings emit playbook_command "ah explain lint".
    assert!(lookup("lint").is_some(), "the 'lint' topic is missing");
}

#[test]
fn explain_vague_qualifier_returns_topic_envelope() {
    let entry = lookup("vague-qualifier").expect("vague-qualifier topic");
    assert!(entry.summary.contains("qualitative"));
    assert_eq!(entry.slug, "vague-qualifier");
}
