use std::fs;
use std::path::Path;

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

    for spec_entry in fs::read_dir(specs_path)? {
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
    let mut scenarios = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, &line) in lines.iter().enumerate() {
        if let Some(heading) = extract_scenario_heading(line) {
            scenarios.push(Scenario {
                id: slugify(heading),
                heading: heading.to_string(),
                spec_path: spec_name.to_string(),
                source_line: i + 1,
                body: extract_body(&lines, i + 1),
            });
        }
    }
    scenarios
}

/// Returns (spec_path, slug_id, first_heading) for each id that appears more than once.
pub fn detect_slug_collisions(scenarios: &[Scenario]) -> Vec<(String, String, String)> {
    use std::collections::HashMap;
    let mut seen: HashMap<(&str, &str), &str> = HashMap::new();
    let mut collisions = Vec::new();

    for s in scenarios {
        let key = (s.spec_path.as_str(), s.id.as_str());
        if let Some(first_heading) = seen.get(&key) {
            collisions.push((s.spec_path.clone(), s.id.clone(), first_heading.to_string()));
        } else {
            seen.insert(key, &s.heading);
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
        assert_eq!(collisions.len(), 1);
    }

    #[test]
    fn collision_tuple_contains_spec_and_id() {
        let scenarios = discover_scenarios(COLLISION_FIXTURE).unwrap();
        let collisions = detect_slug_collisions(&scenarios);
        let (spec, id, _heading) = &collisions[0];
        assert_eq!(spec, "compiler");
        assert_eq!(id, "empty-input-rejected");
    }
}
