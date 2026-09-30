//! Dual-format lint bridge (espectacular-zlw).
//!
//! Purpose: relay specodelic (`spk`) findings for dual-format spec files into
//! the shared lint finding schema by invoking `spk lint` — never
//! reimplementing specodelic rules.
//!
//! Responsibilities:
//! - detect dual-format spec files (`id: spec` YAML frontmatter)
//! - invoke `spk lint <file> --json` and map `data.issues` to findings with
//!   `kind = "spk.<rule_id>"` at warning severity
//! - degrade advisory-only: missing binary → one `spk-unavailable` finding;
//!   broken invocation/parse → one `spk-bridge-failure` finding; plain
//!   openspec files are never touched
//!
//! Rationale: delta spec "Requirement: Dual-Format Lint Bridge" in
//! `openspec/changes/add-spec-quality-checks/specs/lint/spec.md`.

use crate::lint::walker::SpecFile;
use crate::lint::LintFinding;
use serde::Deserialize;
use std::process::Command;

const SPK_PROGRAM: &str = "spk";

/// A dual-format spec file carries `id: spec` in its YAML frontmatter
/// (repo convention from adopt-dual-format-specs).
pub(crate) fn is_dual_format(raw: &str) -> bool {
    let Some(rest) = raw.strip_prefix("---") else {
        return false;
    };
    let Some(end) = rest.find("\n---") else {
        return false;
    };
    rest[..end].lines().any(|l| l.trim() == "id: spec")
}

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
struct SpkIssue {
    rule_id: String,
    message: String,
    #[serde(default)]
    rule_semantics: String,
}

/// Relay spk findings for every dual-format spec. Plain openspec files never
/// trigger an invocation.
pub fn relay_dual_format(specs: &[SpecFile], findings: &mut Vec<LintFinding>) {
    relay_with(SPK_PROGRAM, specs, findings);
}

/// Test seam: same relay against an explicit spk program path.
pub(crate) fn relay_with(program: &str, specs: &[SpecFile], findings: &mut Vec<LintFinding>) {
    for spec in specs {
        if !is_dual_format(&spec.raw) {
            continue;
        }
        let Some(path) = spec.source_path.to_str() else {
            continue;
        };
        match invoke(program, path) {
            Ok(issues) => {
                for issue in issues {
                    findings.push(LintFinding::warning(
                        &format!("spk.{}", issue.rule_id),
                        &spec.spec_path,
                        "",
                        &issue.message,
                        &if issue.rule_semantics.is_empty() {
                            "address the specodelic invariant".to_string()
                        } else {
                            issue.rule_semantics
                        },
                        "edit_spec",
                        "ah explain lint",
                    ));
                }
            }
            Err(Failure::Unavailable(msg)) => findings.push(LintFinding::warning(
                "spk-unavailable",
                &spec.spec_path,
                "",
                &msg,
                "install specodelic to lint dual-format specs",
                "install_tool",
                "ah explain lint",
            )),
            Err(Failure::Broken(msg)) => findings.push(LintFinding::warning(
                "spk-bridge-failure",
                &spec.spec_path,
                "",
                &msg,
                "inspect the spk invocation failure; relayed findings are skipped",
                "edit_spec",
                "ah explain lint",
            )),
        }
    }
}

enum Failure {
    Unavailable(String),
    Broken(String),
}

/// Invoke `spk lint <path> --json` and extract its issues. Every failure mode
/// maps to an advisory [`Failure`] — the bridge never hard-fails the lint run.
fn invoke(program: &str, path: &str) -> Result<Vec<SpkIssue>, Failure> {
    let output = Command::new(program)
        .args(["lint", path, "--json"])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Failure::Unavailable(format!("spk binary not found on PATH ('{program}')"))
            } else {
                Failure::Broken(format!("failed to invoke spk lint: {e}"))
            }
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: SpkEnvelope = serde_json::from_str(stdout.trim())
        .map_err(|e| Failure::Broken(format!("spk lint produced unparseable output: {e}")))?;
    if !envelope.ok {
        let detail = envelope
            .warnings
            .first()
            .map(|w| w.message.clone())
            .unwrap_or_else(|| "unknown error".to_string());
        return Err(Failure::Broken(format!("spk lint failed: {detail}")));
    }
    Ok(envelope.data.map(|d| d.issues).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    //! Shim-based tests substitute a fake `spk` program so the suite is
    //! deterministic without the real binary; the real-spk end-to-end test
    //! (lint.rs) skips gracefully when spk is not installed.

    use super::*;
    use crate::lint::walker;
    use std::path::PathBuf;

    fn spec_with_raw(name: &str, raw: &str) -> SpecFile {
        let mut spec = walker::parse_spec(raw, name);
        spec.source_path = PathBuf::from("/virtual/spec.md");
        spec
    }

    fn write_shim(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    const VALID_ENVELOPE: &str = r#"#!/bin/sh
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"files_linted":1,"issues":[{"file":"spec.md","message":"intent statement has no imperative","rule_id":"linter.ears_syntax","rule_semantics":"statement must match an EARS pattern"}]},"warnings":[],"hints":[],"meta":{}}'
"#;

    const GARBAGE: &str = "#!/bin/sh\necho 'not json at all'\n";

    const DUAL_RAW: &str = "---\nid: spec\nkind: intent\nstatement: \"do it\"\n---\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

    const PLAIN_RAW: &str =
        "# Capability: auth\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

    #[test]
    fn is_dual_format_detects_id_spec_frontmatter() {
        assert!(is_dual_format(DUAL_RAW));
    }

    #[test]
    fn is_dual_format_ignores_plain_and_other_ids() {
        assert!(!is_dual_format(PLAIN_RAW));
        assert!(!is_dual_format("---\nid: auth.login\n---\n\nbody\n"));
        assert!(!is_dual_format("id: spec\n"));
    }

    #[test]
    fn relay_maps_spk_issues_to_spk_prefixed_warning_findings() {
        let dir = tempfile::tempdir().unwrap();
        let shim = write_shim(dir.path(), "spk", VALID_ENVELOPE);
        let specs = vec![spec_with_raw("auth", DUAL_RAW)];
        let mut findings = Vec::new();
        relay_with(shim.to_str().unwrap(), &specs, &mut findings);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.kind, "spk.linter.ears_syntax");
        assert_eq!(f.severity, crate::lint::Severity::Warning);
        assert_eq!(f.spec_path, "auth");
        assert!(f.message.contains("no imperative"));
    }

    #[test]
    fn relay_emits_single_advisory_when_spk_is_missing() {
        let specs = vec![spec_with_raw("auth", DUAL_RAW)];
        let mut findings = Vec::new();
        relay_with("/nonexistent/path/spk", &specs, &mut findings);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "spk-unavailable");
        assert_eq!(findings[0].severity, crate::lint::Severity::Warning);
    }

    #[test]
    fn relay_emits_advisory_on_unparseable_output() {
        let dir = tempfile::tempdir().unwrap();
        let shim = write_shim(dir.path(), "spk", GARBAGE);
        let specs = vec![spec_with_raw("auth", DUAL_RAW)];
        let mut findings = Vec::new();
        relay_with(shim.to_str().unwrap(), &specs, &mut findings);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "spk-bridge-failure");
        assert_eq!(findings[0].severity, crate::lint::Severity::Warning);
    }

    #[test]
    fn relay_is_inert_for_plain_openspec_files() {
        // No frontmatter → no spk invocation: even a nonexistent program must
        // produce zero findings (proves the bridge never shelled out).
        let specs = vec![spec_with_raw("auth", PLAIN_RAW)];
        let mut findings = Vec::new();
        relay_with("/nonexistent/path/spk", &specs, &mut findings);
        assert!(findings.is_empty());
    }
}
