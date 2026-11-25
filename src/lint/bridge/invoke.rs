//! Process seam for `spk`: invoke the binary and decode its envelopes.
//!
//! Every failure mode maps to an advisory [`Failure`] — the bridge never
//! hard-fails the lint run.

use serde::Deserialize;
use std::path::Path;

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

#[derive(Deserialize, Clone)]
pub(crate) struct SpkIssue {
    pub(crate) rule_id: String,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) rule_semantics: String,
    /// Dir-mode envelopes attribute each issue to its file (echoing the
    /// invocation arg form); per-file envelopes may omit it. `None` triggers
    /// the named per-file fallback in the relay (espectacular-hcz).
    #[serde(default)]
    pub(crate) file: Option<String>,
}

pub(crate) enum Failure {
    Unavailable(String),
    Broken(String),
}

/// Per-issue `file` field vs walked source path, modulo `./` noise. The
/// envelope echoes the invocation arg form, and callers pass the same root
/// the walker used — so clean component comparison suffices; anything else
/// lands in the caller's unmatched bucket (named signal, never dropped).
pub(crate) fn issue_matches_path(issue: &SpkIssue, source_path: &std::path::Path) -> bool {
    let Some(file) = issue.file.as_deref() else {
        return false;
    };
    normalize(Path::new(file)) == normalize(source_path)
}

fn normalize(p: &Path) -> std::path::PathBuf {
    p.components()
        .filter(|c| !matches!(c, std::path::Component::CurDir))
        .collect()
}

/// Invoke `spk lint <path> --json` and extract its issues. Every failure mode
/// maps to an advisory [`Failure`] — the bridge never hard-fails the lint run.
pub(crate) fn invoke(program: &str, path: &str) -> Result<Vec<SpkIssue>, Failure> {
    let output = spawn_lint(program, path)?;
    decode_issues(&output)
}

/// Invoke `spk lint <dir> --json` ONCE for a whole linted root
/// (espectacular-hcz): one corpus-context scan per root instead of one per
/// spec file. Same failure mapping as [`invoke`]; issue attribution relies
/// on the per-issue `file` field.
pub(crate) fn invoke_dir(program: &str, dir: &std::path::Path) -> Result<Vec<SpkIssue>, Failure> {
    let output = spawn_lint(program, &dir.to_string_lossy())?;
    decode_issues(&output)
}

fn spawn_lint(program: &str, path: &str) -> Result<std::process::Output, Failure> {
    crate::runner::spawn_with_text_busy_retry(program, &["lint", path, "--json"]).map_err(|e| {
        match e.kind() {
            std::io::ErrorKind::NotFound => {
                Failure::Unavailable(format!("spk binary not found on PATH ('{program}')"))
            }
            _ => Failure::Broken(format!("failed to invoke spk lint: {e}")),
        }
    })
}

fn decode_issues(output: &std::process::Output) -> Result<Vec<SpkIssue>, Failure> {
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
