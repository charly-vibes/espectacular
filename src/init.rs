use crate::fsutil::write_text;
use anyhow::Context;
use genesis::discovery;
use genesis::git_hooks;
use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};
use genesis::scaffold::Scaffold;
use std::path::Path;

#[derive(Debug, Default)]
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
        "Error: openspec/ directory not found at {}.\n\nah requires an OpenSpec project. Create the minimal layout:\n\n  openspec/\n  └─── specs/\n          └─── spec.md      ← \"#### Scenario: ...\" headings go here\n\nOr install the openspec CLI and run: openspec init",
        repo_root.display()
    );

    let mut result = InitResult::default();

    scaffold_espectacular(repo_root, &mut result)?;
    write_espectacular_agents(repo_root, &mut result)?;
    update_managed_file(&repo_root.join("AGENTS.md"), &mut result)?;
    update_managed_file(&repo_root.join("CLAUDE.md"), &mut result)?;
    inject_ah_block_into_espectacular_agents(repo_root, &mut result)?;
    crate::init_contracts::stub_missing_contracts(repo_root, &mut result)?;
    install_hook_integration(repo_root, &mut result)?;
    register_discovery(repo_root, &mut result);

    Ok(result)
}

/// Create `.espectacular/` and its default config.toml via genesis::scaffold.
fn scaffold_espectacular(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
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
    Ok(())
}

/// `.espectacular/AGENTS.md` — always refreshed with the current content.
fn write_espectacular_agents(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    let agents = repo_root.join(".espectacular/AGENTS.md");
    let agents_existed = agents.exists();
    write_text(&agents, ESPECTACULAR_AGENTS_CONTENT)?;
    if agents_existed {
        result.refreshed.push(".espectacular/AGENTS.md".into());
    } else {
        result.created.push(".espectacular/AGENTS.md".into());
    }
    Ok(())
}

/// Also inject the managed block into `.espectacular/AGENTS.md` when present.
fn inject_ah_block_into_espectacular_agents(
    repo_root: &Path,
    result: &mut InitResult,
) -> anyhow::Result<()> {
    let ah_agents = repo_root.join(".espectacular/AGENTS.md");
    if !ah_agents.exists() {
        return Ok(());
    }
    let injector = ah_block_injector();
    if !injector.has_block(&ah_agents, "ah:managed") {
        injector.inject(&ah_agents, "ah:managed", AH_BLOCK_CONTENT)?;
        result.refreshed.push(".espectacular/AGENTS.md".into());
    }
    Ok(())
}

/// Hook integration (espectacular-6ye R2: genesis::git_hooks detection).
fn install_hook_integration(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    match git_hooks::framework(repo_root) {
        git_hooks::Framework::Lefthook => crate::init_hooks::install_lefthook(repo_root, result),
        git_hooks::Framework::Prek => crate::init_hooks::install_prek(repo_root, result),
        git_hooks::Framework::Husky => {
            result.concerns.push(
                "Husky detected but not wirable by ah (lefthook only). \
                 Add `ah check` to your husky pre-commit hook manually to run \
                 spec-test correspondence before commits."
                    .into(),
            );
            Ok(())
        }
        git_hooks::Framework::None => {
            result.concerns.push(
                "No supported pre-commit hook framework detected (lefthook or prek). \
                Please set up pre-commit integration manually to run `ah check` before commits."
                    .into(),
            );
            Ok(())
        }
    }
}

/// Register in .genesis/tools.toml for cross-tool discovery.
fn register_discovery(repo_root: &Path, result: &mut InitResult) {
    if let Err(e) = discovery::register(
        repo_root,
        "espectacular",
        "Spec-test correspondence enforcer",
        "directory",
        ".espectacular",
    ) {
        result.concerns.push(format!(".genesis/tools.toml: {e}"));
    }
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
}
