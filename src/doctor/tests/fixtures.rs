//! Shared fixture builders for the doctor test suite.

use super::super::types::{DoctorOutcome, TOOL_VERSION};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Healthy repo skeleton: config, agent files with the managed block, and a
/// fully wired lefthook config.
pub(in crate::doctor) fn make_healthy_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    write_config(root);
    write_agent_files(root);
    write_lefthook(root);

    dir
}

fn write_config(root: &Path) {
    fs::write(
        root.join(".espectacular/config.toml"),
        format!(
            r#"tool_version = "{}"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
"#,
            TOOL_VERSION
        ),
    )
    .unwrap();
}

fn write_agent_files(root: &Path) {
    for name in ["AGENTS.md", "CLAUDE.md"] {
        fs::write(
            root.join(name),
            format!(
                "# Project\n\n{}\n",
                crate::init::AH_BLOCK_CONTENT_WITH_MARKERS
            ),
        )
        .unwrap();
    }
}

fn write_lefthook(root: &Path) {
    fs::write(
        root.join("lefthook.yml"),
        "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\npre-push:\n  commands:\n    ah-check:\n      run: ah check\n",
    )
    .unwrap();
}

pub(super) fn base_config_toml() -> String {
    format!(
        "tool_version = \"{}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n",
        TOOL_VERSION
    )
}

/// Checks that did not pass, in the genesis report.
pub(super) fn issues(outcome: &DoctorOutcome) -> Vec<&genesis::doctor::CheckEntry> {
    outcome
        .genesis_report
        .checks
        .iter()
        .filter(|c| !c.status.is_pass())
        .collect()
}

pub(super) fn has_issue(outcome: &DoctorOutcome, kind: &str) -> bool {
    issues(outcome).iter().any(|c| c.name == kind)
}
