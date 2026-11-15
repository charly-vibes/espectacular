//! Topic bodies for `ah explain`.
//!
//! Purpose: hold every `TopicEntry` static, grouped by origin.
//!
//! Responsibilities:
//! - `lint`: spec-linter finding topics
//! - `findings_a` / `findings_b`: `ah check` finding kinds
//! - `actions`: `suggested_action` guidance
//! - `general`: workflow, archetypes, supersession, quality themes
//! - `adapters`: adapter detection and configuration
//!
//! Rationale: the bodies are prose-heavy and stable; grouping them by origin
//! keeps each file small while `kinds` keeps the coverage table compile-time
//! exhaustive.

mod actions;
mod adapters;
mod findings_a;
mod findings_b;
mod general;
mod lint;

pub(super) use actions::*;
pub(super) use adapters::*;
pub(super) use findings_a::*;
pub(super) use findings_b::*;
pub(super) use general::*;
pub(super) use lint::*;
