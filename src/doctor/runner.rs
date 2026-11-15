//! Doctor entry point: runs the checks and collects domain findings.

use super::build::build_checks;
use super::detect::{framework_result, property_result};
use super::session::{falsifiability_session_suggestion, lint_session_suggestion};
use super::types::{CapabilitySuggestion, DoctorOutcome, FrameworkDetection, SessionSuggestion};
use crate::config;
use std::path::Path;

pub fn run_doctor(repo_root: &Path) -> anyhow::Result<DoctorOutcome> {
    let genesis_report = run_genesis_checks(repo_root)?;
    let (detections, suggestions, mut session_suggestions) = match config::load(repo_root) {
        Ok(cfg) => config_findings(repo_root, &cfg),
        Err(_) => Default::default(),
    };
    session_suggestions.extend(lint_session_suggestion(repo_root));

    Ok(DoctorOutcome {
        genesis_report,
        detections,
        suggestions,
        session_suggestions,
    })
}

/// Run the genesis doctor framework over the assembled checks.
fn run_genesis_checks(repo_root: &Path) -> anyhow::Result<genesis::doctor::DoctorReport> {
    genesis::doctor::DoctorRunner::new(build_checks(repo_root))
        .with_tool_name("ah")
        .run(repo_root, false)
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Domain-specific findings that need a loaded config.
type ConfigFindings = (
    Vec<FrameworkDetection>,
    Vec<CapabilitySuggestion>,
    Vec<SessionSuggestion>,
);

fn config_findings(repo_root: &Path, cfg: &config::Config) -> ConfigFindings {
    let mut detections: Vec<FrameworkDetection> = Vec::new();
    let mut suggestions: Vec<CapabilitySuggestion> = Vec::new();
    for &fw in &["pytest", "cargo", "vitest"] {
        let (det, rec) = framework_result(fw, repo_root, cfg);
        detections.extend(det);
        suggestions.extend(rec);
    }
    let (det, rec) = property_result(repo_root, cfg);
    detections.extend(det);
    suggestions.extend(rec);
    let session = falsifiability_session_suggestion(repo_root, cfg)
        .into_iter()
        .collect();
    (detections, suggestions, session)
}
