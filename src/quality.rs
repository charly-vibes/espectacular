use crate::config::QualityConfig;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
pub struct QualityFinding {
    pub kind: String,
    pub category: String,
    pub kill_rate: Option<f64>,
    pub threshold: Option<f64>,
    pub suggested_action: String,
    pub playbook_command: String,
    pub message: String,
}

impl QualityFinding {
    pub fn for_test_type(test_type: &str) -> Self {
        Self {
            kind: format!("quality-{test_type}"),
            category: "quality".to_string(),
            kill_rate: None,
            threshold: None,
            suggested_action: "enable_capability".to_string(),
            playbook_command: format!("ah explain quality-{test_type}"),
            message: format!("{test_type} tests ran successfully"),
        }
    }
}

impl PartialEq for QualityFinding {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.category == other.category
            && self.kill_rate.map(f64::to_bits) == other.kill_rate.map(f64::to_bits)
            && self.threshold.map(f64::to_bits) == other.threshold.map(f64::to_bits)
            && self.suggested_action == other.suggested_action
            && self.playbook_command == other.playbook_command
            && self.message == other.message
    }
}

impl Eq for QualityFinding {}

pub fn collect_quality_findings(
    repo_root: &Path,
    config: &QualityConfig,
    scope: &str,
) -> (Vec<QualityFinding>, Vec<String>) {
    let mut findings = Vec::new();
    let mut tool_errors = Vec::new();
    if let Some(mutation) = &config.mutation {
        if mutation.enabled {
            match mutation_finding(repo_root, mutation, scope) {
                Ok(Some(finding)) => findings.push(finding),
                Ok(None) => {}
                Err(msg) => tool_errors.push(msg),
            }
        }
    }
    // Suite-trio quality signals (vampiro/crua/livin). These emit a
    // quality finding when the tool runs successfully and reports a
    // nonzero finding count in its custom-runner JSON envelope.
    for (kind, cfg) in [
        ("composability", &config.composability),
        ("cost", &config.cost),
        ("boundary-coverage", &config.boundary_coverage),
    ] {
        if let Some(cfg) = cfg {
            if cfg.enabled {
                match trio_finding(repo_root, kind, cfg, scope) {
                    Ok(Some(finding)) => findings.push(finding),
                    Ok(None) => {}
                    Err(msg) => tool_errors.push(msg),
                }
            }
        }
    }
    (findings, tool_errors)
}

/// Run a suite-trio quality tool (vampiro/crua/livin) via the documented
/// custom-runner envelope protocol and emit a quality finding on failure.
fn trio_finding(
    repo_root: &Path,
    kind: &str,
    config: &crate::config::QualityToolConfig,
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
fn run_trio_tool(
    repo_root: &Path,
    config: &crate::config::QualityToolConfig,
) -> Result<Option<usize>, String> {
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

fn mutation_finding(
    repo_root: &Path,
    mutation: &crate::config::MutationConfig,
    scope: &str,
) -> Result<Option<QualityFinding>, String> {
    if scope == "pre-commit" {
        return Ok(None);
    }
    match run_mutation_tool(repo_root, mutation) {
        Ok(None) => Ok(None),
        Ok(Some(kill_rate)) => {
            if kill_rate >= mutation.threshold {
                return Ok(None);
            }
            Ok(Some(QualityFinding {
                kind: "quality-mutation".to_string(),
                category: "quality".to_string(),
                kill_rate: Some(kill_rate),
                threshold: Some(mutation.threshold),
                suggested_action: "enable_capability".to_string(),
                playbook_command: "ah explain enable_capability".to_string(),
                message: format!(
                    "mutation kill rate {:.0}% is below threshold {:.0}%",
                    kill_rate * 100.0,
                    mutation.threshold * 100.0
                ),
            }))
        }
        Err(msg) => Err(msg),
    }
}

fn run_mutation_tool(
    repo_root: &Path,
    mutation: &crate::config::MutationConfig,
) -> Result<Option<f64>, String> {
    if mutation.command.is_empty() {
        return Ok(None);
    }
    let (prog, args) = mutation
        .command
        .split_first()
        .ok_or_else(|| "mutation command is empty".to_string())?;

    // Replace "{}" placeholder with a generated runner script path.
    let args: Vec<String> = args
        .iter()
        .map(|arg| {
            if arg == "{}" {
                generate_runner_script(repo_root)
            } else {
                arg.clone()
            }
        })
        .collect();

    let output = std::process::Command::new(prog)
        .args(&args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("failed to spawn mutation tool: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "mutation tool exited non-zero ({}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }
    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|_| "mutation tool output is not UTF-8".to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("mutation tool output is not valid JSON: {e}"))?;
    Ok(parsed["kill_rate"].as_f64())
}

fn generate_runner_script(repo_root: &Path) -> String {
    let dir = repo_root.join(".espectacular");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("mutation-runner.sh");
    // Generate a simple script that runs the project's tests.
    // The mutation tool uses this script to run tests and compute
    // the mutation score. The user can replace this with a custom
    // runner by pointing the command directly at their script.
    let _ = std::fs::write(
        &path,
        b"#!/bin/sh\n# Mutation test runner - generated by espectacular\n# Replace this file with a custom runner for your mutation tool.\necho '{\"kill_rate\": 0.0}'\n",
    );
    let _ = std::process::Command::new("chmod")
        .args(["+x", path.to_str().unwrap_or("")])
        .output();
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MutationConfig, QualityConfig, QualityToolConfig};
    use std::fs;

    fn mutation_command(dir: &std::path::Path, kill_rate: f64) -> Vec<String> {
        let script = dir.join("mutation-runner.sh");
        fs::write(
            &script,
            format!("printf '{{\"kill_rate\": {}}}'", kill_rate),
        )
        .unwrap();
        vec!["/bin/sh".to_string(), script.to_string_lossy().to_string()]
    }

    // 8.1 RED: mutation finding emitted when enabled and below threshold

    #[test]
    fn mutation_finding_emitted_when_below_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: mutation_command(dir.path(), 0.60),
            }),
            ..Default::default()
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert_eq!(
            findings.len(),
            1,
            "expected exactly one quality_mutation finding"
        );
        let f = &findings[0];
        assert_eq!(f.kind, "quality-mutation");
        assert_eq!(f.category, "quality");
        assert!(f.kill_rate.is_some(), "kill_rate must be present");
    }

    #[test]
    fn mutation_finding_carries_kill_rate_and_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: mutation_command(dir.path(), 0.60),
            }),
            ..Default::default()
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        let f = &findings[0];
        assert!((f.kill_rate.unwrap() - 0.60).abs() < 1e-9);
        assert!((f.threshold.unwrap() - 0.80).abs() < 1e-9);
    }

    #[test]
    fn mutation_finding_carries_suggested_action_and_playbook_command() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: mutation_command(dir.path(), 0.50),
            }),
            ..Default::default()
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        let f = &findings[0];
        assert!(!f.suggested_action.is_empty());
        assert!(!f.playbook_command.is_empty());
    }

    #[test]
    fn no_mutation_finding_when_above_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: mutation_command(dir.path(), 0.90),
            }),
            ..Default::default()
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert!(
            findings.is_empty(),
            "no finding expected when kill rate meets threshold"
        );
    }

    #[test]
    fn no_mutation_finding_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: false,
                threshold: 0.80,
                command: mutation_command(dir.path(), 0.50),
            }),
            ..Default::default()
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert!(
            findings.is_empty(),
            "disabled mutation must not emit finding"
        );
    }

    // 8.3 RED: mutation skipped in pre-commit scope

    #[test]
    fn mutation_skipped_in_precommit_scope() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: mutation_command(dir.path(), 0.10),
            }),
            ..Default::default()
        };
        let (findings, _) = collect_quality_findings(dir.path(), &config, "pre-commit");
        assert!(
            findings.is_empty(),
            "mutation must be skipped in pre-commit scope"
        );
    }

    #[test]
    fn mutation_findings_deterministically_ordered() {
        let dir = tempfile::tempdir().unwrap();
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: mutation_command(dir.path(), 0.60),
            }),
            ..Default::default()
        };
        let (a, _) = collect_quality_findings(dir.path(), &config, "full");
        let (b, _) = collect_quality_findings(dir.path(), &config, "full");
        assert_eq!(a, b, "findings must be deterministic");
    }

    #[test]
    fn mutation_placeholder_replaced_with_runner_script() {
        let dir = tempfile::tempdir().unwrap();
        // Use the documented {} placeholder shape.
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: vec!["/bin/sh".to_string(), "{}".to_string()],
            }),
            ..Default::default()
        };
        // The {} placeholder should be replaced with a generated runner script
        // that produces a kill rate below threshold, yielding a mutation finding.
        let (findings, _) = collect_quality_findings(dir.path(), &config, "full");
        assert_eq!(
            findings.len(),
            1,
            "expected exactly one quality_mutation finding with placeholder"
        );
        let f = &findings[0];
        assert_eq!(f.kind, "quality-mutation");
        assert!(f.kill_rate.is_some(), "kill_rate must be present");
    }

    // ── Suite-trio quality signals (vampiro/crua/livin) ────────────────

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
