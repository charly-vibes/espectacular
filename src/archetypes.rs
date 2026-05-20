const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Archetype {
    pub code: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub body: &'static str,
    pub since: &'static str,
}

const CATALOG: &[Archetype] = &[
    Archetype {
        code: "PF",
        name: "Pure Functional",
        summary: "Deterministic behavior where outputs are a function of explicit inputs.",
        body: "## PF — Pure Functional

Deterministic behavior where outputs are a function of explicit inputs.

Use for:
- parsers
- formatters
- validators
- pure transformations
- deterministic calculations

Typical test shapes:
- unit examples for representative inputs
- property-based tests for invariants
- boundary input examples",
        since: "0.0.9",
    },
    Archetype {
        code: "SA",
        name: "Stateful API",
        summary: "Behavior involving state transitions, persisted data, or ordered operations.",
        body: "## SA — Stateful API

Behavior involving state transitions, persisted data, or ordered operations.

Use for:
- create/update/delete flows
- session state
- caches
- workflow state machines
- idempotency rules

Typical test shapes:
- unit or integration tests for state before/after
- repeated-operation tests
- invalid transition tests",
        since: "0.0.9",
    },
    Archetype {
        code: "BP",
        name: "Boundary Protocol",
        summary: "Behavior at an external boundary or protocol seam.",
        body: "## BP — Boundary Protocol

Behavior at an external boundary or protocol seam.

Use for:
- HTTP APIs
- CLI invocation
- filesystem effects
- network calls
- serialization contracts

Typical test shapes:
- shell tests for command behavior
- integration tests against test doubles
- golden input/output fixtures",
        since: "0.0.9",
    },
    Archetype {
        code: "CE",
        name: "Contract/Event",
        summary: "Behavior expressed as emitted events, messages, claims, or cross-tool signals.",
        body: "## CE — Contract/Event

Behavior expressed as emitted events, messages, claims, or cross-tool signals.

Use for:
- structured JSON outputs
- event logs
- report formats
- machine-readable diagnostics
- integration payloads

Typical test shapes:
- schema validation tests
- golden JSON comparisons
- event presence/absence checks",
        since: "0.0.9",
    },
    Archetype {
        code: "NR",
        name: "Non-Regression",
        summary: "Behavior asserting existing guarantees remain true while nearby changes land.",
        body: "## NR — Non-Regression

Behavior asserting existing guarantees remain true while nearby changes land.

Use for:
- bug-fix guardrails
- refactors that must preserve behavior
- compatibility promises
- migration safety checks
- regression coverage for unchanged contracts

Typical test shapes:
- existing regression tests replayed in change scope
- before/after fixture comparisons
- smoke tests that lock existing observable behavior",
        since: "0.1.0",
    },
];

pub fn list_archetypes() -> String {
    list_archetypes_for_version(TOOL_VERSION)
}

pub fn list_archetypes_for_version(version: &str) -> String {
    catalog_for_version(version)
        .iter()
        .map(|a| format!("{} — {}: {}", a.code, a.name, a.summary))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn lookup(code: &str) -> Option<&'static Archetype> {
    lookup_for_version(code, TOOL_VERSION)
}

pub fn lookup_for_version(code: &str, version: &str) -> Option<&'static Archetype> {
    let normalized = normalize(code);
    catalog_for_version(version)
        .into_iter()
        .find(|a| a.code == normalized)
}

pub fn did_you_mean(input: &str) -> Vec<&'static str> {
    did_you_mean_for_version(input, TOOL_VERSION)
}

pub fn did_you_mean_for_version(input: &str, version: &str) -> Vec<&'static str> {
    let normalized = normalize(input);
    catalog_for_version(version)
        .into_iter()
        .filter(|a| a.code.contains(&normalized) || normalized.contains(a.code))
        .map(|a| a.code)
        .collect()
}

pub fn known_codes() -> Vec<&'static str> {
    known_codes_for_version(TOOL_VERSION)
}

pub fn known_codes_for_version(version: &str) -> Vec<&'static str> {
    catalog_for_version(version)
        .into_iter()
        .map(|a| a.code)
        .collect()
}

pub fn is_known(code: &str) -> bool {
    lookup(code).is_some()
}

fn catalog_for_version(version: &str) -> Vec<&'static Archetype> {
    CATALOG
        .iter()
        .filter(|a| version_at_least(version, a.since))
        .collect()
}

fn normalize(code: &str) -> String {
    code.trim().to_uppercase()
}

fn version_at_least(version: &str, since: &str) -> bool {
    parse_version(version) >= parse_version(since)
}

fn parse_version(version: &str) -> (u64, u64, u64) {
    let mut parts = version
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_catalog_includes_nr() {
        assert!(known_codes().contains(&"NR"));
    }

    #[test]
    fn older_versions_hide_newer_archetypes() {
        assert!(!known_codes_for_version("0.0.9").contains(&"NR"));
        assert!(known_codes_for_version("0.1.0").contains(&"NR"));
    }

    #[test]
    fn lookup_is_case_insensitive() {
        let archetype = lookup("pf").unwrap();
        assert_eq!(archetype.code, "PF");
    }

    #[test]
    fn suggestions_respect_version_catalog() {
        assert!(did_you_mean_for_version("n", "0.0.9").is_empty());
        assert_eq!(did_you_mean_for_version("n", "0.1.0"), vec!["NR"]);
    }
}
