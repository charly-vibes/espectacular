use anyhow::Context;
use std::path::Path;

#[derive(Debug)]
pub struct ArchiveResult {
    pub moved: Vec<String>,
}

pub fn run_archive(repo_root: &Path, change: &str) -> anyhow::Result<ArchiveResult> {
    let staged_root = repo_root.join(".espectacular/changes").join(change);

    anyhow::ensure!(
        staged_root.exists(),
        "no staged contracts found for change '{change}'"
    );

    // Collect all staged contracts: .espectacular/changes/<change>/<spec>/<slug>.toml
    let mut staged: Vec<(String, String)> = Vec::new(); // (spec, slug)
    for spec_entry in std::fs::read_dir(&staged_root)
        .with_context(|| format!("cannot read {}", staged_root.display()))?
    {
        let spec_entry = spec_entry?;
        if !spec_entry.file_type()?.is_dir() {
            continue;
        }
        let spec_name = spec_entry.file_name().to_string_lossy().to_string();
        for contract_entry in std::fs::read_dir(spec_entry.path())? {
            let contract_entry = contract_entry?;
            let fname = contract_entry.file_name().to_string_lossy().to_string();
            if fname.ends_with(".toml") {
                let slug = fname.trim_end_matches(".toml").to_string();
                staged.push((spec_name.clone(), slug));
            }
        }
    }

    anyhow::ensure!(
        !staged.is_empty(),
        "no staged contracts found for change '{change}'"
    );

    // Precondition 1: check OpenSpec-archive counterparts exist
    let mut orphans: Vec<String> = Vec::new();
    for (spec, slug) in &staged {
        let deployed_spec = repo_root.join("openspec/specs").join(spec).join("spec.md");
        let has_counterpart = if deployed_spec.exists() {
            let content = std::fs::read_to_string(&deployed_spec)?;
            content.contains("#### Scenario:") && scenario_slug_in_spec(&content, slug)
        } else {
            false
        };
        if !has_counterpart {
            orphans.push(format!("{spec}/{slug}"));
        }
    }
    if !orphans.is_empty() {
        anyhow::bail!(
            "pre-OpenSpec-archive orphans (run `openspec archive {change}` first): {}",
            orphans.join(", ")
        );
    }

    // Precondition 2: check for collisions (base contract exists and is NOT superseded)
    let mut collisions: Vec<String> = Vec::new();
    for (spec, slug) in &staged {
        let base_path = repo_root
            .join(".espectacular")
            .join(spec)
            .join(format!("{slug}.toml"));
        if base_path.exists() {
            let text = std::fs::read_to_string(&base_path)?;
            let is_superseded = text.contains("status = \"superseded\"");
            if !is_superseded {
                collisions.push(format!("{spec}/{slug}"));
            }
        }
    }
    if !collisions.is_empty() {
        anyhow::bail!(
            "collision with existing active contracts: {}",
            collisions.join(", ")
        );
    }

    // All preconditions passed — move staged contracts to base
    let mut moved = Vec::new();
    for (spec, slug) in &staged {
        let src = staged_root.join(spec).join(format!("{slug}.toml"));
        let dest_dir = repo_root.join(".espectacular").join(spec);
        std::fs::create_dir_all(&dest_dir)?;
        let dest = dest_dir.join(format!("{slug}.toml"));
        std::fs::copy(&src, &dest).with_context(|| format!("cannot copy {}", src.display()))?;
        std::fs::remove_file(&src).with_context(|| format!("cannot remove {}", src.display()))?;
        moved.push(format!("{spec}/{slug}"));
    }

    // Remove now-empty spec dirs under changes/<change>/
    for (spec, _) in &staged {
        let dir = staged_root.join(spec);
        let _ = std::fs::remove_dir(&dir); // ok if not empty (shouldn't happen)
    }
    let _ = std::fs::remove_dir(&staged_root);

    Ok(ArchiveResult { moved })
}

fn scenario_slug_in_spec(content: &str, slug: &str) -> bool {
    // Check if any "#### Scenario: <heading>" in content, when slugified, matches slug
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("#### Scenario: ") {
            if crate::openspec::slugify(heading) == slug {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_repo(dir: &Path) {
        fs::create_dir_all(dir.join("openspec/specs")).unwrap();
        fs::create_dir_all(dir.join(".espectacular")).unwrap();
    }

    fn add_deployed_spec(dir: &Path, spec: &str, scenario_heading: &str) {
        let spec_dir = dir.join("openspec/specs").join(spec);
        fs::create_dir_all(&spec_dir).unwrap();
        let content = format!(
            "# Capability\n\n### Requirement: Core\n\n#### Scenario: {scenario_heading}\n- **WHEN** x\n- **THEN** y\n"
        );
        fs::write(spec_dir.join("spec.md"), content).unwrap();
    }

    fn add_base_contract(dir: &Path, spec: &str, slug: &str, status: &str) {
        let base_dir = dir.join(".espectacular").join(spec);
        fs::create_dir_all(&base_dir).unwrap();
        let superseded_by = if status == "superseded" {
            "\"new-scenario\""
        } else {
            "\"\""
        };
        let content = format!(
            "id = \"{slug}\"\ndescription = \"x\"\narchetype = \"PF\"\nstatus = \"{status}\"\nsuperseded_by = {superseded_by}\nauthored_with = \"0.1.0\"\n"
        );
        fs::write(base_dir.join(format!("{slug}.toml")), content).unwrap();
    }

    fn add_staged_contract(dir: &Path, change: &str, spec: &str, slug: &str) {
        let staged_dir = dir.join(".espectacular/changes").join(change).join(spec);
        fs::create_dir_all(&staged_dir).unwrap();
        let content = format!(
            "id = \"{slug}\"\ndescription = \"x\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n"
        );
        fs::write(staged_dir.join(format!("{slug}.toml")), content).unwrap();
    }

    #[test]
    fn archive_moves_staged_contracts_to_base() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());
        add_deployed_spec(dir.path(), "compiler", "Empty input rejected");
        add_staged_contract(dir.path(), "s5", "compiler", "empty-input-rejected");

        let result = run_archive(dir.path(), "s5").unwrap();

        assert_eq!(result.moved, vec!["compiler/empty-input-rejected"]);
        let base = dir
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml");
        assert!(base.exists(), "base contract should exist");
        let staged = dir
            .path()
            .join(".espectacular/changes/s5/compiler/empty-input-rejected.toml");
        assert!(!staged.exists(), "staged contract should be removed");
    }

    #[test]
    fn archive_removes_staged_dirs_after_move() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());
        add_deployed_spec(dir.path(), "compiler", "Empty input rejected");
        add_staged_contract(dir.path(), "s5", "compiler", "empty-input-rejected");

        run_archive(dir.path(), "s5").unwrap();

        let change_dir = dir.path().join(".espectacular/changes/s5");
        assert!(!change_dir.exists(), "staged change dir should be removed");
    }

    #[test]
    fn archive_fails_when_no_staged_change() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());

        let err = run_archive(dir.path(), "s5").unwrap_err();
        assert!(
            err.to_string().contains("no staged contracts"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn archive_fails_on_collision_with_active_base_contract() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());
        add_deployed_spec(dir.path(), "compiler", "Empty input rejected");
        add_staged_contract(dir.path(), "s5", "compiler", "empty-input-rejected");
        add_base_contract(dir.path(), "compiler", "empty-input-rejected", "active");

        let err = run_archive(dir.path(), "s5").unwrap_err();
        assert!(err.to_string().contains("collision"), "wrong error: {err}");
        assert!(
            err.to_string().contains("compiler/empty-input-rejected"),
            "error should name the collision: {err}"
        );
    }

    #[test]
    fn archive_allows_replacement_of_superseded_base_contract() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());
        add_deployed_spec(dir.path(), "compiler", "Empty input rejected");
        // Base contract exists but is superseded
        add_base_contract(dir.path(), "compiler", "empty-input-rejected", "superseded");
        add_staged_contract(dir.path(), "s5", "compiler", "empty-input-rejected");

        let result = run_archive(dir.path(), "s5").unwrap();
        assert_eq!(result.moved, vec!["compiler/empty-input-rejected"]);
    }

    #[test]
    fn archive_fails_when_scenario_not_in_deployed_spec() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());
        // deployed spec does NOT contain the scenario
        add_deployed_spec(dir.path(), "compiler", "Some other scenario");
        add_staged_contract(dir.path(), "s5", "compiler", "empty-input-rejected");

        let err = run_archive(dir.path(), "s5").unwrap_err();
        assert!(
            err.to_string().contains("pre-OpenSpec-archive orphan"),
            "wrong error: {err}"
        );
        assert!(
            err.to_string().contains("compiler/empty-input-rejected"),
            "error should name the orphan: {err}"
        );
    }

    #[test]
    fn archive_fails_when_deployed_spec_missing_entirely() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());
        // No deployed spec at all
        add_staged_contract(dir.path(), "s5", "compiler", "empty-input-rejected");

        let err = run_archive(dir.path(), "s5").unwrap_err();
        assert!(
            err.to_string().contains("pre-OpenSpec-archive orphan"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn archive_moves_multiple_specs_and_contracts() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());
        add_deployed_spec(dir.path(), "compiler", "Empty input rejected");
        add_deployed_spec(dir.path(), "linter", "No warnings on clean file");
        add_staged_contract(dir.path(), "s5", "compiler", "empty-input-rejected");
        add_staged_contract(dir.path(), "s5", "linter", "no-warnings-on-clean-file");

        let result = run_archive(dir.path(), "s5").unwrap();
        assert_eq!(result.moved.len(), 2);
        assert!(result
            .moved
            .contains(&"compiler/empty-input-rejected".to_string()));
        assert!(result
            .moved
            .contains(&"linter/no-warnings-on-clean-file".to_string()));
    }

    #[test]
    fn archive_fails_listing_all_collisions() {
        let dir = TempDir::new().unwrap();
        make_repo(dir.path());
        // Write spec with both scenarios in one file
        let spec_dir = dir.path().join("openspec/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        fs::write(
            spec_dir.join("spec.md"),
            "# Cap\n\n### Requirement: Core\n\n#### Scenario: Empty input rejected\n- **WHEN** x\n- **THEN** y\n\n#### Scenario: Null bytes rejected\n- **WHEN** x\n- **THEN** y\n",
        ).unwrap();
        // Two staged contracts, both collide
        add_staged_contract(dir.path(), "s5", "compiler", "empty-input-rejected");
        add_staged_contract(dir.path(), "s5", "compiler", "null-bytes-rejected");
        add_base_contract(dir.path(), "compiler", "empty-input-rejected", "active");
        add_base_contract(dir.path(), "compiler", "null-bytes-rejected", "active");

        let err = run_archive(dir.path(), "s5").unwrap_err();
        assert!(
            err.to_string().contains("compiler/empty-input-rejected"),
            "should list first collision: {err}"
        );
        assert!(
            err.to_string().contains("compiler/null-bytes-rejected"),
            "should list second collision: {err}"
        );
    }
}
