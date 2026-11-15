//! `ah sync` — create or refresh property-derived contracts (task group 3 of
//! derive-contracts-from-specodelic).
//!
//! Purpose: bridge the specodelic half (Properties rows, read via
//! `spk parse --json`) and the gate (contract TOMLs). Refuses to derive from
//! a file whose `spk lint` is not clean or whose `spk` binary is missing
//! (C-sync-refusal). `--check` writes nothing and reports drift/missing
//! contracts for CI (C-sync-check).
//!
//! Field ownership (design D3): sync owns `id`, `description`, `archetype`,
//! `falsifiability_class`, `derived_from`; humans own `tests`, `status`,
//! `superseded_by` (and `authored_with` provenance, preserved on refresh).
//!
//! Responsibilities:
//! - define the `SyncOutcome` result shape the CLI maps to an exit code
//! - own the property-id → contract-id slug rule
//! - delegate the run (lint refusal, derive, check/write) to [`run`]
//!
//! Rationale: the run itself is staged (collect lint-clean specs, derive the
//! table, then check or write each contract) and lives in a submodule so each
//! stage stays reviewable in isolation.

mod run;

#[cfg(test)]
mod tests;

pub(crate) use run::run_sync;

/// Result of a sync run; the CLI layer maps it to an exit code.
#[derive(Debug, Default, PartialEq, serde::Serialize)]
pub(crate) struct SyncOutcome {
    /// Contract paths created or refreshed (write mode only).
    pub wrote: Vec<String>,
    /// Contracts that already matched their source row (--check only).
    pub fresh: Vec<String>,
    /// Properties with no contract file (--check only).
    pub missing: Vec<String>,
    /// Contracts whose derived fields drifted from the source row (--check).
    pub stale: Vec<String>,
    /// Refusals: (spec file, reason) — lint-dirty or parse-broken files.
    pub refusals: Vec<(String, String)>,
    /// Set when the spk binary is missing (C-sync-refusal, spk-unavailable).
    pub spk_unavailable: Option<String>,
}

impl SyncOutcome {
    pub(crate) fn is_ok(&self) -> bool {
        self.spk_unavailable.is_none()
            && self.refusals.is_empty()
            && self.missing.is_empty()
            && self.stale.is_empty()
    }
}

/// Lowercase and hyphenate a property id for use as a contract id.
pub(crate) fn slugify_property_id(id: &str) -> String {
    let slug: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let mut out = String::new();
    for c in slug.chars() {
        if c == '-' && out.ends_with('-') {
            continue;
        }
        out.push(c);
    }
    out.trim_matches('-').to_ascii_lowercase()
}
