use super::QualityFinding;
use crate::config::QualityToolConfig;
use std::path::Path;

/// Run a suite-trio quality tool (vampiro/crua/livin) via the documented
/// custom-runner envelope protocol and emit a quality finding on failure.
pub(super) fn trio_finding(
    repo_root: &Path,
    kind: &str,
    config: &QualityToolConfig,
    scope: &str,
) -> Result<Option<QualityFinding>, String> {
    if scope == "pre-commit" {
        return Ok(None);
    }
    match run_trio_tool(repo_root, config) {
        Ok(None) => Ok(None),
        Ok(Some(finding_count)) => {
            if finding_count == 0 {
                return Ok(None);
            }
            Ok(Some(QualityFinding {
                kind: format!("quality-{kind}"),
                category: "quality".to_string(),
                kill_rate: None,
                threshold: None,
                suggested_action: "enable_capability".to_string(),
                playbook_command: "ah explain enable_capability".to_string(),
                message: format!("{kind} tool reported {finding_count} finding(s)"),
            }))
        }
        Err(msg) => Err(msg),
    }
}

/// Run a quality tool and parse its custom-runner JSON envelope.
/// Returns the reported finding count (or None if absent).
fn run_trio_tool(repo_root: &Path, config: &QualityToolConfig) -> Result<Option<usize>, String> {
    if config.command.is_empty() {
        return Ok(None);
    }
    let (prog, args) = config
        .command
        .split_first()
        .ok_or_else(|| "quality tool command is empty".to_string())?;
    let output = std::process::Command::new(prog)
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("failed to spawn quality tool {prog}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "quality tool {prog} exited non-zero ({}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }
    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|_| "quality tool output is not UTF-8".to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("quality tool output is not valid JSON: {e}"))?;
    // Custom-runner envelope protocol: {exit_code, passed, findings}.
    let count = parsed["findings"]
        .as_array()
        .map(|v| v.len())
        .or_else(|| parsed["findings"].as_u64().map(|n| n as usize));
    Ok(count)
}

#[cfg(test)]
mod tests {
    use crate::config::{QualityConfig, QualityToolConfig};
    use crate::quality::collect_quality_findings;
    use std::fs;

    fn trio_script(dir: &std::path::Path, findings: &[serde_json::Value]) -> Vec<String> {
        let script = dir.join("trio-runner.sh");
        let payload = serde_json::json!({
            "exit_code": 0,
            "passed": findings.len(),
            "findings": findings,
        });
        fs::write(
            &script,
            format!("printf '{}'", serde_json::to_string(&payload).unwrap()),
        )
        .unwrap();
        vec!["/bin/sh".to_string(), script.to_string_lossy().to_string()]
    }

    #[test]
    fn composability_finding_emitted_when_tool_reports_findings() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: None,
            composability: Some(QualityToolConfig {
                enabled: true,
                command: trio_script(dir.path(), &[serde_json::json!({})]),
            }),
            cost: None,
            boundary_coverage: None,
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert_eq!(
            findings.len(),
            1,
            "expected one quality-composability finding"
        );
        assert_eq!(findings[0].kind, "quality-composability");
    }

    #[test]
    fn cost_finding_emitted_when_tool_reports_findings() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: None,
            composability: None,
            cost: Some(QualityToolConfig {
                enabled: true,
                command: trio_script(dir.path(), &[serde_json::json!({})]),
            }),
            boundary_coverage: None,
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert_eq!(findings.len(), 1, "expected one quality-cost finding");
        assert_eq!(findings[0].kind, "quality-cost");
    }

    #[test]
    fn boundary_coverage_finding_emitted_when_tool_reports_findings() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: None,
            composability: None,
            cost: None,
            boundary_coverage: Some(QualityToolConfig {
                enabled: true,
                command: trio_script(dir.path(), &[serde_json::json!({})]),
            }),
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert_eq!(
            findings.len(),
            1,
            "expected one quality-boundary-coverage finding"
        );
        assert_eq!(findings[0].kind, "quality-boundary-coverage");
    }

    #[test]
    fn trio_finding_not_emitted_when_tool_reports_zero_findings() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: None,
            composability: Some(QualityToolConfig {
                enabled: true,
                command: trio_script(dir.path(), &[]),
            }),
            cost: None,
            boundary_coverage: None,
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert!(
            findings.is_empty(),
            "no finding expected when tool reports zero findings"
        );
    }

    #[test]
    fn trio_finding_skipped_in_precommit_scope() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: None,
            composability: Some(QualityToolConfig {
                enabled: true,
                command: trio_script(dir.path(), &[serde_json::json!({})]),
            }),
            cost: None,
            boundary_coverage: None,
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "pre-commit");
        assert!(
            findings.is_empty(),
            "trio quality signals must be skipped in pre-commit scope"
        );
    }

    #[test]
    fn trio_finding_not_emitted_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: None,
            composability: Some(QualityToolConfig {
                enabled: false,
                command: trio_script(dir.path(), &[serde_json::json!({})]),
            }),
            cost: None,
            boundary_coverage: None,
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert!(findings.is_empty(), "disabled trio must not emit finding");
    }

    #[test]
    fn trio_tool_exit_nonzero_emits_tool_error() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fail.sh");
        fs::write(&script, "#!/bin/sh\nexit 1").unwrap();
        std::process::Command::new("chmod")
            .args(["+x", script.to_str().unwrap()])
            .output()
            .unwrap();
        let config = QualityConfig {
            mutation: None,
            composability: Some(QualityToolConfig {
                enabled: true,
                command: vec!["/bin/sh".to_string(), script.to_string_lossy().to_string()],
            }),
            cost: None,
            boundary_coverage: None,
        };
        let (findings, errors) = collect_quality_findings(dir.path(), &config, "full");
        assert!(findings.is_empty(), "no finding on tool failure");
        assert!(!errors.is_empty(), "tool failure must produce a tool error");
    }
}
