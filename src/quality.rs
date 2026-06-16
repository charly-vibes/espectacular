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
) -> Vec<QualityFinding> {
    let mut findings = Vec::new();
    if let Some(mutation) = &config.mutation {
        if mutation.enabled {
            if let Some(finding) = mutation_finding(repo_root, mutation, scope) {
                findings.push(finding);
            }
        }
    }
    findings
}

fn mutation_finding(
    repo_root: &Path,
    mutation: &crate::config::MutationConfig,
    scope: &str,
) -> Option<QualityFinding> {
    if scope == "pre-commit" {
        return None;
    }
    let kill_rate = run_mutation_tool(repo_root, mutation)?;
    if kill_rate >= mutation.threshold {
        return None;
    }
    Some(QualityFinding {
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
    })
}

fn run_mutation_tool(repo_root: &Path, mutation: &crate::config::MutationConfig) -> Option<f64> {
    if mutation.command.is_empty() {
        return None;
    }
    let (prog, args) = mutation.command.split_first()?;
    let output = std::process::Command::new(prog)
        .args(args)
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = std::str::from_utf8(&output.stdout).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    parsed["kill_rate"].as_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MutationConfig, QualityConfig};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn write_mutation_runner(dir: &std::path::Path, kill_rate: f64) -> std::path::PathBuf {
        let script = dir.join("mutation-runner.sh");
        fs::write(
            &script,
            format!("#!/bin/sh\nprintf '{{\"kill_rate\": {}}}'", kill_rate),
        )
        .unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
        script
    }

    // 8.1 RED: mutation finding emitted when enabled and below threshold

    #[test]
    fn mutation_finding_emitted_when_below_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_mutation_runner(dir.path(), 0.60);
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: vec![script.to_string_lossy().to_string()],
            }),
        };
        let findings = collect_quality_findings(dir.path(), &config, "full");
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
        let script = write_mutation_runner(dir.path(), 0.60);
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: vec![script.to_string_lossy().to_string()],
            }),
        };
        let findings = collect_quality_findings(dir.path(), &config, "full");
        let f = &findings[0];
        assert!((f.kill_rate.unwrap() - 0.60).abs() < 1e-9);
        assert!((f.threshold.unwrap() - 0.80).abs() < 1e-9);
    }

    #[test]
    fn mutation_finding_carries_suggested_action_and_playbook_command() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_mutation_runner(dir.path(), 0.50);
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: vec![script.to_string_lossy().to_string()],
            }),
        };
        let findings = collect_quality_findings(dir.path(), &config, "full");
        let f = &findings[0];
        assert!(!f.suggested_action.is_empty());
        assert!(!f.playbook_command.is_empty());
    }

    #[test]
    fn no_mutation_finding_when_above_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_mutation_runner(dir.path(), 0.90);
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: vec![script.to_string_lossy().to_string()],
            }),
        };
        let findings = collect_quality_findings(dir.path(), &config, "full");
        assert!(
            findings.is_empty(),
            "no finding expected when kill rate meets threshold"
        );
    }

    #[test]
    fn no_mutation_finding_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_mutation_runner(dir.path(), 0.50);
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: false,
                threshold: 0.80,
                command: vec![script.to_string_lossy().to_string()],
            }),
        };
        let findings = collect_quality_findings(dir.path(), &config, "full");
        assert!(
            findings.is_empty(),
            "disabled mutation must not emit finding"
        );
    }

    // 8.3 RED: mutation skipped in pre-commit scope

    #[test]
    fn mutation_skipped_in_precommit_scope() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_mutation_runner(dir.path(), 0.10);
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: vec![script.to_string_lossy().to_string()],
            }),
        };
        let findings = collect_quality_findings(dir.path(), &config, "pre-commit");
        assert!(
            findings.is_empty(),
            "mutation must be skipped in pre-commit scope"
        );
    }

    #[test]
    fn mutation_findings_deterministically_ordered() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_mutation_runner(dir.path(), 0.60);
        let config = QualityConfig {
            mutation: Some(MutationConfig {
                enabled: true,
                threshold: 0.80,
                command: vec![script.to_string_lossy().to_string()],
            }),
        };
        let a = collect_quality_findings(dir.path(), &config, "full");
        let b = collect_quality_findings(dir.path(), &config, "full");
        assert_eq!(a, b, "findings must be deterministic");
    }
}
