use crate::adapters::{self, DetectionSource};
use crate::archetypes;
use crate::init::{detect_hook_framework, HookFramework, AH_BLOCK_START};
use crate::openspec;
use crate::{config, contracts};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DoctorDiagnostic {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FrameworkDetection {
    pub name: String,
    pub detection_source: DetectionSource,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DoctorRecommendation {
    pub capability: String,
    pub detail: String,
    pub apply_command: String,
}

#[derive(Debug)]
pub struct DoctorReport {
    pub healthy: bool,
    pub diagnostics: Vec<DoctorDiagnostic>,
    pub detections: Vec<FrameworkDetection>,
    pub recommendations: Vec<DoctorRecommendation>,
}

#[derive(Debug)]
pub enum DoctorEnableResult {
    Written { path: String, table_name: String },
    AlreadyEnabled,
}

const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

fn source_label(source: DetectionSource) -> &'static str {
    match source {
        DetectionSource::Configured => "configured",
        DetectionSource::Manifest => "manifest",
        DetectionSource::Environment => "environment",
        DetectionSource::SourceImport => "source_import",
    }
}

fn detect_property(repo_root: &Path, cfg: &config::Config) -> Option<DetectionSource> {
    if cfg
        .capabilities
        .property
        .as_ref()
        .map(|c| c.enabled)
        .unwrap_or(false)
    {
        return Some(DetectionSource::Configured);
    }
    if let Ok(text) = fs::read_to_string(repo_root.join("pyproject.toml")) {
        if text.contains("hypothesis") {
            return Some(DetectionSource::Manifest);
        }
    }
    if let Ok(text) = fs::read_to_string(repo_root.join("Cargo.toml")) {
        if text.contains("proptest") {
            return Some(DetectionSource::Manifest);
        }
    }
    None
}

pub fn run_doctor(repo_root: &Path) -> anyhow::Result<DoctorReport> {
    let mut diagnostics: Vec<DoctorDiagnostic> = Vec::new();
    let mut detections: Vec<FrameworkDetection> = Vec::new();
    let mut recommendations: Vec<DoctorRecommendation> = Vec::new();

    let config_path = repo_root.join(".espectacular/config.toml");
    let config_path_str = config_path.to_string_lossy().to_string();

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

        if specs_dir.exists() {
            let specs_str = specs_dir.to_string_lossy().to_string();
            if let Ok(scenarios) = openspec::discover_scenarios(&specs_str) {
                let collisions = openspec::detect_slug_collisions(&scenarios);
                for (slug, _spec, _id) in &collisions {
                    diagnostics.push(DoctorDiagnostic {
                        kind: "collision".into(),
                        detail: format!("duplicate scenario slug: {slug}"),
                    });
                }

                let known_spec_slugs: HashSet<(String, String)> = scenarios
                    .iter()
                    .map(|s| (s.spec_path.clone(), s.id.clone()))
                    .collect();

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

                                if let Ok(contract) = contracts::load_contract(cp.to_str().unwrap())
                                {
                                    if !contract.archetype.is_empty()
                                        && !archetypes::is_known(&contract.archetype)
                                    {
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

        // Framework detection (7.1/7.2)
        for framework in &["pytest", "cargo", "vitest"] {
            match adapters::detect(repo_root, cfg, framework) {
                Some(DetectionSource::Configured) => {
                    detections.push(FrameworkDetection {
                        name: framework.to_string(),
                        detection_source: DetectionSource::Configured,
                    });
                }
                Some(source) => {
                    recommendations.push(DoctorRecommendation {
                        capability: framework.to_string(),
                        detail: format!("{framework} detected via {}", source_label(source)),
                        apply_command: format!("ah doctor --enable {framework}"),
                    });
                }
                None => {}
            }
        }

        // Property-based testing detection (7.1/7.2)
        match detect_property(repo_root, cfg) {
            Some(DetectionSource::Configured) => {
                detections.push(FrameworkDetection {
                    name: "property".to_string(),
                    detection_source: DetectionSource::Configured,
                });
            }
            Some(source) => {
                recommendations.push(DoctorRecommendation {
                    capability: "property".to_string(),
                    detail: format!(
                        "property-based testing framework detected via {}",
                        source_label(source)
                    ),
                    apply_command: "ah doctor --enable property".to_string(),
                });
            }
            None => {}
        }
    }

    // Managed block checks
    for filename in &["AGENTS.md", "CLAUDE.md"] {
        let path = repo_root.join(filename);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if !content.contains(AH_BLOCK_START) {
                    diagnostics.push(DoctorDiagnostic {
                        kind: "missing-managed-block".into(),
                        detail: format!("{filename} is missing the ah managed block"),
                    });
                }
            }
        }
    }

    // Hook detection
    if let HookFramework::None = detect_hook_framework(repo_root) {
        diagnostics.push(DoctorDiagnostic {
            kind: "hook-absent".into(),
            detail: "no supported pre-commit hook framework detected (lefthook or prek)".into(),
        });
    }

    let healthy = diagnostics.is_empty();
    Ok(DoctorReport {
        healthy,
        diagnostics,
        detections,
        recommendations,
    })
}

use crate::init::{append_capability_block, insert_runner_entry};

const KNOWN_CAPABILITIES: &[&str] = &[
    "pytest", "cargo", "vitest", "mutation", "property", "snapshot",
];

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DoctorJsonOutput {
    pub findings: Vec<DoctorFinding>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DoctorFinding {
    pub kind: String,
    pub suggested_action: String,
    pub playbook_command: String,
    pub apply_command: String,
    pub detail: String,
    pub capability: String,
}

/// Convert a DoctorReport into JSON output with recommendation findings.
pub fn doctor_to_json(report: &DoctorReport) -> DoctorJsonOutput {
    let findings: Vec<DoctorFinding> = report
        .recommendations
        .iter()
        .map(|rec| DoctorFinding {
            kind: "recommendation".to_string(),
            suggested_action: "enable_capability".to_string(),
            playbook_command: "ah explain enable_capability".to_string(),
            apply_command: rec.apply_command.clone(),
            detail: rec.detail.clone(),
            capability: rec.capability.clone(),
        })
        .collect();
    DoctorJsonOutput { findings }
}

pub fn run_doctor_enable(repo_root: &Path, capability: &str) -> anyhow::Result<DoctorEnableResult> {
    if !KNOWN_CAPABILITIES.contains(&capability) {
        anyhow::bail!(
            "unknown capability: {capability}; known: {}",
            KNOWN_CAPABILITIES.join(", ")
        );
    }

    let config_path = repo_root.join(".espectacular/config.toml");
    let cfg = config::load_config(config_path.to_str().unwrap())?;

    match capability {
        "pytest" | "cargo" | "vitest" => {
            if cfg.runners.contains_key(capability) {
                return Ok(DoctorEnableResult::AlreadyEnabled);
            }
            let value_toml = match capability {
                "pytest" => r#"["pytest"]"#,
                "cargo" => r#"["cargo", "test"]"#,
                "vitest" => r#"["vitest", "run"]"#,
                _ => unreachable!(),
            };
            let text = fs::read_to_string(&config_path)?;
            let updated = insert_runner_entry(&text, capability, value_toml);
            fs::write(&config_path, &updated)?;
            Ok(DoctorEnableResult::Written {
                path: config_path.to_string_lossy().into_owned(),
                table_name: format!("runners.{capability}"),
            })
        }
        "mutation" => {
            if cfg.capabilities.mutation.is_some() {
                return Ok(DoctorEnableResult::AlreadyEnabled);
            }
            let text = fs::read_to_string(&config_path)?;
            let updated = append_capability_block(&text, "mutation");
            fs::write(&config_path, &updated)?;
            Ok(DoctorEnableResult::Written {
                path: config_path.to_string_lossy().into_owned(),
                table_name: "capabilities.mutation".to_string(),
            })
        }
        "property" => {
            if cfg.capabilities.property.is_some() {
                return Ok(DoctorEnableResult::AlreadyEnabled);
            }
            let text = fs::read_to_string(&config_path)?;
            let updated = append_capability_block(&text, "property");
            fs::write(&config_path, &updated)?;
            Ok(DoctorEnableResult::Written {
                path: config_path.to_string_lossy().into_owned(),
                table_name: "capabilities.property".to_string(),
            })
        }
        "snapshot" => {
            if cfg.capabilities.snapshot.is_some() {
                return Ok(DoctorEnableResult::AlreadyEnabled);
            }
            let text = fs::read_to_string(&config_path)?;
            let updated = append_capability_block(&text, "snapshot");
            fs::write(&config_path, &updated)?;
            Ok(DoctorEnableResult::Written {
                path: config_path.to_string_lossy().into_owned(),
                table_name: "capabilities.snapshot".to_string(),
            })
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::{append_capability_block, insert_runner_entry};
    use std::fs;
    use tempfile::TempDir;

    fn make_healthy_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("openspec/specs")).unwrap();
        fs::create_dir_all(root.join("openspec/changes")).unwrap();
        fs::create_dir_all(root.join(".espectacular")).unwrap();
        fs::write(
            root.join(".espectacular/config.toml"),
            r#"tool_version = "0.3.0"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
"#,
        )
        .unwrap();

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
        fs::write(
            root.join("lefthook.yml"),
            "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\n",
        )
        .unwrap();

        dir
    }

    fn base_config_toml() -> String {
        format!(
            "tool_version = \"{}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n",
            TOOL_VERSION
        )
    }

    // ── 4.7 existing tests (unchanged) ───────────────────────────────────────

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

    // ── 7.1 Red: framework detection reporting ────────────────────────────────

    #[test]
    fn configured_pytest_runner_appears_in_detections() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            &format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\npytest = [\"pytest\"]\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "pytest" && d.detection_source == DetectionSource::Configured),
            "configured pytest must appear in detections; got: {:?}",
            report.detections
        );
    }

    #[test]
    fn configured_cargo_runner_appears_in_detections() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            &format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\ncargo = [\"cargo\", \"test\"]\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "cargo" && d.detection_source == DetectionSource::Configured),
            "configured cargo must appear in detections; got: {:?}",
            report.detections
        );
    }

    #[test]
    fn configured_vitest_runner_appears_in_detections() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            &format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\nvitest = [\"vitest\", \"run\"]\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "vitest" && d.detection_source == DetectionSource::Configured),
            "configured vitest must appear in detections; got: {:?}",
            report.detections
        );
    }

    #[test]
    fn configured_property_capability_appears_in_detections() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            &format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n[capabilities.property]\nenabled = true\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "property" && d.detection_source == DetectionSource::Configured),
            "configured property capability must appear in detections; got: {:?}",
            report.detections
        );
    }

    #[test]
    fn framework_detection_does_not_affect_healthy_flag() {
        let repo = make_healthy_repo();
        // Add pytest via manifest (not configured) → recommendation, not a problem
        fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report.healthy,
            "recommendations must not make healthy=false; diagnostics: {:?}",
            report.diagnostics
        );
    }

    // ── 7.3 Red: recommendation findings ─────────────────────────────────────

    #[test]
    fn pytest_manifest_detected_but_not_configured_emits_recommendation() {
        let repo = make_healthy_repo();
        fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let rec = report
            .recommendations
            .iter()
            .find(|r| r.capability == "pytest");
        assert!(
            rec.is_some(),
            "pytest detected via manifest must emit recommendation; got: {:?}",
            report.recommendations
        );
        let rec = rec.unwrap();
        assert_eq!(
            rec.apply_command, "ah doctor --enable pytest",
            "recommendation apply_command must be --enable invocation"
        );
    }

    #[test]
    fn cargo_manifest_detected_but_not_configured_emits_recommendation() {
        let repo = make_healthy_repo();
        fs::write(repo.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let rec = report
            .recommendations
            .iter()
            .find(|r| r.capability == "cargo");
        assert!(
            rec.is_some(),
            "cargo detected via manifest must emit recommendation; got: {:?}",
            report.recommendations
        );
        assert_eq!(rec.unwrap().apply_command, "ah doctor --enable cargo");
    }

    #[test]
    fn vitest_manifest_detected_but_not_configured_emits_recommendation() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("package.json"),
            r#"{"devDependencies":{"vitest":"^1.0"}}"#,
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let rec = report
            .recommendations
            .iter()
            .find(|r| r.capability == "vitest");
        assert!(
            rec.is_some(),
            "vitest detected via manifest must emit recommendation; got: {:?}",
            report.recommendations
        );
        assert_eq!(rec.unwrap().apply_command, "ah doctor --enable vitest");
    }

    #[test]
    fn property_hypothesis_detected_emits_recommendation() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("pyproject.toml"),
            "[tool.pytest.ini_options]\n\n[project]\ndependencies = [\"hypothesis\"]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let rec = report
            .recommendations
            .iter()
            .find(|r| r.capability == "property");
        assert!(
            rec.is_some(),
            "hypothesis in pyproject must emit property recommendation; got: {:?}",
            report.recommendations
        );
        assert_eq!(rec.unwrap().apply_command, "ah doctor --enable property");
    }

    #[test]
    fn configured_framework_emits_detection_not_recommendation() {
        let repo = make_healthy_repo();
        // pytest both configured AND has manifest signal
        fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            &format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\npytest = [\"pytest\"]\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "pytest" && d.detection_source == DetectionSource::Configured),
            "configured pytest should be in detections"
        );
        assert!(
            !report
                .recommendations
                .iter()
                .any(|r| r.capability == "pytest"),
            "configured pytest must NOT be in recommendations; got: {:?}",
            report.recommendations
        );
    }

    // ── 7.5 Red: --enable writes exact config tables ──────────────────────────

    #[test]
    fn enable_pytest_writes_runner_entry_to_config() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, &base_config_toml()).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        let updated = insert_runner_entry(&text, "pytest", r#"["pytest"]"#);
        fs::write(&config_path, &updated).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("pytest = [\"pytest\"]"),
            "pytest runner entry must be written; got:\n{content}"
        );
    }

    #[test]
    fn enable_cargo_writes_runner_entry_to_config() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, &base_config_toml()).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        let updated = insert_runner_entry(&text, "cargo", r#"["cargo", "test"]"#);
        fs::write(&config_path, &updated).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains(r#"cargo = ["cargo", "test"]"#),
            "cargo runner entry must be written; got:\n{content}"
        );
    }

    #[test]
    fn enable_vitest_writes_runner_entry_to_config() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, &base_config_toml()).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        let updated = insert_runner_entry(&text, "vitest", r#"["vitest", "run"]"#);
        fs::write(&config_path, &updated).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains(r#"vitest = ["vitest", "run"]"#),
            "vitest runner entry must be written; got:\n{content}"
        );
    }

    #[test]
    fn enable_mutation_appends_capability_block() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, &base_config_toml()).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        let updated = append_capability_block(&text, "mutation");
        fs::write(&config_path, &updated).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("[capabilities.mutation]") && content.contains("enabled = true"),
            "mutation capability block must be written; got:\n{content}"
        );
    }

    #[test]
    fn enable_property_appends_capability_block() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, &base_config_toml()).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        let updated = append_capability_block(&text, "property");
        fs::write(&config_path, &updated).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("[capabilities.property]") && content.contains("enabled = true"),
            "property capability block must be written; got:\n{content}"
        );
    }

    #[test]
    fn enable_snapshot_appends_capability_block() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, &base_config_toml()).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        let updated = append_capability_block(&text, "snapshot");
        fs::write(&config_path, &updated).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("[capabilities.snapshot]") && content.contains("enabled = true"),
            "snapshot capability block must be written; got:\n{content}"
        );
    }

    #[test]
    fn run_doctor_enable_pytest_writes_config_and_returns_path() {
        let repo = make_healthy_repo();
        let result = run_doctor_enable(repo.path(), "pytest").unwrap();
        match result {
            DoctorEnableResult::Written { path, table_name } => {
                assert!(
                    path.ends_with("config.toml"),
                    "path must point to config.toml"
                );
                assert_eq!(table_name, "runners.pytest");
                let content = fs::read_to_string(path).unwrap();
                assert!(content.contains("pytest = [\"pytest\"]"));
            }
            DoctorEnableResult::AlreadyEnabled => {
                panic!("should have written, not already-enabled")
            }
        }
    }

    #[test]
    fn run_doctor_enable_mutation_writes_capability_block() {
        let repo = make_healthy_repo();
        let result = run_doctor_enable(repo.path(), "mutation").unwrap();
        match result {
            DoctorEnableResult::Written { path, table_name } => {
                assert_eq!(table_name, "capabilities.mutation");
                let content = fs::read_to_string(path).unwrap();
                assert!(
                    content.contains("[capabilities.mutation]")
                        && content.contains("enabled = true")
                );
            }
            DoctorEnableResult::AlreadyEnabled => {
                panic!("should have written, not already-enabled")
            }
        }
    }

    // ── 7.7 Red: unknown capability and already-enabled no-op ─────────────────

    #[test]
    fn enable_unknown_capability_returns_error() {
        let repo = make_healthy_repo();
        let err = run_doctor_enable(repo.path(), "jest").unwrap_err();
        assert!(
            err.to_string().contains("unknown capability"),
            "unknown capability must return error; got: {err}"
        );
    }

    #[test]
    fn enable_pytest_when_already_configured_is_noop() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            &format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\npytest = [\"pytest\"]\n"
            ),
        )
        .unwrap();
        let original = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
        let result = run_doctor_enable(repo.path(), "pytest").unwrap();
        assert!(
            matches!(result, DoctorEnableResult::AlreadyEnabled),
            "enabling already-configured runner must return AlreadyEnabled"
        );
        let after = fs::read_to_string(repo.path().join(".espectacular/config.toml")).unwrap();
        assert_eq!(
            original, after,
            "config must not be modified when already-enabled"
        );
    }

    #[test]
    fn enable_mutation_when_already_enabled_is_noop() {
        let repo = make_healthy_repo();
        let config_text = format!(
            "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n\n[capabilities.mutation]\nenabled = true\n"
        );
        fs::write(repo.path().join(".espectacular/config.toml"), &config_text).unwrap();
        let result = run_doctor_enable(repo.path(), "mutation").unwrap();
        assert!(
            matches!(result, DoctorEnableResult::AlreadyEnabled),
            "enabling already-enabled mutation must return AlreadyEnabled"
        );
    }

    #[test]
    fn insert_runner_entry_adds_to_existing_runners_section() {
        let input =
            "tool_version = \"0.1.0\"\n[paths]\nspecs = \"s\"\nchanges = \"c\"\n[runners]\n";
        let result = insert_runner_entry(input, "pytest", r#"["pytest"]"#);
        assert!(
            result.contains("pytest = [\"pytest\"]"),
            "must contain new runner entry; got:\n{result}"
        );
        // Must still be valid enough that [runners] header appears once
        assert_eq!(
            result.matches("[runners]").count(),
            1,
            "must not duplicate [runners] header"
        );
    }

    #[test]
    fn insert_runner_entry_does_not_duplicate_section_header() {
        let input = "tool_version = \"0.1.0\"\n[paths]\nspecs = \"s\"\nchanges = \"c\"\n[runners]\npytest = [\"pytest\"]\n";
        let result = insert_runner_entry(input, "cargo", r#"["cargo", "test"]"#);
        assert_eq!(
            result.matches("[runners]").count(),
            1,
            "must not add duplicate [runners] header"
        );
        assert!(result.contains("cargo = [\"cargo\", \"test\"]"));
    }
}
