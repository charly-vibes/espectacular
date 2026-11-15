//! Config-shape, path, collision, and managed-block checks.

use super::types::TOOL_VERSION;
use crate::config;
use crate::init::ah_block_injector;
use crate::openspec;
use genesis::doctor::DoctorCheck;
use genesis::suite_linter::{LintResult, Severity};
use std::path::Path;

pub(super) struct ConfigCheck;
impl DoctorCheck for ConfigCheck {
    fn name(&self) -> &'static str {
        "config"
    }
    fn description(&self) -> &'static str {
        "Validate .espectacular/config.toml"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        match config::load(repo_root) {
            Ok(_) => Ok(vec![]),
            Err(e) => Ok(vec![LintResult::new(
                format!("bad config: {e:#}"),
                Severity::Error,
            )]),
        }
    }
}

pub(super) struct VersionDriftCheck {
    pub(super) config: config::Config,
}
impl DoctorCheck for VersionDriftCheck {
    fn name(&self) -> &'static str {
        "version-drift"
    }
    fn description(&self) -> &'static str {
        "Check that config tool_version matches binary version"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.config.tool_version == TOOL_VERSION {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                format!(
                    "config tool_version {} does not match binary version {TOOL_VERSION}",
                    self.config.tool_version
                ),
                Severity::Error,
            )])
        }
    }
}

pub(super) struct SpecsDirCheck {
    pub(super) specs_dir: std::path::PathBuf,
}
impl DoctorCheck for SpecsDirCheck {
    fn name(&self) -> &'static str {
        "missing-specs-dir"
    }
    fn description(&self) -> &'static str {
        "Check that the specs directory exists"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.specs_dir.exists() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                format!("specs directory not found: {}", self.specs_dir.display()),
                Severity::Error,
            )])
        }
    }
}

pub(super) struct ChangesDirCheck {
    pub(super) changes_dir: std::path::PathBuf,
}
impl DoctorCheck for ChangesDirCheck {
    fn name(&self) -> &'static str {
        "missing-changes-dir"
    }
    fn description(&self) -> &'static str {
        "Check that the changes directory exists"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.changes_dir.exists() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                format!(
                    "changes directory not found: {}",
                    self.changes_dir.display()
                ),
                Severity::Error,
            )])
        }
    }
}

pub(super) struct CollisionCheck {
    pub(super) specs_dir: std::path::PathBuf,
}
impl DoctorCheck for CollisionCheck {
    fn name(&self) -> &'static str {
        "scenario-collisions"
    }
    fn description(&self) -> &'static str {
        "Check for duplicate scenario slugs"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if !self.specs_dir.exists() {
            return Ok(vec![]);
        }
        let specs_str = self.specs_dir.to_string_lossy().to_string();
        let scenarios = match openspec::discover_scenarios(&specs_str) {
            Ok(s) => s,
            Err(_) => return Ok(vec![]),
        };
        let collisions = openspec::detect_slug_collisions(&scenarios);
        Ok(collisions
            .into_iter()
            .map(|(spec, slug, _heading)| {
                LintResult::new(
                    format!("duplicate scenario slug '{slug}' in spec '{spec}'"),
                    Severity::Error,
                )
            })
            .collect())
    }
}

pub(super) struct ManagedBlockCheck {
    pub(super) filename: &'static str,
}
impl DoctorCheck for ManagedBlockCheck {
    fn name(&self) -> &'static str {
        "managed-block"
    }
    fn description(&self) -> &'static str {
        "Check that agent files have the ah managed block"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let path = repo_root.join(self.filename);
        if path.exists() && !ah_block_injector().has_block(&path, "ah:managed") {
            Ok(vec![LintResult::new(
                format!(
                    "{filename} is missing the ah managed block",
                    filename = self.filename
                ),
                Severity::Error,
            )])
        } else {
            Ok(vec![])
        }
    }
}
