//! Topic registry and lookup for `ah explain`.
//!
//! Purpose: assemble the single sorted topic list that `ah explain` lists,
//! looks up, and suggests from.
//!
//! Responsibilities:
//! - `GENERAL_TOPICS` / `ADAPTER_TOPICS`: hand-maintained topic groups
//! - `all_topics`: union of the compile-enforced coverage tables
//! - `lookup` / `did_you_mean`: slug resolution and typo suggestions
//!
//! Rationale: every consumer sees one sorted registry, so the topic order is
//! defined in exactly one place.

use super::kinds::*;
use super::model::TopicEntry;
use super::topics::*;

const GENERAL_TOPICS: &[&TopicEntry] = &[
    &WORKFLOW,
    &SUPERSESSION,
    &ARCHETYPES,
    &PROGRESSIVE_ENABLEMENT,
    &LINT,
    &SCENARIO_SCOPED_TESTS,
];

const ADAPTER_TOPICS: &[&TopicEntry] = &[&ADAPTER_PYTEST, &ADAPTER_CARGO, &ADAPTER_VITEST];

pub fn all_topics() -> Vec<&'static TopicEntry> {
    let mut entries: Vec<&'static TopicEntry> = vec![];
    for kind in ALL_FINDING_KINDS {
        entries.push(finding_kind_entry(*kind));
    }
    for action in ALL_SUGGESTED_ACTIONS {
        entries.push(suggested_action_entry(*action));
    }
    entries.extend(GENERAL_TOPICS);
    entries.extend(ADAPTER_TOPICS);
    for kind in ALL_LINT_KINDS {
        entries.push(lint_kind_entry(*kind));
    }
    entries.sort_by_key(|t| t.slug);
    entries
}

pub fn lookup(slug: &str) -> Option<&'static TopicEntry> {
    all_topics().into_iter().find(|t| t.slug == slug)
}

pub fn did_you_mean(input: &str) -> Vec<&'static str> {
    all_topics()
        .into_iter()
        .filter(|t| t.slug.contains(input) || input.contains(t.slug))
        .map(|t| t.slug)
        .collect()
}
