use crate::fsutil::{refresh_managed_file, write_text};
use crate::openspec;
use anyhow::Context;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct InitResult {
    pub created: Vec<String>,
    pub refreshed: Vec<String>,
    pub concerns: Vec<String>,
    pub stubbed_contracts: Vec<String>,
}

pub const AH_BLOCK_START: &str = "<!-- ah:managed:start -->";
const AH_BLOCK_END: &str = "<!-- ah:managed:end -->";

pub const AH_BLOCK_CONTENT: &str = r#"<!-- ah:managed:start -->
## espectacular

Run `ah check` to verify spec-test correspondence before committing.

- `ah check` — validate all deployed specs
- `ah check --changes <name>` — validate with a change overlay
- `ah init` — set up or refresh espectacular project files
- `ah doctor` — diagnose setup issues
<!-- ah:managed:end -->"#;

const ESPECTACULAR_AGENTS_CONTENT: &str = r#"# espectacular

This directory contains espectacular configuration and scenario contracts.

## Layout

- `config.toml` — project configuration (tool version, paths, runners)
- `<spec>/<scenario>.toml` — per-scenario test contracts
- `changes/<change>/` — staged contracts for in-flight OpenSpec changes

## Workflow

Run `ah check` from the repo root to validate all spec-test correspondence.
Run `ah check --changes <name>` to validate a change overlay before merging.
"#;

const DEFAULT_CONFIG_TOML: &str = r#"tool_version = "0.1.0"

[paths]
specs = "openspec/specs"
changes = "openspec/changes"

[runners]
"#;

#[derive(Debug, PartialEq)]
pub enum HookFramework {
    Lefthook,
    Prek,
    None,
}

pub fn detect_hook_framework(repo_root: &Path) -> HookFramework {
    if repo_root.join("lefthook.yml").exists() || repo_root.join("lefthook.yaml").exists() {
        return HookFramework::Lefthook;
    }
    if repo_root.join(".prek").exists() || repo_root.join("prek.yml").exists() {
        return HookFramework::Prek;
    }
    HookFramework::None
}

pub fn run_init(repo_root: &Path) -> anyhow::Result<InitResult> {
    anyhow::ensure!(
        repo_root.join("openspec").exists(),
        "no openspec/ directory found in {}; ah init requires an OpenSpec project",
        repo_root.display()
    );

    let mut result = InitResult {
        created: Vec::new(),
        refreshed: Vec::new(),
        concerns: Vec::new(),
        stubbed_contracts: Vec::new(),
    };

    // Create .espectacular/
    let espectacular_dir = repo_root.join(".espectacular");
    fs::create_dir_all(&espectacular_dir).context("cannot create .espectacular/")?;

    // config.toml — only if missing
    let config_path = espectacular_dir.join("config.toml");
    if !config_path.exists() {
        write_text(&config_path, DEFAULT_CONFIG_TOML)?;
        result.created.push(".espectacular/config.toml".into());
    }

    // .espectacular/AGENTS.md — always refresh
    let espectacular_agents = espectacular_dir.join("AGENTS.md");
    let agents_existed = espectacular_agents.exists();
    write_text(&espectacular_agents, ESPECTACULAR_AGENTS_CONTENT)?;
    if agents_existed {
        result.refreshed.push(".espectacular/AGENTS.md".into());
    } else {
        result.created.push(".espectacular/AGENTS.md".into());
    }

    // Top-level AGENTS.md — create if absent, inject managed block if present
    let agents_md = repo_root.join("AGENTS.md");
    update_managed_file(&agents_md, &mut result)?;

    // Top-level CLAUDE.md — create if absent, inject managed block if present
    let claude_md = repo_root.join("CLAUDE.md");
    update_managed_file(&claude_md, &mut result)?;

    // Stub contracts for deployed scenarios without existing contracts
    let specs_dir = repo_root.join("openspec/specs");
    if specs_dir.exists() {
        let specs_str = specs_dir.to_string_lossy().to_string();
        if let Ok(scenarios) = openspec::discover_scenarios(&specs_str) {
            for scenario in &scenarios {
                stub_contract_if_missing(repo_root, scenario, &mut result)?;
            }
        }
    }

    // Hook integration
    match detect_hook_framework(repo_root) {
        HookFramework::Lefthook => {
            install_lefthook(repo_root, &mut result)?;
        }
        HookFramework::Prek => {
            install_prek(repo_root, &mut result)?;
        }
        HookFramework::None => {
            result.concerns.push(
                "No supported pre-commit hook framework detected (lefthook or prek). \
                Please set up pre-commit integration manually to run `ah check` before commits."
                    .into(),
            );
        }
    }

    Ok(result)
}

fn update_managed_file(path: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    let existed = path.exists();
    refresh_managed_file(path, AH_BLOCK_CONTENT, AH_BLOCK_START, AH_BLOCK_END)?;
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    if existed {
        result.refreshed.push(name);
    } else {
        result.created.push(name);
    }
    Ok(())
}

const LEFTHOOK_AH_BLOCK: &str = r#"
# ah:managed:start
  ah-check:
    run: ah check
# ah:managed:end
"#;

fn install_lefthook(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    let path = if repo_root.join("lefthook.yml").exists() {
        repo_root.join("lefthook.yml")
    } else {
        repo_root.join("lefthook.yaml")
    };

    let existing =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;

    if existing.contains("ah check") {
        return Ok(());
    }

    // Inject into pre-commit block or append
    let new_content = if existing.contains("pre-commit:") {
        let insert_at = existing.find("pre-commit:").unwrap() + "pre-commit:".len();
        let (before, after) = existing.split_at(insert_at);
        format!("{}{}{}", before, LEFTHOOK_AH_BLOCK, after)
    } else {
        format!("{}pre-commit:\n  commands:{}", existing, LEFTHOOK_AH_BLOCK)
    };

    write_text(&path, new_content)?;
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    result.refreshed.push(name);
    Ok(())
}

fn stub_contract_if_missing(
    repo_root: &Path,
    scenario: &openspec::Scenario,
    result: &mut InitResult,
) -> anyhow::Result<()> {
    // spec_path is the spec name, e.g. "compiler"
    let spec_name = &scenario.spec_path;
    if spec_name.is_empty() {
        return Ok(());
    }

    let contract_dir = repo_root.join(".espectacular").join(&spec_name);
    let contract_path = contract_dir.join(format!("{}.toml", scenario.id));

    if contract_path.exists() {
        return Ok(());
    }

    fs::create_dir_all(&contract_dir)
        .with_context(|| format!("cannot create {}", contract_dir.display()))?;

    let stub = format!(
        "id = \"{id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n",
        id = scenario.id
    );
    write_text(&contract_path, stub)?;

    result
        .stubbed_contracts
        .push(format!("{}/{}.toml", spec_name, scenario.id));
    Ok(())
}

fn install_prek(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    let path = if repo_root.join(".prek").exists() {
        repo_root.join(".prek")
    } else {
        repo_root.join("prek.yml")
    };

    let existing =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;

    if existing.contains("ah check") {
        return Ok(());
    }

    let new_content = format!("{}ah check\n", existing);
    write_text(&path, new_content)?;
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    result.refreshed.push(name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_repo(has_openspec: bool) -> TempDir {
        let dir = TempDir::new().unwrap();
        if has_openspec {
            fs::create_dir_all(dir.path().join("openspec")).unwrap();
        }
        dir
    }

    // 4.1 RED: ah init creates expected files in fresh repo with openspec/

    #[test]
    fn init_refuses_without_openspec_dir() {
        let repo = make_repo(false);
        let result = run_init(repo.path());
        assert!(result.is_err(), "should fail without openspec/");
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("openspec"),
            "error should mention openspec, got: {msg}"
        );
    }

    #[test]
    fn init_creates_espectacular_config_when_missing() {
        let repo = make_repo(true);
        let result = run_init(repo.path()).unwrap();
        let config_path = repo.path().join(".espectacular/config.toml");
        assert!(
            config_path.exists(),
            ".espectacular/config.toml must be created"
        );
        assert!(
            result.created.iter().any(|s| s.contains("config.toml")),
            "created list should contain config.toml"
        );
    }

    #[test]
    fn init_creates_espectacular_agents_md() {
        let repo = make_repo(true);
        run_init(repo.path()).unwrap();
        let path = repo.path().join(".espectacular/AGENTS.md");
        assert!(path.exists(), ".espectacular/AGENTS.md must be created");
    }

    #[test]
    fn init_creates_top_level_agents_md_when_absent() {
        let repo = make_repo(true);
        run_init(repo.path()).unwrap();
        let path = repo.path().join("AGENTS.md");
        assert!(
            path.exists(),
            "top-level AGENTS.md must be created when absent"
        );
    }

    #[test]
    fn init_creates_top_level_claude_md_when_absent() {
        let repo = make_repo(true);
        run_init(repo.path()).unwrap();
        let path = repo.path().join("CLAUDE.md");
        assert!(
            path.exists(),
            "top-level CLAUDE.md must be created when absent"
        );
    }

    #[test]
    fn init_is_idempotent() {
        let repo = make_repo(true);
        run_init(repo.path()).unwrap();
        // Second run must not error
        let result = run_init(repo.path());
        assert!(result.is_ok(), "second run must succeed (idempotent)");
    }

    #[test]
    fn init_does_not_overwrite_existing_agents_md() {
        let repo = make_repo(true);
        let agents_path = repo.path().join("AGENTS.md");
        fs::write(&agents_path, "# My custom AGENTS\n").unwrap();
        run_init(repo.path()).unwrap();
        let content = fs::read_to_string(&agents_path).unwrap();
        assert!(
            content.contains("My custom AGENTS"),
            "must not overwrite existing AGENTS.md body content"
        );
    }

    #[test]
    fn init_refreshes_managed_block_in_existing_claude_md() {
        let repo = make_repo(true);
        let claude_path = repo.path().join("CLAUDE.md");
        fs::write(&claude_path, "# Project\n\nSome content.\n").unwrap();
        let result = run_init(repo.path()).unwrap();
        let content = fs::read_to_string(&claude_path).unwrap();
        assert!(
            content.contains("espectacular") || content.contains("ah check"),
            "CLAUDE.md should have managed ah block"
        );
        assert!(
            result.refreshed.iter().any(|s| s.contains("CLAUDE.md")),
            "refreshed list should contain CLAUDE.md"
        );
    }

    // 4.5 RED: hook detection precedence

    #[test]
    fn hook_detection_returns_none_when_no_framework() {
        let repo = make_repo(true);
        assert_eq!(detect_hook_framework(repo.path()), HookFramework::None);
    }

    #[test]
    fn hook_detection_returns_lefthook_when_lefthook_yml_present() {
        let repo = make_repo(true);
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n",
        )
        .unwrap();
        assert_eq!(detect_hook_framework(repo.path()), HookFramework::Lefthook);
    }

    #[test]
    fn hook_detection_returns_prek_when_prek_config_present() {
        let repo = make_repo(true);
        fs::write(repo.path().join(".prek"), "").unwrap();
        assert_eq!(detect_hook_framework(repo.path()), HookFramework::Prek);
    }

    #[test]
    fn hook_detection_prefers_lefthook_over_prek() {
        let repo = make_repo(true);
        fs::write(repo.path().join("lefthook.yml"), "").unwrap();
        fs::write(repo.path().join(".prek"), "").unwrap();
        assert_eq!(
            detect_hook_framework(repo.path()),
            HookFramework::Lefthook,
            "lefthook must win over prek"
        );
    }

    #[test]
    fn init_reports_concern_when_no_hook_framework() {
        let repo = make_repo(true);
        let result = run_init(repo.path()).unwrap();
        assert!(
            !result.concerns.is_empty(),
            "must report concern when no hook framework is present"
        );
        let concerns_text = result.concerns.join(" ");
        assert!(
            concerns_text.contains("pre-commit") || concerns_text.contains("hook"),
            "concern must mention pre-commit or hook"
        );
    }

    #[test]
    fn init_does_not_write_raw_git_hook_when_no_framework() {
        let repo = make_repo(true);
        fs::create_dir_all(repo.path().join(".git/hooks")).unwrap();
        run_init(repo.path()).unwrap();
        assert!(
            !repo.path().join(".git/hooks/pre-commit").exists(),
            "must NOT write raw .git/hooks/pre-commit fallback"
        );
    }

    #[test]
    fn init_installs_lefthook_integration_when_lefthook_present() {
        let repo = make_repo(true);
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n",
        )
        .unwrap();
        let result = run_init(repo.path()).unwrap();
        let lefthook_content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
        assert!(
            lefthook_content.contains("ah check")
                || result.refreshed.iter().any(|s| s.contains("lefthook")),
            "lefthook.yml should include ah check integration"
        );
    }

    #[test]
    fn init_installs_prek_integration_when_only_prek_present() {
        let repo = make_repo(true);
        fs::write(repo.path().join(".prek"), "").unwrap();
        let result = run_init(repo.path()).unwrap();
        let prek_content = fs::read_to_string(repo.path().join(".prek")).unwrap();
        assert!(
            prek_content.contains("ah check")
                || result.refreshed.iter().any(|s| s.contains("prek")),
            ".prek should include ah check integration"
        );
    }

    // 4.3 RED: stub empty TOML contracts for existing deployed scenarios

    fn make_repo_with_scenarios(scenarios: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("openspec")).unwrap();
        fs::create_dir_all(dir.path().join("openspec/specs")).unwrap();
        for (spec, scenario_heading) in scenarios {
            let spec_dir = dir.path().join("openspec/specs").join(spec);
            fs::create_dir_all(&spec_dir).unwrap();
            let content = format!(
                "# Capability: {spec}\n\n## DEPLOYED Requirements\n\n### Requirement: Test\n\n#### Scenario: {scenario_heading}\n- **GIVEN** something\n- **WHEN** action\n- **THEN** result\n"
            );
            fs::write(spec_dir.join("spec.md"), content).unwrap();
        }
        dir
    }

    #[test]
    fn init_stubs_contracts_for_scenarios_without_existing_contracts() {
        let repo = make_repo_with_scenarios(&[("compiler", "Empty input rejected")]);
        let result = run_init(repo.path()).unwrap();
        let stub_path = repo
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml");
        assert!(
            stub_path.exists(),
            "stub contract must be created at .espectacular/compiler/empty-input-rejected.toml"
        );
        assert!(
            result
                .stubbed_contracts
                .iter()
                .any(|s| s.contains("empty-input-rejected")),
            "stubbed_contracts must include the scenario slug"
        );
    }

    #[test]
    fn init_stub_declares_no_tests() {
        let repo = make_repo_with_scenarios(&[("compiler", "Empty input rejected")]);
        run_init(repo.path()).unwrap();
        let stub_path = repo
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml");
        let content = fs::read_to_string(&stub_path).unwrap();
        // Stub must have required fields but no [[tests.*]] table
        assert!(content.contains("id ="), "stub must have id field");
        assert!(content.contains("status ="), "stub must have status field");
        assert!(
            !content.contains("[[tests"),
            "stub must NOT declare any tests"
        );
    }

    #[test]
    fn init_does_not_overwrite_existing_contracts() {
        let repo = make_repo_with_scenarios(&[("compiler", "Empty input rejected")]);
        // Pre-create the contract
        fs::create_dir_all(repo.path().join(".espectacular/compiler")).unwrap();
        let stub_path = repo
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml");
        fs::write(&stub_path, "id = \"empty-input-rejected\"\ncustom = true\n").unwrap();
        let result = run_init(repo.path()).unwrap();
        let content = fs::read_to_string(&stub_path).unwrap();
        assert!(
            content.contains("custom = true"),
            "must not overwrite existing contract"
        );
        assert!(
            !result
                .stubbed_contracts
                .iter()
                .any(|s| s.contains("empty-input-rejected")),
            "must not report existing contract as stubbed"
        );
    }

    #[test]
    fn init_stubs_contracts_for_multiple_specs() {
        let repo = make_repo_with_scenarios(&[
            ("compiler", "Empty input rejected"),
            ("runtime", "Handle timeout"),
        ]);
        let result = run_init(repo.path()).unwrap();
        assert_eq!(
            result.stubbed_contracts.len(),
            2,
            "must stub one contract per scenario"
        );
        assert!(repo
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml")
            .exists());
        assert!(repo
            .path()
            .join(".espectacular/runtime/handle-timeout.toml")
            .exists());
    }
}
