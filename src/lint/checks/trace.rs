//! Scenario-to-property trace checks (derive-contracts-from-specodelic 5.3).
//!
//! Purpose: give the `VERIFIES` coverage convention its own advisory checker —
//! dangling links, unlinked scenarios, and untraced properties — so the
//! property-derived-contract coverage map cannot silently rot.
//!
//! Responsibilities:
//! - `VerifiesDanglingCheck`: a `- **VERIFIES** [[spec.P-x]]` bullet whose
//!   property id has no Properties row in the file
//! - `ScenarioUnlinkedCheck`: a scenario without any VERIFIES bullet in a
//!   dual-format file that has Properties rows
//! - `PropertyUntracedCheck`: a Properties row cited by no scenario's
//!   VERIFIES bullets
//!
//! All three are advisory (`Severity::Warning`) and never affect the exit
//! code (deployed lint spec C-trace-advisory). Unlinked scenarios remain
//! legitimate under design D1 — they fall back to their own contracts — so
//! these findings are adoption signals, not gate failures.

use crate::check::verifies_property_ids;
use crate::lint::bridge::is_dual_format;
use crate::lint::walker::SpecFile;
use crate::lint::{LintCheck, LintFinding};

/// Property ids declared in the file's `## Properties` table (first column),
/// in declaration order. Empty for files without a Properties section.
fn property_ids(raw: &str) -> Vec<String> {
    let mut in_properties = false;
    let mut ids = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == "## Properties" {
            in_properties = true;
            continue;
        }
        if in_properties && trimmed.starts_with("## ") {
            break;
        }
        if !in_properties || !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed.trim_matches('|').split('|').collect();
        if cells.len() < 2 {
            continue;
        }
        let id = cells[0].trim();
        // Skip the header and column-separator rows; a valid id is non-empty
        // and not a dashed rule.
        if id.is_empty() || id.starts_with('-') || id == "id" {
            continue;
        }
        ids.push(id.to_string());
    }
    ids
}

/// Gate for all three checks: dual-format files with at least one Properties
/// row. Plain openspec files and dual-format files without rows are untouched
/// (no trace semantics apply).
fn trace_target(spec: &SpecFile) -> bool {
    is_dual_format(&spec.raw) && !property_ids(&spec.raw).is_empty()
}

pub(crate) struct VerifiesDanglingCheck;

impl LintCheck for VerifiesDanglingCheck {
    fn kind(&self) -> &'static str {
        "verifies-dangling"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        if !trace_target(spec) {
            return;
        }
        let ids = property_ids(&spec.raw);
        let file_id = crate::openspec::frontmatter_id(&spec.raw);
        for req in &spec.requirements {
            for scenario in &req.scenarios {
                for prop in verifies_property_ids(&scenario.body, file_id.as_deref()) {
                    if !ids.contains(&prop) {
                        findings.push(LintFinding::warning(
                            self.kind(),
                            &spec.spec_path,
                            &scenario.id,
                            &format!(
                                "scenario's VERIFIES bullet links to property `{prop}` which has no Properties row in this file"
                            ),
                            &format!(
                                "correct the property id in the bullet, or add a `{prop}` Properties row, or remove the bullet and give the scenario its own contract toml"
                            ),
                            "edit_spec",
                            "ah explain verifies-dangling",
                        ));
                    }
                }
            }
        }
    }
}

pub(crate) struct ScenarioUnlinkedCheck;

impl LintCheck for ScenarioUnlinkedCheck {
    fn kind(&self) -> &'static str {
        "scenario-unlinked"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        if !trace_target(spec) {
            return;
        }
        let file_id = crate::openspec::frontmatter_id(&spec.raw);
        for req in &spec.requirements {
            for scenario in &req.scenarios {
                if verifies_property_ids(&scenario.body, file_id.as_deref()).is_empty() {
                    findings.push(LintFinding::warning(
                        self.kind(),
                        &spec.spec_path,
                        &scenario.id,
                        "scenario has no VERIFIES link — in a file with Properties rows it relies on its own contract toml (design D1 fallback)",
                        "add a `- **VERIFIES** [[<file-id>.P-id]]` bullet to link the scenario to a property's derived contract, or keep the scenario's own contract",
                        "edit_spec",
                        "ah explain scenario-unlinked",
                    ));
                }
            }
        }
    }
}

pub(crate) struct PropertyUntracedCheck;

impl LintCheck for PropertyUntracedCheck {
    fn kind(&self) -> &'static str {
        "property-untraced"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        if !trace_target(spec) {
            return;
        }
        let file_id = crate::openspec::frontmatter_id(&spec.raw);
        let cited: Vec<String> = spec
            .requirements
            .iter()
            .flat_map(|req| req.scenarios.iter())
            .flat_map(|s| verifies_property_ids(&s.body, file_id.as_deref()))
            .collect();
        for prop in property_ids(&spec.raw) {
            if !cited.contains(&prop) {
                findings.push(LintFinding::warning(
                    self.kind(),
                    &spec.spec_path,
                    &prop,
                    &format!(
                        "property `{prop}` is cited by no scenario's VERIFIES bullet — no scenario covers it"
                    ),
                    &format!(
                        "add `- **VERIFIES** [[spec.{prop}]]` to a covering scenario, or supersede the row if it is no longer testable"
                    ),
                    "edit_spec",
                    "ah explain property-untraced",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::bridge::is_dual_format;
    use crate::lint::walker;

    fn spec_from(raw: &str) -> SpecFile {
        let mut spec = walker::parse_spec(raw, "auth");
        spec.source_path = std::path::PathBuf::from("/virtual/spec.md");
        spec
    }

    const DUAL_PARTIAL: &str = "---\nid: spec\nkind: intent\nstatement: \"WHEN x THE system SHALL y.\"\n---\n\n# Spec\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-a | unit | [[spec.C-a]] | input | output |\n| P-b | unit | [[spec.C-a]] | input | output |\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n\n#### Scenario: linked\n- **WHEN** a token is checked\n- **VERIFIES** [[spec.P-a]]\n\n#### Scenario: unlinked\n- **WHEN** a session expires\n- **THEN** the user is logged out\n";

    const DUAL_FULLY_TRACED: &str = "---\nid: spec\nkind: intent\nstatement: \"WHEN x THE system SHALL y.\"\n---\n\n# Spec\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-a | unit | [[spec.C-a]] | input | output |\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n\n#### Scenario: linked\n- **WHEN** a token is checked\n- **VERIFIES** [[spec.P-a]]\n";

    const DUAL_DANGLING: &str = "---\nid: spec\nkind: intent\nstatement: \"WHEN x THE system SHALL y.\"\n---\n\n# Spec\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-a | unit | [[spec.C-a]] | input | output |\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n\n#### Scenario: linked\n- **WHEN** a token is checked\n- **VERIFIES** [[spec.P-missing]]\n";

    const DUAL_NO_PROPERTIES: &str = "---\nid: spec\nkind: intent\nstatement: \"WHEN x THE system SHALL y.\"\n---\n\n# Spec\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n\n#### Scenario: linked\n- **WHEN** a token is checked\n- **THEN** it is accepted\n";

    const PLAIN_RAW: &str =
        "# Capability: auth\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

    #[test]
    fn dangling_verifies_link_is_flagged_with_scenario_id() {
        let spec = spec_from(DUAL_DANGLING);
        assert!(is_dual_format(&spec.raw));
        let mut findings = Vec::new();
        VerifiesDanglingCheck.check(&spec, &mut findings);
        assert_eq!(findings.len(), 1, "{findings:?}");
        let f = &findings[0];
        assert_eq!(f.kind, "verifies-dangling");
        assert_eq!(f.severity, crate::lint::Severity::Warning);
        assert_eq!(f.scenario_id, "linked");
        assert!(f.message.contains("P-missing"), "{f:?}");
    }

    #[test]
    fn unlinked_scenario_and_untraced_property_are_flagged() {
        let spec = spec_from(DUAL_PARTIAL);
        let mut findings = Vec::new();
        ScenarioUnlinkedCheck.check(&spec, &mut findings);
        assert_eq!(
            findings.len(),
            1,
            "the 'unlinked' scenario only: {findings:?}"
        );
        assert_eq!(findings[0].kind, "scenario-unlinked");
        assert_eq!(findings[0].scenario_id, "unlinked");

        findings.clear();
        PropertyUntracedCheck.check(&spec, &mut findings);
        let ids: Vec<&str> = findings.iter().map(|f| f.scenario_id.as_str()).collect();
        assert_eq!(ids, vec!["P-b"], "only the uncited property: {findings:?}");
    }

    #[test]
    fn fully_traced_dual_file_emits_no_trace_findings() {
        let spec = spec_from(DUAL_FULLY_TRACED);
        for check in [
            &VerifiesDanglingCheck as &dyn LintCheck,
            &ScenarioUnlinkedCheck,
            &PropertyUntracedCheck,
        ] {
            let mut findings = Vec::new();
            check.check(&spec, &mut findings);
            assert!(findings.is_empty(), "{}: {findings:?}", check.kind());
        }
    }

    #[test]
    fn dual_file_without_properties_rows_is_untouched() {
        // No Properties rows → no unlinked/untraced semantics apply; a lone
        // VERIFIES bullet can't exist meaningfully either (no rows at all).
        let spec = spec_from(DUAL_NO_PROPERTIES);
        for check in [
            &VerifiesDanglingCheck as &dyn LintCheck,
            &ScenarioUnlinkedCheck,
            &PropertyUntracedCheck,
        ] {
            let mut findings = Vec::new();
            check.check(&spec, &mut findings);
            assert!(findings.is_empty(), "{}: {findings:?}", check.kind());
        }
    }

    #[test]
    fn plain_openspec_files_are_untouched() {
        let spec = spec_from(PLAIN_RAW);
        for check in [
            &VerifiesDanglingCheck as &dyn LintCheck,
            &ScenarioUnlinkedCheck,
            &PropertyUntracedCheck,
        ] {
            let mut findings = Vec::new();
            check.check(&spec, &mut findings);
            assert!(findings.is_empty(), "{}: {findings:?}", check.kind());
        }
    }
}
