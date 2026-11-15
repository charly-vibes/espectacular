use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Scenario {
    pub id: String,
    pub heading: String,
    pub spec_path: String,
    /// One-based source line — set by discovery and asserted by parse tests
    /// (ordering/dedupe diagnostics); no production read site yet.
    #[allow(dead_code)]
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

/// Extract the frontmatter `id:` value from a spec file's content, mirroring
/// specodelic's convention. Under Revision 18 the value is the parent-dir
/// derived real id (any value is accepted here — the naming law is spk's
/// `linter.id_matches_file` to enforce). None when there is no frontmatter
/// or no id line.
pub fn frontmatter_id(content: &str) -> Option<String> {
    let rest = content.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    rest[..end].lines().find_map(|l| {
        let value = l.trim().strip_prefix("id:")?.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// Frontmatter id per deployed spec (`<dir>/spec.md`), keyed by the spec
/// directory name. VERIFIES links inside a dual-format file are file-local
/// (`<own-file-id>.<property-id>`), so resolving them to property-derived
/// contracts requires knowing each file's own id.
pub fn frontmatter_ids(specs_dir: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let mut ids = BTreeMap::new();
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
        if let Some(id) = frontmatter_id(&content) {
            ids.insert(spec_name, id);
        }
    }
    Ok(ids)
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
mod tests;
