//! CLI output for `ah explain`.
//!
//! Purpose: render the topic registry as a listing, a verbose body, or a JSON
//! envelope, and fail with a helpful message on missing or unknown topics.
//!
//! Responsibilities:
//! - `--list` output: one sorted slug per line
//! - `<topic>` output: the plain body, or the JSON envelope under `--json`
//! - usage text and typo suggestions on missing/unknown topics
//!
//! Rationale: the output shapes are asserted by CLI tests, so all printing
//! lives here while the registry stays a pure data source.

use super::model::{HintJson, TopicEntry, TopicJson};
use super::registry::{all_topics, did_you_mean, lookup};

fn all_slugs() -> Vec<&'static str> {
    all_topics().into_iter().map(|t| t.slug).collect()
}

fn print_listing() {
    let mut slugs = all_slugs();
    slugs.sort_unstable();
    for slug in slugs {
        println!("{slug}");
    }
}

fn print_usage_and_exit() -> ! {
    eprintln!("usage: ah explain <topic>");
    eprintln!("topics: {}", all_slugs().join(", "));
    std::process::exit(1);
}

fn print_unknown_topic_and_exit(slug: &str) -> ! {
    let suggestions = did_you_mean(slug);
    if suggestions.is_empty() {
        eprintln!("unknown topic: {slug}");
    } else {
        eprintln!(
            "unknown topic: {slug}. Did you mean: {}?",
            suggestions.join(", ")
        );
    }
    std::process::exit(1);
}

fn topic_json(entry: &'static TopicEntry) -> TopicJson {
    TopicJson {
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
    }
}

fn print_json(entry: &'static TopicEntry) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(&topic_json(entry))?);
    Ok(())
}

pub fn run_explain(topic: Option<&str>, list: bool, json: bool) -> anyhow::Result<()> {
    if list {
        print_listing();
        return Ok(());
    }

    let Some(slug) = topic else {
        print_usage_and_exit()
    };

    let Some(entry) = lookup(slug) else {
        print_unknown_topic_and_exit(slug)
    };

    if json {
        print_json(entry)?;
    } else {
        println!("{}", entry.body);
    }

    Ok(())
}
