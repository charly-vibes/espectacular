//! Data shapes for `ah explain`.
//!
//! Purpose: define the static topic record rendered by the CLI and its JSON
//! projection.
//!
//! Responsibilities:
//! - `TopicEntry` / `Hint`: topic bodies and their advisory hints
//! - `TopicJson` / `HintJson`: the `--json` envelope
//!
//! Rationale: the JSON shape is serialized verbatim by `run`; keeping it
//! apart from the body registry makes the wire format auditable on its own.

use serde::Serialize;

pub struct TopicEntry {
    pub slug: &'static str,
    pub summary: &'static str,
    pub body: &'static str,
    pub when: &'static str,
    pub do_action: &'static str,
    pub human_approval: bool,
    pub related_topics: &'static [&'static str],
    pub hints: &'static [Hint],
}

pub struct Hint {
    pub kind: &'static str,
    pub message: &'static str,
}

#[derive(Serialize)]
pub struct TopicJson {
    pub topic: &'static str,
    pub summary: &'static str,
    pub when: &'static str,
    #[serde(rename = "do")]
    pub do_action: &'static str,
    pub human_approval: bool,
    pub related_topics: &'static [&'static str],
    pub hints: Vec<HintJson>,
}

#[derive(Serialize)]
pub struct HintJson {
    pub kind: &'static str,
    pub message: &'static str,
}
