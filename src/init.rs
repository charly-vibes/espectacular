use crate::fsutil::write_text;
use crate::openspec;
use anyhow::Context;
use genesis::discovery;
use genesis::git_hooks;
use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};
use genesis::scaffold::Scaffold;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct InitResult {
    pub created: Vec<String>,
    pub refreshed: Vec<String>,
    pub concerns: Vec<String>,
    pub stubbed_contracts: Vec<String>,
}

/// The content between managed block markers (command reference + adapter listing).
/// Kept separate from the markers so genesis::managed_block wraps it.
pub const AH_BLOCK_CONTENT: &str = r#"
## espectacular

Run `ah check` to verify spec-test correspondence before committing.

- `ah check` — validate all deployed specs
- `ah check --changes <name>` — validate with a change overlay
- `ah init` — set up or refresh espectacular project files
- `ah doctor` — diagnose setup issues
- `ah explain <topic>` — playbook guidance for finding kinds and suggested actions
- `ah doctor --enable <adapter>` — write adapter config into .espectacular/config.toml
- `ah signals` — emit dont drift signals
"#;

/// Full managed block content including markers (for test fixtures only).
#[cfg(test)]
pub const AH_BLOCK_CONTENT_WITH_MARKERS: &str = r#"<!-- ah:managed:start -->
## espectacular

Run `ah check` to verify spec-test correspondence before committing.

- `ah check` — validate all deployed specs
- `ah check --changes <name>` — validate with a change overlay
- `ah init` — set up or refresh espectacular project files
- `ah doctor` — diagnose setup issues
- `ah explain <topic>` — playbook guidance for finding kinds and suggested actions
- `ah doctor --enable <adapter>` — write adapter config into .espectacular/config.toml
- `ah signals` — emit dont drift signals
<!-- ah:managed:end -->"#;

/// Build a BlockInjector for the `ah:managed` block.
pub fn ah_block_injector() -> BlockInjector {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::with_markers(
        "ah:managed",
        "<!-- ah:managed:start -->",
        "<!-- ah:managed:end -->",
    ));
    BlockInjector::new(reg)
}

const ESPECTACULAR_AGENTS_CONTENT: &str =
    "Before acting on any `ah check` finding, run its `playbook_command` to get the \
canonical remediation steps. Use `ah explain <topic>` to look up the playbook for any \
finding kind.\n";

fn default_config_toml() -> String {
    format!(
        r#"tool_version = "{}"

[paths]
specs = "openspec/specs"
changes = "openspec/changes"

[runners]
"#,
        env!("CARGO_PKG_VERSION")
    )
}

// ── config text helpers (shared with doctor --enable) ────────────────────────

pub fn insert_runner_entry(config_text: &str, key: &str, value_toml: &str) -> String {
    let new_line = format!("{key} = {value_toml}\n");
    if let Some(section_pos) = find_section_start(config_text, "[runners]") {
        let after_header = section_pos + "[runners]".len();
        let rest = &config_text[after_header..];
        let section_content_len = rest.find("\n[").map(|p| p + 1).unwrap_or(rest.len());
        let insert_at = after_header + section_content_len;
        let base = &config_text[..insert_at];
        let tail = &config_text[insert_at..];
        let separator = if base.ends_with('\n') { "" } else { "\n" };
        format!("{base}{separator}{new_line}{tail}")
    } else {
        let trimmed = config_text.trim_end();
        format!("{trimmed}\n\n[runners]\n{new_line}")
    }
}

pub fn append_capability_block(config_text: &str, capability: &str) -> String {
    let trimmed = config_text.trim_end();
    format!("{trimmed}\n\n[capabilities.{capability}]\nenabled = true\n")
}

fn find_section_start(text: &str, header: &str) -> Option<usize> {
    if text.starts_with(header) {
        return Some(0);
    }
    let needle = format!("\n{header}");
    text.find(&needle).map(|pos| pos + 1)
}

pub fn run_init(repo_root: &Path) -> anyhow::Result<InitResult> {
    anyhow::ensure!(
        repo_root.join("openspec").exists(),
        "Error: openspec/ directory not found at {}.\n\nah requires an OpenSpec project. Create the minimal layout:\n\n  openspec/\n  └── specs/\n      └── <spec>/\n          └── spec.md      ← \"#### Scenario: ...\" headings go here\n\nOr install the openspec CLI and run: openspec init",
        repo_root.display()
    );

    let mut result = InitResult {
        created: Vec::new(),
        refreshed: Vec::new(),
        concerns: Vec::new(),
        stubbed_contracts: Vec::new(),
    };

    // Create .espectacular/ directory and default config.toml via genesis::scaffold
    let scaffold_result = Scaffold::new(repo_root)
        .dir(".espectacular")
        .default_config(".espectacular/config.toml", default_config_toml())
        .build()
        .context("cannot scaffold .espectacular/")?;
    for path in &scaffold_result.created {
        let display = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        result.created.push(display);
    }

    // .espectacular/AGENTS.md — always refresh
    let espectacular_dir = repo_root.join(".espectacular");
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

    // Also inject managed block into .espectacular/AGENTS.md
    let ah_agents = espectacular_dir.join("AGENTS.md");
    if ah_agents.exists() {
        let injector = ah_block_injector();
        if !injector.has_block(&ah_agents, "ah:managed") {
            injector.inject(&ah_agents, "ah:managed", AH_BLOCK_CONTENT)?;
            result.refreshed.push(".espectacular/AGENTS.md".into());
        }
    }

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

    // Hook integration (espectacular-6ye R2: genesis::git_hooks detection)
    match git_hooks::framework(repo_root) {
        git_hooks::Framework::Lefthook => {
            install_lefthook(repo_root, &mut result)?;
        }
        git_hooks::Framework::Prek => {
            install_prek(repo_root, &mut result)?;
        }
        git_hooks::Framework::Husky => {
            result.concerns.push(
                "Husky detected but not wirable by ah (lefthook only). \
                 Add `ah check` to your husky pre-commit hook manually to run \
                 spec-test correspondence before commits."
                    .into(),
            );
        }
        git_hooks::Framework::None => {
            result.concerns.push(
                "No supported pre-commit hook framework detected (lefthook or prek). \
                Please set up pre-commit integration manually to run `ah check` before commits."
                    .into(),
            );
        }
    }

    // Register in .genesis/tools.toml for cross-tool discovery
    match discovery::register(
        repo_root,
        "espectacular",
        "Spec-test correspondence enforcer",
        "directory",
        ".espectacular",
    ) {
        Ok(()) => {}
        Err(e) => result.concerns.push(format!(".genesis/tools.toml: {e}")),
    }

    Ok(result)
}

fn update_managed_file(path: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    let existed = path.exists();
    let injector = ah_block_injector();
    injector
        .inject(path, "ah:managed", AH_BLOCK_CONTENT)
        .context("cannot inject ah:managed block")?;
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    if existed {
        result.refreshed.push(name);
    } else {
        result.created.push(name);
    }
    Ok(())
}

const LEFTHOOK_AH_COMMAND: &str = "  ah-check:\n    run: ah check\n";

fn install_lefthook(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    // Managed block wired through genesis::git_hooks::lefthook::ensure_wired
    // (espectacular-6ye R2): column-0 anchor injection + file-level idempotence,
    // replacing the hand-rolled find/split injection.
    let block = BlockDef::with_markers("ah:managed", "# ah:managed:start", "# ah:managed:end");
    match genesis::git_hooks::lefthook::ensure_wired(
        repo_root,
        genesis::git_hooks::lefthook::Stage::PreCommit,
        &block,
        LEFTHOOK_AH_COMMAND,
    ) {
        Ok(git_hooks::lefthook::WiredOutcome::Injected) => {
            result.refreshed.push("lefthook.yml".to_string());
        }
        Ok(git_hooks::lefthook::WiredOutcome::AlreadyWired) => {}
        Err(e) => return Err(e).with_context(|| "failed to wire ah check into lefthook.yml"),
    }
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

    let contract_dir = repo_root.join(".espectacular").join(spec_name);
    let contract_path = contract_dir.join(format!("{}.toml", scenario.id));

    if contract_path.exists() {
        return Ok(());
    }

    fs::create_dir_all(&contract_dir)
        .with_context(|| format!("cannot create {}", contract_dir.display()))?;

    let stub = format!(
        "id = \"{id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"{version}\"\n",
        id = scenario.id,
        version = env!("CARGO_PKG_VERSION")
    );
    write_text(&contract_path, stub)?;

    result
        .stubbed_contracts
        .push(format!("{}/{}.toml", spec_name, scenario.id));
    Ok(())
}

fn install_prek(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    let path = repo_root.join("prek.toml");
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
    use genesis::aix::agents_block;
    #[test]
    fn aix_agents_block_adoption() {
        let block = agents_block("ah", AH_BLOCK_CONTENT);
        assert!(block.contains("<!-- ah:START -->"));
        assert!(block.contains("<!-- ah:END -->"));
        assert!(block.contains("ah check"));
    }

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
    fn init_without_openspec_prints_minimal_layout() {
        let repo = make_repo(false);
        let result = run_init(repo.path());
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("openspec/") && msg.contains("specs/"),
            "error should describe minimal directory layout, got: {msg}"
        );
        assert!(
            msg.contains("#### Scenario:") || msg.contains("Scenario:"),
            "error should mention #### Scenario: headings expected in specs, got: {msg}"
        );
        assert!(
            msg.contains("Create") || msg.contains("create"),
            "error should give an actionable next step, got: {msg}"
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

    fn once_matches(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }

    #[test]
    fn init_wires_lefthook_managed_block_via_genesis() {
        // RED (espectacular-6ye R2): injection must go through
        // genesis::git_hooks::lefthook::ensure_wired — managed markers +
        // ah-check command land under the column-0 pre-commit anchor.
        let repo = make_repo(true);
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n    lint:\n      run: echo lint\n",
        )
        .unwrap();
        run_init(repo.path()).unwrap();
        let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
        assert!(
            content.contains("ah:managed:start") && content.contains("ah:managed:end"),
            "injected block must carry ah:managed markers; got:\n{content}"
        );
        assert!(content.contains("ah check"));
        assert!(
            content.contains("lint"),
            "existing commands must be preserved"
        );
    }

    #[test]
    fn init_lefthook_wiring_is_idempotent() {
        let repo = make_repo(true);
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n",
        )
        .unwrap();
        run_init(repo.path()).unwrap();
        let twice = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
        run_init(repo.path()).unwrap();
        let twice_after = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
        assert_eq!(
            once_matches(&twice, "ah:managed:start"),
            1,
            "first init injects exactly one managed block"
        );
        assert_eq!(
            once_matches(&twice_after, "ah:managed:start"),
            1,
            "second init must not duplicate the managed block"
        );
        assert!(twice_after.contains("ah check"));
    }

    #[test]
    fn init_installs_prek_integration_when_prek_toml_present() {
        // BEHAVIORAL DRIFT accepted (6ye): prek signal follows genesis's
        // prek.toml; the legacy .prek/prek.yml signals are dropped.
        let repo = make_repo(true);
        fs::write(repo.path().join("prek.toml"), "[hooks]\n").unwrap();
        let result = run_init(repo.path()).unwrap();
        let prek_content = fs::read_to_string(repo.path().join("prek.toml")).unwrap();
        assert!(
            prek_content.contains("ah check")
                || result.refreshed.iter().any(|s| s.contains("prek")),
            "prek.toml should include ah check integration"
        );
    }

    #[test]
    fn init_reports_concern_for_legacy_prek_dotfile_only() {
        // BEHAVIORAL DRIFT accepted (6ye): .prek is no longer a prek signal —
        // genesis detects prek.toml. A repo with only .prek gets a concern,
        // not a config write.
        let repo = make_repo(true);
        fs::write(repo.path().join(".prek"), "").unwrap();
        let result = run_init(repo.path()).unwrap();
        assert!(
            !result.concerns.is_empty(),
            ".prek alone must surface a concern now that prek.toml is the prek signal"
        );
        assert!(
            !result.refreshed.iter().any(|s| s.contains("prek")),
            "must not write to .prek"
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

    // 10.0 RED: espectacular AGENTS.md contains single meta-instruction only
    #[test]
    fn init_writes_single_meta_instruction_to_espectacular_agents_md() {
        let repo = make_repo(true);
        run_init(repo.path()).unwrap();
        let content = fs::read_to_string(repo.path().join(".espectacular/AGENTS.md")).unwrap();
        assert!(
            content.contains("playbook_command"),
            ".espectacular/AGENTS.md must contain meta-instruction referencing playbook_command"
        );
        assert!(
            content.contains("ah explain"),
            ".espectacular/AGENTS.md must reference ah explain"
        );
        assert!(
            !content.contains("## Layout"),
            ".espectacular/AGENTS.md must not contain layout documentation"
        );
        assert!(
            !content.contains("## Workflow"),
            ".espectacular/AGENTS.md must not contain workflow documentation"
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
