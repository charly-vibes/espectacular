use std::fs;
use std::path::Path;

use anyhow::Context;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Scenario {
    pub id: String,
    pub heading: String,
    pub spec_path: String,
    pub source_line: usize,
    pub body: String,
}

pub fn slugify(heading: &str) -> String {
    let mut slug = String::new();
    let mut last_was_sep = true;
    for ch in heading.chars() {
        if ch.is_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('-');
            last_was_sep = true;
        }
    }
    slug.trim_end_matches('-').to_string()
}

pub fn discover_scenarios(specs_dir: &str) -> anyhow::Result<Vec<Scenario>> {
    let mut scenarios = Vec::new();
    let specs_path = Path::new(specs_dir);

    for spec_entry in fs::read_dir(specs_path)
        .with_context(|| format!("reading specs directory `{}`", specs_path.display()))?
    {
        let spec_entry = spec_entry?;
        let spec_name = spec_entry.file_name().to_string_lossy().into_owned();
        let spec_file = spec_entry.path().join("spec.md");
        if !spec_file.exists() {
            continue;
        }

        let content = fs::read_to_string(&spec_file)?;
        scenarios.extend(parse_scenarios_from_spec(&content, &spec_name));
    }

    Ok(scenarios)
}

fn extract_scenario_heading(line: &str) -> Option<&str> {
    line.strip_prefix("#### Scenario: ").map(str::trim)
}

fn extract_body(lines: &[&str], after: usize) -> String {
    let mut body_lines: Vec<&str> = lines[after..]
        .iter()
        .copied()
        .take_while(|l| !l.starts_with('#'))
        .collect();
    while body_lines.last().map(|l: &&str| l.trim().is_empty()) == Some(true) {
        body_lines.pop();
    }
    body_lines.join("\n")
}

fn parse_scenarios_from_spec(content: &str, spec_name: &str) -> Vec<Scenario> {
    use std::collections::HashSet;

    let mut scenarios = Vec::new();
    // Section-sync mirrors (same slugified id AND identical body, e.g. the
    // `## ADDED Requirements` / `## Requirements` pair in a dual-format delta)
    // are one logical scenario. Same id with a different body still passes
    // through and collides in detect_slug_collisions.
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, &line) in lines.iter().enumerate() {
        if let Some(heading) = extract_scenario_heading(line) {
            let id = slugify(heading);
            let body = extract_body(&lines, i + 1);
            if !seen.insert((id.clone(), body.clone())) {
                continue;
            }
            scenarios.push(Scenario {
                id,
                heading: heading.to_string(),
                spec_path: spec_name.to_string(),
                source_line: i + 1,
                body,
            });
        }
    }
    scenarios
}

/// Returns (spec_path, slug_id, first_heading) for each scenario whose slug appears more than once.
/// Unlike the original implementation, this includes ALL scenarios sharing a colliding slug,
/// not just the 2nd, 3rd, ... duplicates.
pub fn detect_slug_collisions(scenarios: &[Scenario]) -> Vec<(String, String, String)> {
    use std::collections::{HashMap, HashSet};

    // First pass: count occurrences per (spec_path, id) key
    let mut counts: HashMap<(&str, &str), usize> = HashMap::new();
    for s in scenarios {
        let key = (s.spec_path.as_str(), s.id.as_str());
        *counts.entry(key).or_insert(0) += 1;
    }

    // Build set of keys that have collisions (count > 1)
    let colliding_keys: HashSet<(&str, &str)> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(key, _)| key)
        .collect();

    // Collect all scenarios whose key is in the collision set
    let mut collisions = Vec::new();
    for s in scenarios {
        let key = (s.spec_path.as_str(), s.id.as_str());
        if colliding_keys.contains(&key) {
            collisions.push((s.spec_path.clone(), s.id.clone(), s.heading.clone()));
        }
    }

    collisions.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    collisions
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "tests/fixtures/simple/openspec/specs";
    const COLLISION_FIXTURE: &str = "tests/fixtures/collision/openspec/specs";

    #[test]
    fn missing_dir_error_names_the_path() {
        // espectacular-iq4 / GH#28.2: a bare "No such file or directory (os
        // error 2)" cost the reporter an hour — any IO failure from this
        // function must name the directory it was reading.
        let err = discover_scenarios("/nonexistent/ah-iq4-specs").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("/nonexistent/ah-iq4-specs"),
            "error must name the specs dir; got: {msg}"
        );
    }

    #[test]
    fn discovers_scenarios_from_headings() {
        let scenarios = discover_scenarios(FIXTURE).unwrap();
        assert_eq!(scenarios.len(), 2);
    }

    #[test]
    fn scenario_id_is_slugified_heading() {
        let scenarios = discover_scenarios(FIXTURE).unwrap();
        let ids: Vec<&str> = scenarios.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"empty-input-rejected"));
        assert!(ids.contains(&"null-bytes-rejected"));
    }

    #[test]
    fn scenario_heading_preserved_verbatim() {
        let scenarios = discover_scenarios(FIXTURE).unwrap();
        let found = scenarios
            .iter()
            .find(|s| s.id == "empty-input-rejected")
            .unwrap();
        assert_eq!(found.heading, "Empty input rejected");
    }

    #[test]
    fn scenario_spec_path_is_relative_spec_name() {
        let scenarios = discover_scenarios(FIXTURE).unwrap();
        let found = scenarios
            .iter()
            .find(|s| s.id == "empty-input-rejected")
            .unwrap();
        assert_eq!(found.spec_path, "compiler");
    }

    #[test]
    fn scenario_source_line_is_one_based() {
        let scenarios = discover_scenarios(FIXTURE).unwrap();
        let found = scenarios
            .iter()
            .find(|s| s.id == "empty-input-rejected")
            .unwrap();
        // heading is at line 7 in the fixture
        assert_eq!(found.source_line, 7);
    }

    #[test]
    fn scenario_body_contains_given_when_then() {
        let scenarios = discover_scenarios(FIXTURE).unwrap();
        let found = scenarios
            .iter()
            .find(|s| s.id == "empty-input-rejected")
            .unwrap();
        assert!(found.body.contains("GIVEN"));
        assert!(found.body.contains("WHEN"));
        assert!(found.body.contains("THEN"));
    }

    #[test]
    fn slugify_lowercases_and_hyphens() {
        assert_eq!(slugify("Empty input rejected"), "empty-input-rejected");
    }

    #[test]
    fn slugify_strips_non_alphanumeric() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
    }

    #[test]
    fn slugify_collapses_repeated_separators() {
        assert_eq!(slugify("foo  --  bar"), "foo-bar");
    }

    #[test]
    fn slugify_trims_leading_trailing_separators() {
        assert_eq!(slugify(" -- foo -- "), "foo");
    }

    // 1.4 RED: collision detection
    #[test]
    fn no_collisions_in_clean_fixture() {
        let scenarios = discover_scenarios(FIXTURE).unwrap();
        let collisions = detect_slug_collisions(&scenarios);
        assert!(collisions.is_empty());
    }

    #[test]
    fn detects_slug_collision() {
        let scenarios = discover_scenarios(COLLISION_FIXTURE).unwrap();
        let collisions = detect_slug_collisions(&scenarios);
        // Both scenarios share the same slug, so both are flagged
        assert_eq!(collisions.len(), 2);
    }

    #[test]
    fn collision_tuple_contains_spec_and_id() {
        let scenarios = discover_scenarios(COLLISION_FIXTURE).unwrap();
        let collisions = detect_slug_collisions(&scenarios);
        let (spec, id, _heading) = &collisions[0];
        assert_eq!(spec, "compiler");
        assert_eq!(id, "empty-input-rejected");
    }

    // adopt-dual-format-specs 1.1 RED: mirror deduplication
    #[test]
    fn mirror_identical_id_and_body_yields_one_scenario() {
        let content = "\
## ADDED Requirements

### Requirement: Sample
#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected

## Requirements

### Requirement: Sample
#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected
";
        let scenarios = parse_scenarios_from_spec(content, "compiler");
        assert_eq!(
            scenarios.len(),
            1,
            "mirrored identical (id, body) must dedupe"
        );
        assert_eq!(scenarios[0].id, "empty-input-rejected");
    }

    #[test]
    fn mirror_dedupe_keeps_first_occurrence_source_line() {
        let content = "\
## ADDED Requirements

#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected

## Requirements

#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected
";
        let scenarios = parse_scenarios_from_spec(content, "compiler");
        assert_eq!(scenarios.len(), 1);
        // heading is on line 3 (first occurrence)
        assert_eq!(scenarios[0].source_line, 3);
    }

    #[test]
    fn same_id_different_body_still_collides() {
        let content = "\
#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected

#### Scenario: Empty input rejected
- **GIVEN** null bytes
- **WHEN** submitted
- **THEN** rejected
";
        let scenarios = parse_scenarios_from_spec(content, "compiler");
        assert_eq!(scenarios.len(), 2, "distinct bodies are distinct scenarios");
        let collisions = detect_slug_collisions(&scenarios);
        assert_eq!(
            collisions.len(),
            2,
            "both sides of a distinct-body collision are flagged"
        );
    }

    #[test]
    fn frontmatter_and_table_rows_never_yield_scenarios() {
        let content = "\
---
id: spec
statement: \"#### Scenario: not-a-scenario\"
---

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C1 | invariant | #### Scenario: table-row-fake | |

## Requirements

#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected
";
        let scenarios = parse_scenarios_from_spec(content, "compiler");
        let ids: Vec<&str> = scenarios.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["empty-input-rejected"],
            "only real headings count"
        );
    }

    #[test]
    fn plain_openspec_discovery_unchanged() {
        // P-optin: a file with no frontmatter and no specodelic tables parses
        // exactly as before (both scenarios discovered).
        let content = "\
## Requirements

#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected

#### Scenario: Null bytes rejected
- **GIVEN** null bytes
- **WHEN** submitted
- **THEN** rejected
";
        let scenarios = parse_scenarios_from_spec(content, "compiler");
        assert_eq!(scenarios.len(), 2);
    }
}
