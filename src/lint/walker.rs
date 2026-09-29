//! Spec walker for `ah lint`.
//!
//! Purpose: statically walk deployed OpenSpec spec files and yield each
//! spec file, its requirements, and every `#### Scenario:` block together
//! with its parent requirement — the context all lint checks consume.
//!
//! Responsibilities:
//! - discover spec files under a specs root (`<capability>/spec.md`)
//! - parse `### Requirement:` sections and their scenario children
//! - provide `SpecFile` units to the lint engine's check registry
//!
//! Rationale: checks (espectacular-rty/aar) must see scenario AND parent
//! requirement context (e.g. missing-negative-scenario is requirement-level,
//! imperative-step is scenario-level), so a single walker shapes that context.

use crate::openspec::slugify;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// One parsed spec file: a capability with its requirement units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecFile {
    /// Capability directory name, e.g. `"auth"` (mirrors `Scenario::spec_path`).
    pub spec_path: String,
    pub requirements: Vec<RequirementUnit>,
}

/// A `### Requirement:` block and every scenario declared under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementUnit {
    /// Requirement heading text after the `### Requirement: ` prefix.
    pub heading: String,
    /// Prose between the requirement heading and its first scenario/next heading.
    pub body: String,
    pub scenarios: Vec<ScenarioBlock>,
}

/// A `#### Scenario:` block with its parent linkage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioBlock {
    /// Slugified scenario id (same convention as `openspec::discover_scenarios`).
    pub id: String,
    pub heading: String,
    /// Bullet lines under the scenario heading (WHEN/THEN/AND steps).
    pub body: String,
    /// 1-based line number of the scenario heading.
    pub source_line: usize,
}

/// Walk every `<specs_dir>/<capability>/spec.md` and parse it into units.
pub fn walk_specs(specs_dir: &Path) -> anyhow::Result<Vec<SpecFile>> {
    let mut specs = Vec::new();
    for entry in fs::read_dir(specs_dir)? {
        let entry = entry?;
        let spec_name = entry.file_name().to_string_lossy().into_owned();
        let spec_file = entry.path().join("spec.md");
        if !spec_file.exists() {
            continue;
        }
        let content = fs::read_to_string(&spec_file)?;
        specs.push(parse_spec(&content, &spec_name));
    }
    specs.sort_by(|a, b| a.spec_path.cmp(&b.spec_path));
    Ok(specs)
}

/// Parse one spec file's content into units. `pub(crate)` so check modules
/// can unit-test against inline spec markdown.
pub(crate) fn parse_spec(content: &str, spec_name: &str) -> SpecFile {
    use std::collections::HashSet;

    let lines: Vec<&str> = content.lines().collect();
    let mut requirements: Vec<RequirementUnit> = Vec::new();
    let mut current_req: Option<usize> = None;
    // Section-sync mirrors (same slugified id AND identical body, e.g. the
    // `## ADDED Requirements` / `## Requirements` pair in a dual-format
    // delta) are one logical scenario — same dedupe convention as
    // openspec::parse_scenarios_from_spec (deployed gate semantics:
    // deduplicate-mirrored-delta-sections).
    let mut seen: HashSet<(String, String)> = HashSet::new();

    for (i, &line) in lines.iter().enumerate() {
        if let Some(heading) = line.strip_prefix("### Requirement: ") {
            requirements.push(RequirementUnit {
                heading: heading.trim().to_string(),
                body: String::new(),
                scenarios: Vec::new(),
            });
            current_req = Some(requirements.len() - 1);
            continue;
        }
        if let Some(heading) = line.strip_prefix("#### Scenario: ") {
            let heading = heading.trim();
            let body = extract_body(&lines, i + 1);
            if !seen.insert((slugify(heading), body.clone())) {
                continue;
            }
            let scenario = ScenarioBlock {
                id: slugify(heading),
                heading: heading.to_string(),
                body,
                source_line: i + 1,
            };
            let idx = match current_req {
                Some(idx) => idx,
                None => {
                    // Scenario outside any requirement: attach to a
                    // synthetic parent so the walker never drops context.
                    requirements.push(RequirementUnit {
                        heading: String::new(),
                        body: String::new(),
                        scenarios: Vec::new(),
                    });
                    requirements.len() - 1
                }
            };
            requirements[idx].scenarios.push(scenario);
            continue;
        }
        // Prose between the requirement heading and its first scenario.
        if let Some(idx) = current_req {
            let req = &mut requirements[idx];
            if req.scenarios.is_empty()
                && !req.heading.is_empty()
                && !line.starts_with('#')
                && !line.trim().is_empty()
            {
                if !req.body.is_empty() {
                    req.body.push('\n');
                }
                req.body.push_str(line);
            }
        }
    }

    SpecFile {
        spec_path: spec_name.to_string(),
        requirements,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn clean_specs() -> PathBuf {
        PathBuf::from("tests/fixtures/lint/clean/openspec/specs")
    }

    fn defective_specs() -> PathBuf {
        PathBuf::from("tests/fixtures/lint/defective/openspec/specs")
    }

    #[test]
    fn walker_visits_every_scenario_with_parent_requirement() {
        let specs = walk_specs(&clean_specs()).unwrap();
        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(spec.spec_path, "auth");
        assert_eq!(spec.requirements.len(), 1);
        let req = &spec.requirements[0];
        assert_eq!(req.heading, "User Login");
        assert!(req.body.contains("valid credentials"));
        assert_eq!(req.scenarios.len(), 2);
        assert!(req.scenarios.iter().all(|s| !s.id.is_empty()));
        assert!(req.scenarios.iter().all(|s| s.source_line > 0));
        assert!(req.scenarios.iter().all(|s| !s.body.is_empty()));
    }

    #[test]
    fn walker_parses_every_fixture() {
        for dir in [clean_specs(), defective_specs()] {
            let specs = walk_specs(&dir).unwrap();
            assert!(!specs.is_empty(), "walker parsed nothing for {dir:?}");
            for spec in &specs {
                assert!(!spec.requirements.is_empty());
                for req in &spec.requirements {
                    for scenario in &req.scenarios {
                        assert_eq!(scenario.id, slugify(&scenario.heading));
                    }
                }
            }
        }
        let defective = walk_specs(&defective_specs()).unwrap();
        let scenario_count: usize = defective
            .iter()
            .flat_map(|s| &s.requirements)
            .map(|r| r.scenarios.len())
            .sum();
        assert_eq!(scenario_count, 2);
    }

    #[test]
    fn walker_dedupes_section_sync_mirrors() {
        // Dual-format deltas mirror scenarios across ## ADDED Requirements /
        // ## Requirements sections; same id + identical body is ONE logical
        // scenario (deployed gate semantics: deduplicate-mirrored-delta-
        // sections). The walker must not double-emit them.
        let specs = walk_specs(&PathBuf::from("tests/fixtures/lint/dual/openspec/specs")).unwrap();
        let scenario_count: usize = specs
            .iter()
            .flat_map(|s| &s.requirements)
            .map(|r| r.scenarios.len())
            .sum();
        assert_eq!(scenario_count, 2, "mirrored scenarios double-emitted");
    }
}
