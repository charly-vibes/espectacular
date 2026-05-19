use crate::init::{detect_hook_framework, HookFramework, AH_BLOCK_START};
use crate::openspec;
use crate::{config, contracts};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DoctorDiagnostic {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug)]
pub struct DoctorReport {
    pub healthy: bool,
    pub diagnostics: Vec<DoctorDiagnostic>,
}

const VALID_ARCHETYPES: &[&str] = &["PF", "SA", "BP", "CE"];
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run_doctor(repo_root: &Path) -> anyhow::Result<DoctorReport> {
    let mut diagnostics: Vec<DoctorDiagnostic> = Vec::new();

    let config_path = repo_root.join(".espectacular/config.toml");
    let config_path_str = config_path.to_string_lossy().to_string();

    // Config schema validity
    let config = match config::load_config(&config_path_str) {
        Ok(c) => Some(c),
        Err(e) => {
            diagnostics.push(DoctorDiagnostic {
                kind: "bad-config".into(),
                detail: format!("{e:#}"),
            });
            None
        }
    };

    // Tool version drift (only if config loaded)
    if let Some(ref cfg) = config {
        if cfg.tool_version != TOOL_VERSION {
            diagnostics.push(DoctorDiagnostic {
                kind: "version-drift".into(),
                detail: format!(
                    "config tool_version {} does not match binary version {TOOL_VERSION}",
                    cfg.tool_version
                ),
            });
        }

        // Paths existence
        let specs_dir = repo_root.join(&cfg.paths.specs);
        if !specs_dir.exists() {
            diagnostics.push(DoctorDiagnostic {
                kind: "missing-path".into(),
                detail: format!("specs directory not found: {}", specs_dir.display()),
            });
        }
        let changes_dir = repo_root.join(&cfg.paths.changes);
        if !changes_dir.exists() {
            diagnostics.push(DoctorDiagnostic {
                kind: "missing-path".into(),
                detail: format!("changes directory not found: {}", changes_dir.display()),
            });
        }

        // Scenario collisions and orphan/archetype checks (only if specs dir exists)
        if specs_dir.exists() {
            let specs_str = specs_dir.to_string_lossy().to_string();
            if let Ok(scenarios) = openspec::discover_scenarios(&specs_str) {
                // Collisions
                let collisions = openspec::detect_slug_collisions(&scenarios);
                for (slug, _spec, _id) in &collisions {
                    diagnostics.push(DoctorDiagnostic {
                        kind: "collision".into(),
                        detail: format!("duplicate scenario slug: {slug}"),
                    });
                }

                // Build set of known scenario slugs for orphan detection
                let known_spec_slugs: HashSet<(String, String)> = scenarios
                    .iter()
                    .map(|s| (s.spec_path.clone(), s.id.clone()))
                    .collect();

                // Check each contract for unknown archetype and orphans
                let espectacular_dir = repo_root.join(".espectacular");
                if let Ok(entries) = fs::read_dir(&espectacular_dir) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if !entry_path.is_dir() {
                            continue;
                        }
                        let spec_name = entry_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                        // skip changes/ subdirectory
                        if spec_name == "changes" {
                            continue;
                        }
                        if let Ok(contract_entries) = fs::read_dir(&entry_path) {
                            for ce in contract_entries.flatten() {
                                let cp = ce.path();
                                if cp.extension().and_then(|e| e.to_str()) != Some("toml") {
                                    continue;
                                }
                                let slug = cp.file_stem().unwrap().to_string_lossy().to_string();

                                // Orphan: no matching scenario
                                if !known_spec_slugs.contains(&(spec_name.clone(), slug.clone())) {
                                    diagnostics.push(DoctorDiagnostic {
                                        kind: "orphan-contract".into(),
                                        detail: format!(
                                            "contract {}/{}.toml has no matching scenario",
                                            spec_name, slug
                                        ),
                                    });
                                    continue;
                                }

                                // Archetype check
                                if let Ok(contract) = contracts::load_contract(cp.to_str().unwrap())
                                {
                                    if !VALID_ARCHETYPES.contains(&contract.archetype.as_str()) {
                                        diagnostics.push(DoctorDiagnostic {
                                            kind: "unknown-archetype".into(),
                                            detail: format!(
                                                "{}/{}.toml has unknown archetype: {}",
                                                spec_name, slug, contract.archetype
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Managed block checks
    for filename in &["AGENTS.md", "CLAUDE.md"] {
        let path = repo_root.join(filename);
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if !content.contains(AH_BLOCK_START) {
                        diagnostics.push(DoctorDiagnostic {
                            kind: "missing-managed-block".into(),
                            detail: format!("{filename} is missing the ah managed block"),
                        });
                    }
                }
                Err(_) => {}
            }
        }
    }

    // Hook detection
    match detect_hook_framework(repo_root) {
        HookFramework::None => {
            diagnostics.push(DoctorDiagnostic {
                kind: "hook-absent".into(),
                detail: "no supported pre-commit hook framework detected (lefthook or prek)".into(),
            });
        }
        _ => {}
    }

    let healthy = diagnostics.is_empty();
    Ok(DoctorReport {
        healthy,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_healthy_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // openspec dirs
        fs::create_dir_all(root.join("openspec/specs")).unwrap();
        fs::create_dir_all(root.join("openspec/changes")).unwrap();

        // .espectacular/config.toml
        fs::create_dir_all(root.join(".espectacular")).unwrap();
        fs::write(
            root.join(".espectacular/config.toml"),
            r#"tool_version = "0.1.0"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
"#,
        )
        .unwrap();

        // managed blocks in AGENTS.md and CLAUDE.md
        fs::write(
            root.join("AGENTS.md"),
            format!("# Project\n\n{}\n", crate::init::AH_BLOCK_CONTENT),
        )
        .unwrap();
        fs::write(
            root.join("CLAUDE.md"),
            format!("# Project\n\n{}\n", crate::init::AH_BLOCK_CONTENT),
        )
        .unwrap();

        // lefthook for hook detection
        fs::write(
            root.join("lefthook.yml"),
            "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\n",
        )
        .unwrap();

        dir
    }

    // 4.7 RED tests

    #[test]
    fn healthy_repo_exits_zero_with_no_diagnostics() {
        let repo = make_healthy_repo();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report.healthy,
            "healthy repo must report healthy=true; diagnostics: {:?}",
            report.diagnostics
        );
        assert!(
            report.diagnostics.is_empty(),
            "healthy repo must have no diagnostics; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn bad_config_emits_config_schema_diagnostic() {
        let repo = make_healthy_repo();
        // Write a config missing required fields
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            "tool_version = \"\"\n[paths]\nspecs = \"\"\nchanges = \"\"\n[runners]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report.diagnostics.iter().any(|d| d.kind == "bad-config"),
            "bad config must emit bad-config diagnostic; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn missing_specs_path_emits_missing_paths_diagnostic() {
        let repo = make_healthy_repo();
        fs::remove_dir_all(repo.path().join("openspec/specs")).unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report.diagnostics.iter().any(|d| d.kind == "missing-path"),
            "missing specs dir must emit missing-path diagnostic; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn missing_changes_path_emits_missing_paths_diagnostic() {
        let repo = make_healthy_repo();
        fs::remove_dir_all(repo.path().join("openspec/changes")).unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report.diagnostics.iter().any(|d| d.kind == "missing-path"),
            "missing changes dir must emit missing-path diagnostic; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn version_drift_emits_version_drift_diagnostic() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            r#"tool_version = "0.0.1"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
"#,
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report.diagnostics.iter().any(|d| d.kind == "version-drift"),
            "version mismatch must emit version-drift diagnostic; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn missing_managed_block_in_agents_md_emits_diagnostic() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("AGENTS.md"),
            "# Project\n\nNo block here.\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.kind == "missing-managed-block"),
            "missing managed block must emit missing-managed-block; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn missing_managed_block_in_claude_md_emits_diagnostic() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("CLAUDE.md"),
            "# Project\n\nNo block here.\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.kind == "missing-managed-block"),
            "missing managed block in CLAUDE.md must emit missing-managed-block; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn hook_absent_emits_hook_absent_diagnostic() {
        let repo = make_healthy_repo();
        fs::remove_file(repo.path().join("lefthook.yml")).unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report.diagnostics.iter().any(|d| d.kind == "hook-absent"),
            "no hook framework must emit hook-absent diagnostic; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn scenario_collision_emits_collision_diagnostic() {
        let repo = make_healthy_repo();
        // Create two specs with the same heading slug
        let spec_dir = repo.path().join("openspec/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        let content = "# Capability: compiler\n\n## DEPLOYED Requirements\n\n### Requirement: R\n\n#### Scenario: Empty input rejected\n- **GIVEN** x\n- **WHEN** y\n- **THEN** z\n\n#### Scenario: Empty input rejected\n- **GIVEN** x\n- **WHEN** y\n- **THEN** z\n";
        fs::write(spec_dir.join("spec.md"), content).unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report.diagnostics.iter().any(|d| d.kind == "collision"),
            "duplicate scenario headings must emit collision diagnostic; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn orphan_contract_emits_orphan_diagnostic() {
        let repo = make_healthy_repo();
        // No scenario in specs, but a contract file exists
        let contract_dir = repo.path().join(".espectacular/compiler");
        fs::create_dir_all(&contract_dir).unwrap();
        fs::write(
            contract_dir.join("ghost-scenario.toml"),
            "id = \"ghost-scenario\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.kind == "orphan-contract"),
            "orphan contract must emit orphan-contract diagnostic; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn unknown_archetype_emits_diagnostic() {
        let repo = make_healthy_repo();
        // Create scenario and matching contract with bad archetype
        let spec_dir = repo.path().join("openspec/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        let spec_content = "# Capability: compiler\n\n## DEPLOYED Requirements\n\n### Requirement: R\n\n#### Scenario: Empty input rejected\n- **GIVEN** x\n- **WHEN** y\n- **THEN** z\n";
        fs::write(spec_dir.join("spec.md"), spec_content).unwrap();
        let contract_dir = repo.path().join(".espectacular/compiler");
        fs::create_dir_all(&contract_dir).unwrap();
        fs::write(
            contract_dir.join("empty-input-rejected.toml"),
            "id = \"empty-input-rejected\"\ndescription = \"\"\narchetype = \"UNKNOWN_TYPE\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.healthy);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.kind == "unknown-archetype"),
            "unknown archetype must emit unknown-archetype diagnostic; got: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn each_problem_emits_exactly_one_diagnostic_kind() {
        // bad config only → exactly one bad-config diagnostic
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            "tool_version = \"\"\n[paths]\nspecs = \"\"\nchanges = \"\"\n[runners]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let bad_config_count = report
            .diagnostics
            .iter()
            .filter(|d| d.kind == "bad-config")
            .count();
        assert_eq!(
            bad_config_count, 1,
            "should emit exactly one bad-config diagnostic"
        );
    }
}
