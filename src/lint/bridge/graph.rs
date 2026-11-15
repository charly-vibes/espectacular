//! Additive `spk graph` relay (task 5.2): typing violations and dangling
//! references become `spk.graph.*` warnings.

use crate::lint::walker::SpecFile;
use crate::lint::LintFinding;
use serde::Deserialize;

#[derive(Deserialize)]
struct SpkGraphEnvelope {
    #[serde(default)]
    data: Option<SpkGraphData>,
}

#[derive(Deserialize)]
struct SpkGraphData {
    #[serde(default)]
    violations: Vec<SpkGraphViolation>,
    #[serde(default)]
    dangling: Vec<String>,
}

#[derive(Deserialize)]
struct SpkGraphViolation {
    #[serde(default)]
    edge_kind: String,
    #[serde(default)]
    from: String,
    #[serde(default)]
    to: String,
    #[serde(default)]
    reason: String,
}

/// Relay `spk graph` typing violations and dangling references as
/// `spk.graph.*` warnings. Degrades silently on invocation failure — the lint
/// relay already surfaces an spk-unavailable/spk-bridge-failure advisory, and
/// graph is additive signal (design D2: ah consumes, spk owns validity).
pub(crate) fn relay_graph(
    program: &str,
    spec: &SpecFile,
    path: &str,
    findings: &mut Vec<LintFinding>,
) {
    let Some(data) = graph_data(program, path) else {
        return;
    };
    push_violations(spec, &data.violations, findings);
    push_dangling(spec, &data.dangling, findings);
}

/// Run `spk graph <path> --json`. Any spawn/decode failure is `None`: graph
/// signal is additive, never gating.
fn graph_data(program: &str, path: &str) -> Option<SpkGraphData> {
    let output =
        crate::runner::spawn_with_text_busy_retry(program, &["graph", path, "--json"]).ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: SpkGraphEnvelope = serde_json::from_str(stdout.trim()).ok()?;
    envelope.data
}

fn push_violations(
    spec: &SpecFile,
    violations: &[SpkGraphViolation],
    findings: &mut Vec<LintFinding>,
) {
    for v in violations {
        findings.push(LintFinding::warning(
            "spk.graph.typing",
            &spec.spec_path,
            "",
            &format!(
                "typing violation on {} {} → {}: {}",
                v.edge_kind, v.from, v.to, v.reason
            ),
            "resolve the spk graph typing violation — the cited row has the wrong kind for this edge",
            "edit_spec",
            "ah explain lint",
        ));
    }
}

fn push_dangling(spec: &SpecFile, dangling: &[String], findings: &mut Vec<LintFinding>) {
    for d in dangling {
        findings.push(LintFinding::warning(
            "spk.graph.dangling",
            &spec.spec_path,
            "",
            d,
            "resolve the dangling reference — the target is not defined in any spec file",
            "edit_spec",
            "ah explain lint",
        ));
    }
}
