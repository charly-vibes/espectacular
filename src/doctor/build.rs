//! Assembles the check list handed to genesis' doctor runner.

use super::checks::{
    ChangesDirCheck, CollisionCheck, ConfigCheck, ManagedBlockCheck, SpecsDirCheck,
    VersionDriftCheck,
};
use super::corpus::{OrphanContractCheck, UnknownArchetypeCheck};
use super::hooks::{HookCheck, HookWiredCheck, MutationOwnershipCheck};
use crate::config;
use genesis::doctor::DoctorCheck;
use std::path::Path;

pub(super) fn build_checks(repo_root: &Path) -> Vec<Box<dyn DoctorCheck>> {
    let mut checks = base_checks();
    if let Ok(cfg) = config::load(repo_root) {
        checks.extend(config_checks(repo_root, &cfg));
    }
    checks
}

/// Checks that run without reading the repo config.
fn base_checks() -> Vec<Box<dyn DoctorCheck>> {
    let mut checks: Vec<Box<dyn DoctorCheck>> = Vec::new();
    checks.push(Box::new(ConfigCheck));
    checks.push(Box::new(HookCheck));
    checks.push(Box::new(HookWiredCheck));
    for &filename in &["AGENTS.md", "CLAUDE.md"] {
        checks.push(Box::new(ManagedBlockCheck { filename }));
    }
    checks
}

/// Checks parameterised by the loaded config.
fn config_checks(repo_root: &Path, cfg: &config::Config) -> Vec<Box<dyn DoctorCheck>> {
    let specs_dir = repo_root.join(&cfg.paths.specs);
    vec![
        Box::new(VersionDriftCheck {
            config: cfg.clone(),
        }),
        Box::new(MutationOwnershipCheck {
            config: cfg.clone(),
        }),
        Box::new(SpecsDirCheck {
            specs_dir: specs_dir.clone(),
        }),
        Box::new(ChangesDirCheck {
            changes_dir: repo_root.join(&cfg.paths.changes),
        }),
        Box::new(CollisionCheck {
            specs_dir: specs_dir.clone(),
        }),
        Box::new(OrphanContractCheck {
            repo_root: repo_root.to_path_buf(),
            specs_dir: specs_dir.clone(),
        }),
        Box::new(UnknownArchetypeCheck {
            repo_root: repo_root.to_path_buf(),
            specs_dir,
        }),
    ]
}
