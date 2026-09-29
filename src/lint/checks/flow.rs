//! Scenario-flow lint checks (espectacular-rty).
//!
//! Purpose: detect spec defects in how scenarios express behavior — unbound
//! qualifiers, UI-mechanic steps, AND-chained bloat, and missing negative
//! scenarios — so AI agents don't implement correct-by-gate but
//! wrong-by-intent behavior.
//!
//! Responsibilities:
//! - `VagueQualifierCheck`: qualitative terms without an adjacent numeric
//!   bound in requirement bodies and scenario steps (task 3.1)
//! - `ImperativeStepCheck`: UI mechanics in WHEN/THEN/AND steps (task 3.2)
//! - `ConjunctiveBloatCheck`: more than the configured maximum AND-chained
//!   steps in one scenario (task 3.3)
//! - `MissingNegativeScenarioCheck`: requirements with no scenario exercising
//!   an error, rejection, or boundary violation (task 3.4)
//!
//! Rationale: all four are heuristic, advisory-only checks (warning severity);
//! they shape findings per the delta spec
//! `openspec/changes/add-spec-quality-checks/specs/lint/spec.md`.

use crate::lint::checks::line_contains_term;
use crate::lint::walker::{RequirementUnit, ScenarioBlock, SpecFile};
use crate::lint::{LintCheck, LintFinding};

/// Qualitative terms that must carry an adjacent numeric bound.
const QUALIFIER_TERMS: &[&str] = &[
    "fast",
    "scalable",
    "user-friendly",
    "easy",
    "simple",
    "intuitive",
];

/// UI-mechanic markers that must not appear in WHEN/THEN/AND steps.
const IMPERATIVE_MARKERS: &[&str] = &[
    "click",
    "navigates to",
    "fills",
    "button",
    "modal",
    "css",
    "selector",
    "hover",
    "scroll",
];

/// Language marking a scenario as exercising an error, rejection, or
/// boundary violation.
const NEGATIVE_MARKERS: &[&str] = &[
    "exits non-zero",
    "reject",
    "error",
    "invalid",
    "fail",
    "denied",
    "violation",
    "exceed",
];

/// Bullet-step marker of a scenario body line: `"- **WHEN** ..."` → `"WHEN"`.
fn step_marker(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix("- **")?;
    let end = rest.find("**")?;
    Some(&rest[..end])
}

fn bullet_lines(body: &str) -> Vec<&str> {
    body.lines()
        .filter(|l| l.trim_start().starts_with("- "))
        .collect()
}

/// Scan a prose line for unbound qualitative terms (no digit on the line).
fn scan_unbound_qualifier(line: &str, findings: &mut Vec<String>) {
    let has_term = QUALIFIER_TERMS.iter().any(|t| line_contains_term(line, t));
    if has_term && !line.chars().any(|c| c.is_ascii_digit()) {
        if let Some(term) = QUALIFIER_TERMS.iter().find(|t| line_contains_term(line, t)) {
            findings.push(format!("unbound qualifier '{term}'"));
        }
    }
}

/// Vague qualifier detection (task 3.1): flag requirement bodies and scenario
/// steps containing unbound qualitative terms without an adjacent numeric
/// measurement.
pub struct VagueQualifierCheck;

impl LintCheck for VagueQualifierCheck {
    fn kind(&self) -> &'static str {
        "vague-qualifier"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        for req in &spec.requirements {
            let mut hits = Vec::new();
            for line in req.body.lines() {
                scan_unbound_qualifier(line, &mut hits);
            }
            for scenario in &req.scenarios {
                for line in scenario.body.lines() {
                    scan_unbound_qualifier(line, &mut hits);
                }
            }
            for hit in hits {
                findings.push(LintFinding::warning(
                    "vague-qualifier",
                    &spec.spec_path,
                    "",
                    &hit,
                    "add a measurable response condition (e.g. within 200 ms)",
                    "edit_spec",
                    "ah explain vague-qualifier",
                ));
            }
        }
    }
}

/// Imperative step detection (task 3.2): flag WHEN/THEN/AND steps that
/// describe UI mechanics rather than business intent.
pub struct ImperativeStepCheck;

impl LintCheck for ImperativeStepCheck {
    fn kind(&self) -> &'static str {
        "imperative-step"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        for req in &spec.requirements {
            for scenario in &req.scenarios {
                for line in bullet_lines(&scenario.body) {
                    let marker = step_marker(line).unwrap_or("");
                    if !matches!(marker, "WHEN" | "THEN" | "AND") {
                        continue;
                    }
                    let lower = line.to_lowercase();
                    if let Some(mechanic) = IMPERATIVE_MARKERS.iter().find(|m| lower.contains(**m))
                    {
                        findings.push(LintFinding::warning(
                            "imperative-step",
                            &spec.spec_path,
                            &scenario.id,
                            &format!("UI mechanic '{mechanic}' in {marker} step"),
                            "rephrase to describe the business action (e.g. submits the form)",
                            "edit_spec",
                            "ah explain imperative-step",
                        ));
                    }
                }
            }
        }
    }
}

/// Conjunctive bloat detection (task 3.3): flag scenarios chaining more than
/// the configured maximum of AND-linked steps.
pub struct ConjunctiveBloatCheck {
    pub max_and_steps: usize,
}

impl ConjunctiveBloatCheck {
    fn and_count(scenario: &ScenarioBlock) -> usize {
        bullet_lines(&scenario.body)
            .iter()
            .filter(|l| step_marker(l) == Some("AND"))
            .count()
    }
}

impl LintCheck for ConjunctiveBloatCheck {
    fn kind(&self) -> &'static str {
        "conjunctive-bloat"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        for req in &spec.requirements {
            for scenario in &req.scenarios {
                let and_count = Self::and_count(scenario);
                if and_count > self.max_and_steps {
                    findings.push(LintFinding::warning(
                        "conjunctive-bloat",
                        &spec.spec_path,
                        &scenario.id,
                        &format!(
                            "scenario chains {and_count} AND steps (max {})",
                            self.max_and_steps
                        ),
                        "split the scenario into focused single-behavior scenarios",
                        "edit_spec",
                        "ah explain conjunctive-bloat",
                    ));
                }
            }
        }
    }
}

/// Missing negative scenario detection (task 3.4): flag requirements with no
/// scenario exercising an error, rejection, or boundary violation.
pub struct MissingNegativeScenarioCheck;

fn has_negative_language(req: &RequirementUnit) -> bool {
    req.scenarios.iter().any(|s| {
        let lower = format!("{}\n{}", s.heading, s.body).to_lowercase();
        NEGATIVE_MARKERS.iter().any(|m| lower.contains(m))
    })
}

impl LintCheck for MissingNegativeScenarioCheck {
    fn kind(&self) -> &'static str {
        "missing-negative-scenario"
    }

    fn check(&self, spec: &SpecFile, findings: &mut Vec<LintFinding>) {
        for req in &spec.requirements {
            if req.heading.is_empty() || has_negative_language(req) {
                continue;
            }
            findings.push(LintFinding::warning(
                "missing-negative-scenario",
                &spec.spec_path,
                "",
                &format!("requirement '{}' has no negative scenario", req.heading),
                "add a scenario for the corresponding failure mode",
                "edit_spec",
                "ah explain missing-negative-scenario",
            ));
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
    fn vague_qualifier_flags_unbound_term_in_requirement_body() {
        let findings = run(
            &VagueQualifierCheck,
            "### Requirement: Speed\nThe system SHALL update the dashboard fast.\n",
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("fast"));
        assert_eq!(findings[0].severity, crate::lint::Severity::Warning);
    }

    #[test]
    fn vague_qualifier_accepts_bounded_term() {
        let findings = run(
            &VagueQualifierCheck,
            "### Requirement: Speed\nThe system SHALL respond fast (p95 < 200 ms).\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn vague_qualifier_flags_qualifier_in_scenario_step() {
        let findings = run(
            &VagueQualifierCheck,
            "### Requirement: UX\nThe system SHALL show the result.\n\n#### Scenario: Feedback\n- **WHEN** the result arrives\n- **THEN** the response is user-friendly\n",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn vague_qualifier_ignores_partial_word_matches() {
        let findings = run(
            &VagueQualifierCheck,
            "### Requirement: Serving\nThe system SHALL serve breakfast promptly.\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn imperative_step_flags_ui_mechanics_in_when_step() {
        let findings = run(
            &ImperativeStepCheck,
            "### Requirement: Submit\nThe system SHALL accept submissions.\n\n#### Scenario: Submit form\n- **WHEN** the user clicks the Submit button\n- **THEN** the form is submitted\n",
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("click"));
        assert_eq!(findings[0].scenario_id, "submit-form");
    }

    #[test]
    fn imperative_step_flags_url_navigation_and_field_fill() {
        let findings = run(
            &ImperativeStepCheck,
            "### Requirement: Nav\nThe system SHALL show settings.\n\n#### Scenario: Open settings\n- **WHEN** the user navigates to /dashboard/settings\n- **AND** fills the email field\n- **THEN** the settings page is shown\n",
        );
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn imperative_step_accepts_declarative_business_steps() {
        let findings = run(
            &ImperativeStepCheck,
            "### Requirement: Pay\nThe system SHALL accept payments.\n\n#### Scenario: Pay\n- **WHEN** the user submits a payment\n- **THEN** the payment is recorded\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn conjunctive_bloat_flags_scenario_over_default_limit() {
        let findings = run(
            &ConjunctiveBloatCheck { max_and_steps: 5 },
            "### Requirement: Flow\nThe system SHALL process the flow.\n\n#### Scenario: Chain\n- **WHEN** step one happens\n- **AND** step two happens\n- **AND** step three happens\n- **AND** step four happens\n- **AND** step five happens\n- **AND** step six happens\n- **AND** step seven happens\n- **THEN** the flow completes\n",
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("6 AND steps"));
    }

    #[test]
    fn conjunctive_bloat_accepts_scenario_exactly_at_limit() {
        let findings = run(
            &ConjunctiveBloatCheck { max_and_steps: 5 },
            "### Requirement: Flow\nThe system SHALL process the flow.\n\n#### Scenario: Chain\n- **WHEN** step one happens\n- **AND** step two happens\n- **AND** step three happens\n- **AND** step four happens\n- **AND** step five happens\n- **THEN** the flow completes\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn conjunctive_bloat_honors_configured_max() {
        let md = "### Requirement: Flow\nThe system SHALL process the flow.\n\n#### Scenario: Chain\n- **WHEN** step one happens\n- **AND** step two happens\n- **AND** step three happens\n- **AND** step four happens\n- **AND** step five happens\n- **THEN** the flow completes\n";
        // 4 AND steps: over a max of 3, exactly at the limit of 4.
        assert_eq!(
            run(&ConjunctiveBloatCheck { max_and_steps: 3 }, md).len(),
            1
        );
        assert_eq!(
            run(&ConjunctiveBloatCheck { max_and_steps: 4 }, md).len(),
            0
        );
    }

    #[test]
    fn missing_negative_scenario_flags_happy_path_only_requirement() {
        let findings = run(
            &MissingNegativeScenarioCheck,
            "### Requirement: Export\nThe system SHALL export the report.{HAPPY_SCENARIO}",
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Export"));
        assert_eq!(findings[0].scenario_id, "");
    }

    #[test]
    fn missing_negative_scenario_accepts_negative_scenario() {
        let findings = run(
            &MissingNegativeScenarioCheck,
            "### Requirement: Login\nThe system SHALL authenticate users.\n\n#### Scenario: Bad credentials\n- **WHEN** the user submits invalid credentials\n- **THEN** the command exits non-zero and no session is created\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn missing_negative_scenario_accepts_rejection_language() {
        let findings = run(
            &MissingNegativeScenarioCheck,
            "### Requirement: Parse\nThe system SHALL parse input.\n\n#### Scenario: Bad input\n- **WHEN** the input is malformed\n- **THEN** the request is rejected with a parse error\n",
        );
        assert!(findings.is_empty());
    }
}
