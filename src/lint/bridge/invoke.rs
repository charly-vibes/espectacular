//! Process seam for `spk`: invoke the binary and decode its envelopes.
//!
//! Every failure mode maps to an advisory [`Failure`] — the bridge never
//! hard-fails the lint run.

use serde::Deserialize;

#[derive(Deserialize)]
struct SpkEnvelope {
    ok: bool,
    #[serde(default)]
    data: Option<SpkData>,
    #[serde(default)]
    warnings: Vec<SpkWarning>,
}

#[derive(Deserialize)]
struct SpkData {
    #[serde(default)]
    issues: Vec<SpkIssue>,
}

#[derive(Deserialize)]
struct SpkWarning {
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
pub(crate) struct SpkIssue {
    pub(crate) rule_id: String,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) rule_semantics: String,
}

pub(crate) enum Failure {
    Unavailable(String),
    Broken(String),
}

/// Invoke `spk lint <path> --json` and extract its issues. Every failure mode
/// maps to an advisory [`Failure`] — the bridge never hard-fails the lint run.
pub(crate) fn invoke(program: &str, path: &str) -> Result<Vec<SpkIssue>, Failure> {
    let output = crate::runner::spawn_with_text_busy_retry(program, &["lint", path, "--json"])
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                Failure::Unavailable(format!("spk binary not found on PATH ('{program}')"))
            }
            _ => Failure::Broken(format!("failed to invoke spk lint: {e}")),
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: SpkEnvelope = serde_json::from_str(stdout.trim())
        .map_err(|e| Failure::Broken(format!("spk lint produced unparseable output: {e}")))?;
    if !envelope.ok {
        return Err(Failure::Broken(format!(
            "spk lint failed: {}",
            failure_detail(&envelope.warnings)
        )));
    }
    Ok(envelope.data.map(|d| d.issues).unwrap_or_default())
}

/// First spk warning message, or `"unknown error"` when spk reported none.
fn failure_detail(warnings: &[SpkWarning]) -> String {
    warnings
        .first()
        .map(|w| w.message.clone())
        .unwrap_or_else(|| "unknown error".to_string())
}
