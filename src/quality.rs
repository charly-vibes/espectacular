//! Quality-signal collection: mutation kill rate plus the suite-trio tools
//! (vampiro/crua/livin). Every tool is advisory — failures become tool errors,
//! never hard gate failures.

mod mutation;
mod trio;

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
            match mutation::mutation_finding(repo_root, mutation, scope) {
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
                match trio::trio_finding(repo_root, kind, cfg, scope) {
                    Ok(Some(finding)) => findings.push(finding),
                    Ok(None) => {}
                    Err(msg) => tool_errors.push(msg),
                }
            }
        }
    }
    (findings, tool_errors)
}
