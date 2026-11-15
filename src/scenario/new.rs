use crate::openspec::slugify;
use anyhow::Context;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ScenarioNewResult {
    pub scenario_path: String,
    pub contract_path: String,
}

pub fn run_scenario_new(
    repo_root: &Path,
    change: &str,
    spec: &str,
    requirement: &str,
    heading: &str,
) -> anyhow::Result<ScenarioNewResult> {
    let spec_path = change_spec_path(repo_root, change, spec);
    anyhow::ensure!(
        spec_path.exists(),
        "change spec not found: openspec/changes/{change}/specs/{spec}/spec.md"
    );

    let content = read_with_context(&spec_path)?;
    let (section_start, section_end) = requirement_bounds(&content, requirement)?;
    ensure_scenario_absent(&content[section_start..section_end], heading, requirement)?;

    let new_content = insert_scenario(&content, section_end, heading);
    write_with_context(&spec_path, &new_content)?;

    let slug = slugify(heading);
    let contract_path = write_staged_contract(repo_root, change, spec, &slug)?;
    Ok(ScenarioNewResult {
        scenario_path: format!("openspec/changes/{change}/specs/{spec}/spec.md"),
        contract_path,
    })
}

fn change_spec_path(repo_root: &Path, change: &str, spec: &str) -> PathBuf {
    repo_root
        .join("openspec/changes")
        .join(change)
        .join("specs")
        .join(spec)
        .join("spec.md")
}

fn read_with_context(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))
}

fn write_with_context(path: &Path, content: &str) -> anyhow::Result<()> {
    std::fs::write(path, content).with_context(|| format!("cannot write {}", path.display()))
}

/// Byte range of a requirement body: just after its `### Requirement:` line up
/// to the next `### ` heading (or EOF).
fn requirement_bounds(content: &str, requirement: &str) -> anyhow::Result<(usize, usize)> {
    let req_needle = format!("### Requirement: {requirement}");
    let req_pos = content
        .find(&req_needle)
        .ok_or_else(|| anyhow::anyhow!("requirement '{requirement}' not found in spec"))?;
    let section_start = req_pos + req_needle.len();
    let section_end = content[section_start..]
        .find("\n### ")
        .map(|p| section_start + p)
        .unwrap_or(content.len());
    Ok((section_start, section_end))
}

fn ensure_scenario_absent(section: &str, heading: &str, requirement: &str) -> anyhow::Result<()> {
    let collision_needle = format!("#### Scenario: {heading}");
    anyhow::ensure!(
        !section.contains(&collision_needle),
        "scenario '{heading}' already exists under requirement '{requirement}'"
    );
    Ok(())
}

fn insert_scenario(content: &str, at: usize, heading: &str) -> String {
    let skeleton = format!(
        "\n\n#### Scenario: {heading}\n- **WHEN** [describe the action or condition]\n- **THEN** [describe the expected observable result]"
    );
    format!("{}{}{}", &content[..at], skeleton, &content[at..])
}

fn write_staged_contract(
    repo_root: &Path,
    change: &str,
    spec: &str,
    slug: &str,
) -> anyhow::Result<String> {
    let contract_dir = repo_root
        .join(".espectacular/changes")
        .join(change)
        .join(spec);
    std::fs::create_dir_all(&contract_dir)?;

    let version = env!("CARGO_PKG_VERSION");
    let toml_content = format!(
        "id = \"{slug}\"\ndescription = \"\"\narchetype = \"\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"{version}\"\n"
    );
    let contract_file = contract_dir.join(format!("{slug}.toml"));
    write_with_context(&contract_file, &toml_content)?;
    Ok(format!(".espectacular/changes/{change}/{spec}/{slug}.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_repo_with_change(change: &str, spec: &str, requirement: &str) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let spec_dir = root
            .join("openspec/changes")
            .join(change)
            .join("specs")
            .join(spec);
        fs::create_dir_all(&spec_dir).unwrap();

        let spec_md = format!(
            "# Capability: {spec}\n\n## ADDED Requirements\n\n### Requirement: {requirement}\n"
        );
        fs::write(spec_dir.join("spec.md"), &spec_md).unwrap();
        dir
    }

    // ── 4.9 RED: ah scenario new happy-path ──────────────────────────────────

    #[test]
    fn new_scenario_creates_markdown_skeleton() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let result = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        let spec_path = dir.path().join(&result.scenario_path);
        let content = fs::read_to_string(&spec_path).unwrap();
        assert!(
            content.contains("#### Scenario: Null bytes rejected"),
            "skeleton heading missing"
        );
        assert!(
            content.contains("- **WHEN** [describe the action or condition]"),
            "WHEN placeholder missing"
        );
        assert!(
            content.contains("- **THEN** [describe the expected observable result]"),
            "THEN placeholder missing"
        );
    }

    #[test]
    fn new_scenario_creates_staged_contract() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let result = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        let contract_path = dir.path().join(&result.contract_path);
        assert!(contract_path.exists(), "contract file not created");
        assert_eq!(
            result.contract_path,
            ".espectacular/changes/s5/compiler/null-bytes-rejected.toml"
        );
    }

    #[test]
    fn new_scenario_toml_has_correct_defaults() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let result = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        let contract_path = dir.path().join(&result.contract_path);
        let text = fs::read_to_string(&contract_path).unwrap();
        assert!(text.contains("id = \"null-bytes-rejected\""), "id wrong");
        assert!(text.contains("status = \"active\""), "status wrong");
        assert!(text.contains("superseded_by = \"\""), "superseded_by wrong");
        assert!(text.contains("description = \"\""), "description wrong");
        assert!(text.contains("archetype = \"\""), "archetype wrong");
        assert!(
            text.contains(&format!(
                "authored_with = \"{}\"",
                env!("CARGO_PKG_VERSION")
            )),
            "authored_with wrong"
        );
    }

    #[test]
    fn new_scenario_appends_after_existing_scenarios() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let spec_dir = root.join("openspec/changes/s5/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        let initial = "# Capability: compiler\n\n## ADDED Requirements\n\n### Requirement: Input validation\n\n#### Scenario: Empty input rejected\n- **WHEN** a\n- **THEN** b\n\n### Requirement: Other\n";
        fs::write(spec_dir.join("spec.md"), initial).unwrap();

        run_scenario_new(
            root,
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        let content = fs::read_to_string(spec_dir.join("spec.md")).unwrap();
        let pos_empty = content.find("Empty input rejected").unwrap();
        let pos_null = content.find("Null bytes rejected").unwrap();
        let pos_other = content.find("### Requirement: Other").unwrap();
        assert!(pos_empty < pos_null, "new scenario before existing");
        assert!(pos_null < pos_other, "new scenario after Other section");
    }

    #[test]
    fn new_scenario_returns_correct_paths() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let result = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        assert_eq!(
            result.scenario_path,
            "openspec/changes/s5/specs/compiler/spec.md"
        );
        assert_eq!(
            result.contract_path,
            ".espectacular/changes/s5/compiler/null-bytes-rejected.toml"
        );
    }

    // ── 4.11 RED: non-destructive failure ────────────────────────────────────

    #[test]
    fn new_scenario_fails_when_change_spec_missing() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("change spec not found"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn new_scenario_fails_when_requirement_missing() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let err = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Nonexistent requirement",
            "Some scenario",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("not found in spec"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn new_scenario_fails_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let spec_dir = root.join("openspec/changes/s5/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        let initial = "# Capability\n\n### Requirement: Input validation\n\n#### Scenario: Null bytes rejected\n- **WHEN** a\n- **THEN** b\n";
        fs::write(spec_dir.join("spec.md"), initial).unwrap();

        let err = run_scenario_new(
            root,
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("already exists"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn new_scenario_writes_nothing_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let _ = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Some heading",
        );
        let contract_dir = dir.path().join(".espectacular/changes/s5/compiler");
        assert!(
            !contract_dir.exists(),
            "contract dir should not be created on failure"
        );
    }
}
