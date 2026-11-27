//! JSON envelope serialization for the doctor outcome.

use super::types::{CapabilitySuggestion, DoctorOutcome, SessionSuggestion};

/// Serialize a doctor outcome to a JSON envelope.
///
/// The envelope data contains both the genesis doctor report (checks, summary, tool)
/// and the domain-specific suggestions as a `suggestions` array.
pub fn doctor_to_envelope(outcome: &DoctorOutcome) -> serde_json::Value {
    let suggestions_json: Vec<serde_json::Value> =
        outcome.suggestions.iter().map(suggestion_json).collect();
    let session_suggestions_json: Vec<serde_json::Value> = outcome
        .session_suggestions
        .iter()
        .map(session_suggestion_json)
        .collect();

    let data = serde_json::json!({
        "tool": outcome.genesis_report.tool,
        "checks": outcome.genesis_report.checks,
        "summary": outcome.genesis_report.summary,
        "suggestions": suggestions_json,
        "session_suggestions": session_suggestions_json,
    });

    let envelope = genesis::envelope::Envelope::success(
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION"),
        genesis::envelope::EnvelopeKind::Doctor,
        &data,
        vec![],
        vec![],
    );
    serde_json::to_value(&envelope).expect("envelope serialization")
}

fn suggestion_json(suggestion: &CapabilitySuggestion) -> serde_json::Value {
    serde_json::json!({
        "kind": "recommendation",
        "suggested_action": "enable_capability",
        "playbook_command": "ah explain enable_capability",
        "apply_command": suggestion.apply_command,
        "detail": suggestion.detail,
        "capability": suggestion.capability,
    })
}

fn session_suggestion_json(suggestion: &SessionSuggestion) -> serde_json::Value {
    serde_json::json!({
        "kind": "suggestion",
        "suggested_action": "run_ah_lint",
        "playbook_command": "ah explain lint",
        "apply_command": suggestion.apply_command,
        "detail": suggestion.detail,
    })
}
