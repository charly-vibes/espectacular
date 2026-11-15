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
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// One parsed spec file: a capability with its requirement units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecFile {
    /// Capability directory name, e.g. `"auth"` (mirrors `Scenario::spec_path`).
    pub spec_path: String,
    pub requirements: Vec<RequirementUnit>,
    /// Raw spec markdown — spec-file-level checks (missing-non-goals) scan
    /// headings outside any requirement unit; the dual-format bridge detects
    /// `id: spec` frontmatter in it.
    pub raw: String,
    /// On-disk path of the spec file (set by `walk_specs`; empty for
    /// synthetically parsed units). The dual-format bridge invokes
    /// `spk lint` against it.
    pub source_path: std::path::PathBuf,
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
    /// Contract archetype (`"PF"`, `"SA"`, `"BP"`, …) loaded from the
    /// scenario's `.espectacular/<spec>/<slug>.toml`, when one exists.
    /// `None` for scenarios without a contract or when the contract cannot
    /// be read (lint is advisory and must not hard-fail on contract state).
    pub archetype: Option<String>,
}

/// One spec file that could not be read (e.g. invalid UTF-8). The lint
/// engine turns these into error-severity findings instead of hard-failing,
/// so the rest of the corpus still lints (espectacular-pt8, deployed lint
/// spec C-finding-schema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecReadError {
    /// Capability directory name, mirrors `SpecFile::spec_path`.
    pub spec_path: String,
    /// On-disk path of the unreadable file.
    pub source_path: std::path::PathBuf,
    /// Underlying read error message.
    pub message: String,
}

/// Like [`walk_specs`], but per-file read failures are collected instead of
/// aborting the walk. Root-level failures (specs dir unreadable) still
/// return `Err` — the spec scenario covers malformed *files*, not a missing
/// corpus root.
pub fn walk_specs_with_errors(
    specs_dir: &Path,
) -> anyhow::Result<(Vec<SpecFile>, Vec<SpecReadError>)> {
    let mut specs = Vec::new();
    let mut errors = Vec::new();
    for entry in fs::read_dir(specs_dir)? {
        let entry = entry?;
        let spec_name = entry.file_name().to_string_lossy().into_owned();
        let spec_file = entry.path().join("spec.md");
        if !spec_file.exists() {
            continue;
        }
        match fs::read_to_string(&spec_file) {
            Ok(content) => specs.push({
                let mut spec = parse_spec(&content, &spec_name);
                spec.source_path = spec_file.clone();
                load_archetypes(&mut spec, contracts_root_for(specs_dir));
                spec
            }),
            Err(e) => errors.push(SpecReadError {
                spec_path: spec_name,
                source_path: spec_file,
                message: e.to_string(),
            }),
        }
    }
    specs.sort_by(|a, b| a.spec_path.cmp(&b.spec_path));
    Ok((specs, errors))
}

/// Nearest ancestor of `specs_dir` carrying an `.espectacular` contracts
/// directory, if any. Mirrors `lint_config_for`'s ancestor walk.
fn contracts_root_for(specs_dir: &Path) -> Option<std::path::PathBuf> {
    specs_dir
        .ancestors()
        .find(|d| d.join(".espectacular").is_dir())
        .map(|d| d.join(".espectacular"))
}

/// Populate each scenario's `archetype` from its contract file
/// `<contracts_root>/<spec_path>/<scenario_id>.toml`. Missing or unreadable
/// contracts leave `None` — advisory lint never fails on contract state
/// (contract validity is `ah check`'s job).
fn load_archetypes(spec: &mut SpecFile, contracts_root: Option<std::path::PathBuf>) {
    let Some(root) = contracts_root else {
        return;
    };
    for req in &mut spec.requirements {
        for scenario in &mut req.scenarios {
            let toml_path = root
                .join(&spec.spec_path)
                .join(format!("{}.toml", scenario.id));
            scenario.archetype = fs::read_to_string(&toml_path)
                .ok()
                .and_then(|text| toml::from_str::<crate::contracts::Contract>(&text).ok())
                .map(|c| c.archetype);
        }
    }
}

/// Parse one spec file's content into units. `pub(crate)` so check modules
/// can unit-test against inline spec markdown.
pub(crate) fn parse_spec(content: &str, spec_name: &str) -> SpecFile {
    let lines: Vec<&str> = content.lines().collect();
    let mut parser = SpecParser::new();
    parser.scan(&lines);
    SpecFile {
        spec_path: spec_name.to_string(),
        requirements: parser.requirements,
        raw: content.to_string(),
        source_path: std::path::PathBuf::new(),
    }
}

/// Line-oriented state machine behind [`parse_spec`].
///
/// Section-sync mirrors (same slugified id AND identical body, e.g. the
/// `## ADDED Requirements` / `## Requirements` pair in a dual-format delta)
/// are one logical scenario — same dedupe convention as
/// openspec::parse_scenarios_from_spec (deployed gate semantics:
/// deduplicate-mirrored-delta-sections).
struct SpecParser {
    requirements: Vec<RequirementUnit>,
    current_req: Option<usize>,
    seen: HashSet<(String, String)>,
}

impl SpecParser {
    fn new() -> Self {
        Self {
            requirements: Vec::new(),
            current_req: None,
            seen: HashSet::new(),
        }
    }

    fn scan(&mut self, lines: &[&str]) {
        for (i, &line) in lines.iter().enumerate() {
            if let Some(heading) = line.strip_prefix("### Requirement: ") {
                self.open_requirement(heading);
            } else if let Some(heading) = line.strip_prefix("#### Scenario: ") {
                self.push_scenario(lines, i, heading);
            } else {
                self.append_prose(line);
            }
        }
    }

    fn open_requirement(&mut self, heading: &str) {
        self.requirements.push(RequirementUnit {
            heading: heading.trim().to_string(),
            body: String::new(),
            scenarios: Vec::new(),
        });
        self.current_req = Some(self.requirements.len() - 1);
    }

    /// Attach one `#### Scenario:` block to the current requirement. Mirrored
    /// duplicates (same id AND body) are dropped.
    fn push_scenario(&mut self, lines: &[&str], line_no: usize, heading: &str) {
        let heading = heading.trim();
        let body = extract_body(lines, line_no + 1);
        if !self.seen.insert((slugify(heading), body.clone())) {
            return;
        }
        let scenario = ScenarioBlock {
            id: slugify(heading),
            heading: heading.to_string(),
            body,
            source_line: line_no + 1,
            archetype: None,
        };
        let idx = self.parent_index();
        self.requirements[idx].scenarios.push(scenario);
    }

    /// Index of the requirement a scenario hangs off. A scenario outside any
    /// requirement attaches to a synthetic parent so the walker never drops
    /// context.
    fn parent_index(&mut self) -> usize {
        if let Some(idx) = self.current_req {
            return idx;
        }
        self.requirements.push(RequirementUnit {
            heading: String::new(),
            body: String::new(),
            scenarios: Vec::new(),
        });
        self.requirements.len() - 1
    }

    /// Prose between the requirement heading and its first scenario.
    fn append_prose(&mut self, line: &str) {
        let Some(req) = self.current_req.map(|idx| &mut self.requirements[idx]) else {
            return;
        };
        if !req.scenarios.is_empty()
            || req.heading.is_empty()
            || line.starts_with('#')
            || line.trim().is_empty()
        {
            return;
        }
        if !req.body.is_empty() {
            req.body.push('\n');
        }
        req.body.push_str(line);
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
        let (specs, _) = walk_specs_with_errors(&clean_specs()).unwrap();
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

    /// Walk `dir` and assert every scenario id mirrors its heading slug —
    /// one nesting level per concern keeps the fixture test flat.
    fn assert_walk_ids_match_headings(dir: &Path) {
        let (specs, _) = walk_specs_with_errors(dir).unwrap();
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

    /// Total scenario count across every requirement of every spec file.
    fn scenario_count(specs: &[SpecFile]) -> usize {
        specs
            .iter()
            .flat_map(|s| &s.requirements)
            .map(|r| r.scenarios.len())
            .sum()
    }

    #[test]
    fn walker_parses_every_fixture() {
        for dir in [clean_specs(), defective_specs()] {
            assert_walk_ids_match_headings(&dir);
        }
        let (defective, _) = walk_specs_with_errors(&defective_specs()).unwrap();
        assert_eq!(scenario_count(&defective), 2);
    }

    #[test]
    fn walker_dedupes_section_sync_mirrors() {
        // Dual-format deltas mirror scenarios across ## ADDED Requirements /
        // ## Requirements sections; same id + identical body is ONE logical
        // scenario (deployed gate semantics: deduplicate-mirrored-delta-
        // sections). The walker must not double-emit them.
        let (specs, _) =
            walk_specs_with_errors(&PathBuf::from("tests/fixtures/lint/dual/openspec/specs"))
                .unwrap();
        assert_eq!(
            scenario_count(&specs),
            2,
            "mirrored scenarios double-emitted"
        );
    }
}
