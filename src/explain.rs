//! Explain topics (`ah explain`).
//!
//! Purpose: give readers (and agents) guidance for every finding kind,
//! suggested action, lint check, adapter, and workflow theme `ah` emits.
//!
//! Responsibilities:
//! - `model`: the topic record and its JSON projection
//! - `kinds`: the compile-enforced variant-to-topic coverage table
//! - `topics/*`: the topic bodies themselves
//! - `registry`: the single sorted topic list, lookup, and typo suggestions
//! - `run`: the `ah explain` CLI output shapes
//!
//! Rationale: coverage must be a compile-time property (an exhaustive match),
//! while the bodies are prose that changes often — splitting them keeps the
//! coverage table reviewable and every file small.

mod kinds;
mod model;
mod registry;
mod run;
mod topics;

pub use run::*;

#[cfg(test)]
mod lint_topics_tests;
#[cfg(test)]
mod tests;
