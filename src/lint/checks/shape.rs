//! Spec-shape lint checks (espectacular-aar).
//!
//! Purpose: detect spec-file and archetype-level authoring defects — missing
//! non-goals sections, unresolved ambiguity markers, and presentation
//! primitives leaking into non-presentation archetypes — so AI agents don't
//! implement correct-by-gate but wrong-by-intent behavior.
//!
//! Responsibilities:
//! - `MissingNonGoalsCheck`: capability spec files lacking a Non-Goals /
//!   Out-of-Scope section (task 3.5)
//! - `UnresolvedAmbiguityCheck`: `[NEEDS CLARIFICATION` markers in
//!   requirement bodies or scenario steps (task 3.6)
//! - `EntangledSpecCheck`: PF/SA scenarios referencing presentation-layer
//!   primitives, archetype-gated via the scenario's contract (task 3.7)
//!
//! Rationale: all three are heuristic, advisory-only checks (warning
//! severity); they shape findings per the delta spec
//! `openspec/changes/add-spec-quality-checks/specs/lint/spec.md`.

use crate::lint::checks::line_contains_term;
use crate::lint::walker::{ScenarioBlock, SpecFile};
use crate::lint::{LintCheck, LintFinding};

/// Heading terms that count as an explicit non-goals section.
const NON_GOALS_HEADING_TERMS: &[&str] = &["non-goals", "non goals", "out of scope"];

/// Authoring-time ambiguity marker (matched case-insensitively).
const NEEDS_CLARIFICATION_MARKER: &str = "[needs clarification";

/// Presentation-layer primitives that must not appear in PF/SA scenarios.
const ENTANGLED_MATCHERS: &[&str] = &[
    "button", "modal", "banner", "css", "selector", "hover", "scroll", "color", "font", "pixel",
    "dropdown", "tooltip", "toast", "green", "red", "blue",
];

/// Domain-legitimate phrases that superficially resemble presentation terms
/// (e.g. "event routing layer") — matching lines are skipped.
const ENTANGLED_EXCLUSIONS: &[&str] = &["event routing"];

/// Contract archetypes whose scenarios must stay free of presentation
/// primitives. `BP` scenarios are excluded: transport mechanics are
/// legitimate at a boundary seam.
const ENTANGLED_ARCHETYPES: &[&str] = &["PF", "SA"];

/// Extract the ambiguity marker text (`[NEEDS CLARIFICATION: …]`) for
/// inclusion in the finding message.
fn marker_text(line: &str) -> String {
    let start = line.to_lowercase().find(NEEDS_CLARIFICATION_MARKER);
    let Some(start) = start else {
        return line.trim().to_string();
    };
    match line[start..].find(']') {
        Some(end) => line[start..start + end + 1].to_string(),
        None => line[start..].trim_end().to_string(),
    }
}

/// Missing non-goals detection (task 3.5): flag spec capability files lacking
/// an explicit Non-Goals / Out-of-Scope section, whose absence enables silent
/// scope creep during AI-assisted implementation.
pub struct MissingNonGoalsCheck;

impl MissingNonGoalsCheck {
    fn has_non_goals(spec: &SpecFile) -> bool {
        spec.raw.lines().any(|line| {
            line.trim_start().starts_with('#')
                && NON_GOALS_HEADING_TERMS
                    .iter()
                    .any(|t| line.to_lowercase().contains(t))
        })
    }
}

impl LintCheck for MissingNonGoalsCheck {
    fn kind(&self) -> &'static str {
        "missing-non-goals"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        if Self::has_non_goals(spec) {
            return;
        }
        findings.push(LintFinding::warning(
            "missing-non-goals",
            &spec.spec_path,
            "",
            "spec has no Non-Goals / Out-of-Scope section",
            "add a `## Non-Goals` section listing explicit exclusions",
            "edit_spec",
            "ah explain missing-non-goals",
        ));
    }
}

/// Unresolved ambiguity detection (task 3.6): flag requirement bodies and
/// scenario steps containing `[NEEDS CLARIFICATION` markers.
pub struct UnresolvedAmbiguityCheck;

fn scan_ambiguity(line: &str) -> Option<String> {
    if line.to_lowercase().contains(NEEDS_CLARIFICATION_MARKER) {
        Some(marker_text(line))
    } else {
        None
    }
}

impl LintCheck for UnresolvedAmbiguityCheck {
    fn kind(&self) -> &'static str {
        "unresolved-ambiguity"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        let mut push = |spec: &SpecFile, scenario_id: &str, marker: String| {
            findings.push(LintFinding::warning(
                "unresolved-ambiguity",
                &spec.spec_path,
                scenario_id,
                &format!("unresolved ambiguity marker {marker}"),
                "resolve the deferred decision and remove the marker",
                "edit_spec",
                "ah explain unresolved-ambiguity",
            ));
        };
        for req in &spec.requirements {
            for line in req.body.lines() {
                if let Some(marker) = scan_ambiguity(line) {
                    push(spec, "", marker);
                }
            }
            for scenario in &req.scenarios {
                for line in scenario.body.lines() {
                    if let Some(marker) = scan_ambiguity(line) {
                        push(spec, &scenario.id, marker);
                    }
                }
            }
        }
    }
}

/// Entangled specification detection (task 3.7): flag PF/SA scenarios whose
/// text references presentation-layer primitives — the archetype declares
/// the scenario belongs to a layer where UI mechanics must not appear.
pub struct EntangledSpecCheck;

impl EntangledSpecCheck {
    fn is_gated(scenario: &ScenarioBlock) -> bool {
        scenario
            .archetype
            .as_deref()
            .map(|a| ENTANGLED_ARCHETYPES.contains(&a))
            .unwrap_or(false)
    }

    fn matching_primitive(line: &str) -> Option<&'static str> {
        if ENTANGLED_EXCLUSIONS
            .iter()
            .any(|e| line.to_lowercase().contains(e))
        {
            return None;
        }
        ENTANGLED_MATCHERS
            .iter()
            .find(|m| line_contains_term(line, m))
            .copied()
    }
}

impl LintCheck for EntangledSpecCheck {
    fn kind(&self) -> &'static str {
        "entangled-spec"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        for req in &spec.requirements {
            for scenario in &req.scenarios {
                if !Self::is_gated(scenario) {
                    continue;
                }
                for line in scenario.body.lines() {
                    if let Some(primitive) = Self::matching_primitive(line) {
                        findings.push(LintFinding::warning(
                            "entangled-spec",
                            &spec.spec_path,
                            &scenario.id,
                            &format!(
                                "presentation primitive '{}' in {} scenario",
                                primitive,
                                scenario.archetype.as_deref().unwrap_or("?")
                            ),
                            "specify the domain event or state instead of the visual primitive",
                            "edit_spec",
                            "ah explain entangled-spec",
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::walker;

    fn spec(md: &str) -> SpecFile {
        walker::parse_spec(md, "test")
    }

    fn run(check: &dyn LintCheck, md: &str) -> Vec<LintFinding> {
        let mut findings = Vec::new();
        check.check(&spec(md), &mut findings);
        findings
    }

    #[test]
    fn missing_non_goals_flags_spec_without_section() {
        let findings = run(
            &MissingNonGoalsCheck,
            "# Capability: auth\n\n## Requirements\n\n### Requirement: Login\n",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "missing-non-goals");
        assert_eq!(findings[0].severity, crate::lint::Severity::Warning);
    }

    #[test]
    fn missing_non_goals_accepts_out_of_scope_variant() {
        for heading in [
            "## Non-Goals",
            "## Non Goals",
            "## non-goals",
            "## Out of Scope",
        ] {
            let md = format!("# Capability: auth\n{heading}\n\n- nothing\n");
            assert!(
                run(&MissingNonGoalsCheck, &md).is_empty(),
                "fired on {heading}"
            );
        }
    }

    #[test]
    fn missing_non_goals_ignores_prose_mention() {
        // The term in prose (not a heading) does not satisfy the section.
        let md = "# Capability: auth\n\nThis spec has no non-goals section yet.\n";
        assert_eq!(run(&MissingNonGoalsCheck, md).len(), 1);
    }

    #[test]
    fn unresolved_ambiguity_flags_marker_in_requirement_body() {
        let findings = run(
            &UnresolvedAmbiguityCheck,
            "### Requirement: Auth\nThe system SHALL authenticate. [NEEDS CLARIFICATION: which auth provider?]\n",
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0]
            .message
            .contains("[NEEDS CLARIFICATION: which auth provider?]"));
    }

    #[test]
    fn unresolved_ambiguity_flags_marker_in_scenario_step() {
        let findings = run(
            &UnresolvedAmbiguityCheck,
            "### Requirement: Auth\nThe system SHALL authenticate.\n\n#### Scenario: Login\n- **WHEN** the user logs in [NEEDS CLARIFICATION: mfa?]\n- **THEN** the session is created\n",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].scenario_id, "login");
    }

    #[test]
    fn unresolved_ambiguity_accepts_clean_spec() {
        let findings = run(
            &UnresolvedAmbiguityCheck,
            "### Requirement: Auth\nThe system SHALL authenticate users.\n",
        );
        assert!(findings.is_empty());
    }

    fn scenario_with_archetype(id: &str, archetype: Option<&str>, body: &str) -> ScenarioBlock {
        ScenarioBlock {
            id: id.to_string(),
            heading: id.to_string(),
            body: body.to_string(),
            source_line: 1,
            archetype: archetype.map(|a| a.to_string()),
        }
    }

    fn run_over_scenarios(
        check: &dyn LintCheck,
        scenarios: Vec<ScenarioBlock>,
    ) -> Vec<LintFinding> {
        let spec = SpecFile {
            spec_path: "ui".to_string(),
            requirements: vec![walker::RequirementUnit {
                heading: "Feedback".to_string(),
                body: String::new(),
                scenarios,
            }],
            raw: String::new(),
            source_path: std::path::PathBuf::new(),
        };
        let mut findings = Vec::new();
        check.check(&spec, &mut findings);
        findings
    }

    #[test]
    fn entangled_spec_flags_presentation_primitive_in_pf_scenario() {
        let findings = run_over_scenarios(
            &EntangledSpecCheck,
            vec![scenario_with_archetype(
                "update-the-dashboard",
                Some("PF"),
                "- **THEN** a green notification banner is displayed\n",
            )],
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("banner"));
        assert_eq!(findings[0].scenario_id, "update-the-dashboard");
    }

    #[test]
    fn entangled_spec_is_archetype_gated() {
        let md = "- **WHEN** the user navigates to /dashboard/settings\n";
        // BP: transport mechanics are legitimate at a boundary seam.
        assert!(run_over_scenarios(
            &EntangledSpecCheck,
            vec![scenario_with_archetype("nav", Some("BP"), md)]
        )
        .is_empty());
        // No contract: nothing to gate on, stay silent.
        assert!(run_over_scenarios(
            &EntangledSpecCheck,
            vec![scenario_with_archetype("nav", None, md)]
        )
        .is_empty());
    }

    #[test]
    fn entangled_spec_excludes_domain_legitimate_language() {
        let findings = run_over_scenarios(
            &EntangledSpecCheck,
            vec![scenario_with_archetype(
                "deliver-message",
                Some("SA"),
                "- **THEN** the event routing layer delivers the message\n",
            )],
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn entangled_spec_uses_token_match_not_substring() {
        // "redirect" must not match "red"; "considered" must not match "red".
        let findings = run_over_scenarios(
            &EntangledSpecCheck,
            vec![scenario_with_archetype(
                "route",
                Some("SA"),
                "- **THEN** the command redirects to the considered route\n",
            )],
        );
        assert!(findings.is_empty());
    }

    // ---- property-derived contract bindings (DDL pilot, design D6) --------

    #[test]
    fn p_non_goals() {
        missing_non_goals_flags_spec_without_section();
        missing_non_goals_accepts_out_of_scope_variant();
    }

    #[test]
    fn p_ambiguity() {
        unresolved_ambiguity_flags_marker_in_requirement_body();
        unresolved_ambiguity_flags_marker_in_scenario_step();
        unresolved_ambiguity_accepts_clean_spec();
    }

    #[test]
    fn p_entangled() {
        entangled_spec_flags_presentation_primitive_in_pf_scenario();
        entangled_spec_is_archetype_gated();
        entangled_spec_excludes_domain_legitimate_language();
    }
}
