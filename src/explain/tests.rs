//! Registry invariants for the explain topics.
//!
//! Purpose: prove that the topic registry is complete, sorted, unique, and
//! JSON-serializable.
//!
//! Responsibilities:
//! - coverage: every finding kind and suggested action has a non-empty body
//! - shape: JSON fields present, hints well-formed, listing sorted
//! - property-derived bindings (`p_*`) exercised by the contract tests
//!
//! Rationale: the compile-time match proves a body exists; these tests prove
//! the bodies are usable, listed once each, and reachable by slug.

use super::kinds::*;
use super::model::{HintJson, TopicJson};
use super::registry::{all_topics, did_you_mean, lookup};

// 9.3 — every FindingKind variant has a topic body
#[test]
fn every_finding_kind_has_a_topic() {
    for kind in ALL_FINDING_KINDS {
        let entry = finding_kind_entry(*kind);
        assert!(
            !entry.slug.is_empty(),
            "FindingKind {:?} has empty slug",
            kind
        );
        assert!(
            !entry.body.is_empty(),
            "FindingKind {:?} has empty body",
            kind
        );
    }
}

// 9.3 — every SuggestedAction variant has a topic body
#[test]
fn every_suggested_action_has_a_topic() {
    for action in ALL_SUGGESTED_ACTIONS {
        let entry = suggested_action_entry(*action);
        assert!(
            !entry.slug.is_empty(),
            "SuggestedAction {:?} has empty slug",
            action
        );
        assert!(
            !entry.body.is_empty(),
            "SuggestedAction {:?} has empty body",
            action
        );
    }
}

// 9.3 — quality finding kinds present
#[test]
fn quality_finding_kinds_present() {
    assert!(lookup("quality-mutation").is_some());
    assert!(lookup("quality-property").is_some());
    assert!(lookup("quality-snapshot").is_some());
}

// 9.5 — general topics present
#[test]
fn general_topics_present() {
    assert!(lookup("workflow").is_some());
    assert!(lookup("supersession").is_some());
    assert!(lookup("archetypes").is_some());
    assert!(lookup("progressive-enablement").is_some());
}

// derive-contracts-from-specodelic tasks 4.2/4.4 — new topics resolve
#[test]
fn derived_contract_topics_present() {
    for slug in [
        "contract-stale",
        "run_ah_sync",
        "spk-frontmatter-mismatch",
        "verifies-dangling",
        "scenario-unlinked",
        "property-untraced",
    ] {
        let entry = lookup(slug).unwrap_or_else(|| panic!("missing topic: {slug}"));
        assert_eq!(entry.slug, slug);
        assert!(!entry.body.is_empty());
    }
}

// 9.5 — general topic bodies are non-empty
#[test]
fn general_topic_bodies_non_empty() {
    for slug in &[
        "workflow",
        "supersession",
        "archetypes",
        "progressive-enablement",
    ] {
        let entry = lookup(slug).unwrap_or_else(|| panic!("missing general topic: {slug}"));
        assert!(
            !entry.body.is_empty(),
            "general topic {slug} has empty body"
        );
    }
}

// 9.7 — --json fields present on all topics
#[test]
fn json_fields_present_on_all_topics() {
    for entry in all_topics() {
        let output = TopicJson {
            topic: entry.slug,
            summary: entry.summary,
            when: entry.when,
            do_action: entry.do_action,
            human_approval: entry.human_approval,
            related_topics: entry.related_topics,
            hints: entry
                .hints
                .iter()
                .map(|h| HintJson {
                    kind: h.kind,
                    message: h.message,
                })
                .collect(),
        };
        let json = serde_json::to_string(&output).expect("serialization failed");
        assert!(json.contains("\"topic\""));
        assert!(json.contains("\"summary\""));
        assert!(json.contains("\"when\""));
        assert!(json.contains("\"do\""));
        assert!(json.contains("\"hints\""));
    }
}

// 9.9 — --list returns stable sorted output
#[test]
fn list_is_sorted() {
    let topics = all_topics();
    let slugs: Vec<&str> = topics.iter().map(|t| t.slug).collect();
    let mut sorted = slugs.clone();
    sorted.sort_unstable();
    assert_eq!(slugs, sorted, "all_topics() is not sorted");
}

// 9.9 — unknown topic returns None
#[test]
fn unknown_topic_returns_none() {
    assert!(lookup("totally-unknown-xyzzy").is_none());
}

// 9.9 — did_you_mean returns suggestions for partial match
#[test]
fn did_you_mean_returns_suggestions() {
    let suggestions = did_you_mean("test");
    assert!(!suggestions.is_empty(), "expected suggestions for 'test'");
    assert!(suggestions.contains(&"test-failing"));
}

// 9.11 — adapter topics present
#[test]
fn adapter_topics_present() {
    assert!(lookup("adapter-pytest").is_some());
    assert!(lookup("adapter-cargo").is_some());
    assert!(lookup("adapter-vitest").is_some());
}

// 9.11 — adapter topic bodies are non-empty
#[test]
fn adapter_topic_bodies_non_empty() {
    for slug in &["adapter-pytest", "adapter-cargo", "adapter-vitest"] {
        let entry = lookup(slug).unwrap_or_else(|| panic!("missing adapter topic: {slug}"));
        assert!(!entry.body.is_empty());
    }
}

// 9.13 — no duplicate slugs in registry
#[test]
fn no_duplicate_slugs() {
    let topics = all_topics();
    let mut seen = std::collections::HashSet::new();
    for entry in &topics {
        assert!(
            seen.insert(entry.slug),
            "duplicate topic slug: {}",
            entry.slug
        );
    }
}

// 9.13 — topic count matches expected
#[test]
fn topic_count_is_complete() {
    let topics = all_topics();
    // 20 finding kinds + 7 suggested actions + 5 general (incl. lint)
    // + 3 adapter + 7 lint kinds (espectacular-eia) = 42; +2 finding/action
    // + 2 lint kinds from derive-contracts-from-specodelic (4.2/4.4)
    // + 2 trace kinds from 5.3 = 48; +1 scenario-scoped-tests general
    // topic (espectacular-ncu, GH#32) = 49; +1 batch-fallback finding
    // kind (batch-runner-spawns task 2.2) = 50
    assert_eq!(
        topics.len(),
        50,
        "expected 50 topics (22 finding + 8 action + 6 general + 3 adapter + 11 lint kinds), got {}",
        topics.len()
    );
}

// hint shape
#[test]
fn hints_have_kind_and_message() {
    for entry in all_topics() {
        for hint in entry.hints {
            assert!(!hint.kind.is_empty());
            assert!(!hint.message.is_empty());
        }
    }
}

// ---- property-derived contract bindings (DDL migration, design D6) ----

#[test]
fn p_enforced() {
    every_finding_kind_has_a_topic();
    every_suggested_action_has_a_topic();
    no_duplicate_slugs();
}

#[test]
fn p_coverage() {
    every_finding_kind_has_a_topic();
    every_suggested_action_has_a_topic();
    general_topics_present();
    topic_count_is_complete();
}

#[test]
fn p_json() {
    json_fields_present_on_all_topics();
}

#[test]
fn p_listing() {
    list_is_sorted();
    topic_count_is_complete();
}

#[test]
fn p_unknown() {
    unknown_topic_returns_none();
    did_you_mean_returns_suggestions();
}

#[test]
fn p_quality() {
    quality_finding_kinds_present();
}

#[test]
fn p_adapter() {
    adapter_topics_present();
    adapter_topic_bodies_non_empty();
}
