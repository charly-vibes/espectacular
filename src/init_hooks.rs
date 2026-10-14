// Init hook integration (lefthook / prek wiring). Split out of src/init.rs
// (espectacular-r2p follow-through): the pretender gate caps app-role files
// at 400 lines; init.rs carried the wiring and its tests past the cap.
use crate::fsutil::write_text;
use crate::init::InitResult;
use anyhow::Context;
use genesis::git_hooks;
use genesis::managed_block::BlockDef;
use std::fs;
use std::path::Path;

// Content is the entries INSIDE the stage's `commands:` mapping
// (genesis-r99 contract): ensure_wired inserts it verbatim after the
// stage's `commands:` line, so it carries the commands:-child indent
// (4 spaces). Leading \n keeps the start marker on its own line:
// ensure_wired concatenates marker + content verbatim, and without it
// the glued line begins with "#" — YAML comments swallow the ah-check
// key (genesis-au8 glue pattern). Found while pinning espectacular-n4v.
const LEFTHOOK_AH_COMMAND: &str = "\n    ah-check:\n      run: ah check\n";

pub(crate) fn install_lefthook(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
    // Managed block wired through genesis::git_hooks::lefthook::ensure_wired
    // (espectacular-6ye R2): column-0 anchor injection + file-level idempotence,
    // replacing the hand-rolled find/split injection.
    let block = BlockDef::with_markers("ah:managed", "# ah:managed:start", "# ah:managed:end");
    let stages = [
        genesis::git_hooks::lefthook::Stage::PreCommit,
        genesis::git_hooks::lefthook::Stage::PrePush,
    ];
    let existing = fs::read_to_string(repo_root.join("lefthook.yml")).unwrap_or_default();
    for stage in stages {
        // GH#47 (espectacular-r2p): ensure_wired's idempotence matches only
        // the exact managed block text — a user-authored `run: ah check`
        // gate outside the block is invisible to it, so injection would
        // double the gate. Scan the stage's section per stage (mirroring
        // the install_prek guard) and skip wiring a stage that already
        // runs ah check. A stage without the user gate is still wired.
        if stage_section_has_ah_check(&existing, stage.key()) {
            continue;
        }
        match genesis::git_hooks::lefthook::ensure_wired(
            repo_root,
            stage,
            &block,
            LEFTHOOK_AH_COMMAND,
        ) {
            Ok(git_hooks::lefthook::WiredOutcome::Injected) => {
                result.refreshed.push("lefthook.yml".to_string());
            }
            Ok(git_hooks::lefthook::WiredOutcome::AlreadyWired) => {}
            Err(e) => return Err(e).with_context(|| "failed to wire ah check into lefthook.yml"),
        }
    }
    Ok(())
}

/// Whether the given lefthook stage's section already contains an `ah check`
/// invocation (GH#47). The section spans from the stage's column-0 key to the
/// next non-blank column-0 line. A missing stage returns false — its wiring
/// proceeds.
fn stage_section_has_ah_check(content: &str, stage_key: &str) -> bool {
    let start = if content.starts_with(stage_key) {
        0
    } else {
        match content.find(&format!("\n{stage_key}:")) {
            Some(index) => index + 1,
            None => return false,
        }
    };
    for line in content[start..].lines().skip(1) {
        if !line.is_empty() && !line.starts_with([' ', '\t']) {
            break;
        }
        if line.contains("ah check") {
            return true;
        }
    }
    false
}

pub(crate) fn install_prek(repo_root: &Path, result: &mut InitResult) -> anyhow::Result<()> {
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
    use crate::init::run_init;
    use tempfile::TempDir;

    use std::fs;

    fn make_repo(has_openspec: bool) -> TempDir {
        let dir = TempDir::new().unwrap();
        if has_openspec {
            fs::create_dir_all(dir.path().join("openspec")).unwrap();
        }
        dir
    }

    // 4.1 RED: ah init creates expected files in fresh repo with openspec/

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
    fn init_wires_ah_check_into_prepush_stage() {
        // RED (espectacular-6ye R4): init must wire ah check into the
        // pre-push stage too — hooks that only gate pre-commit stay decorative
        // for the push-time trust gate.
        let repo = make_repo(true);
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n",
        )
        .unwrap();
        run_init(repo.path()).unwrap();
        let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
        assert!(
            content.contains("pre-push:"),
            "init must create a pre-push stage"
        );
        let push_section = &content[content.find("pre-push:").unwrap()..];
        assert!(
            push_section.contains("ah check"),
            "pre-push stage must run ah check"
        );
    }

    #[test]
    fn init_wires_lefthook_managed_block_via_genesis() {
        // RED (espectacular-6ye R2): injection must go through
        // genesis::git_hooks::lefthook::ensure_wired — managed markers +
        // ah-check command nested inside the stage's `commands:` mapping
        // (genesis-r99: a stage-level key is silently ignored by lefthook).
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
            content
                .contains("  commands:\n# ah:managed:start\n    ah-check:\n      run: ah check\n"),
            "ah-check must nest inside commands: (genesis-r99); got:\n{content}"
        );
        assert!(
            content.contains("lint"),
            "existing commands must be preserved"
        );
    }

    // GH#47 (espectacular-r2p): a stage that already runs ah check via a
    // user-authored gate must not gain the managed duplicate — per-stage
    // pre-insertion scan, mirroring the install_prek guard.
    #[test]
    fn init_skips_wiring_stage_with_existing_user_ah_check_gate() {
        let repo = make_repo(true);
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n    ah-check-gate:\n      run: ah check\n",
        )
        .unwrap();
        run_init(repo.path()).unwrap();
        let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
        let commit_end = content.find("\npre-push:").unwrap_or(content.len());
        let commit_section = &content[..commit_end];
        assert_eq!(
            once_matches(commit_section, "run: ah check"),
            1,
            "user gate must be preserved and the managed block must not duplicate it; got:\n{commit_section}"
        );
        assert!(
            !commit_section.contains("ah:managed:start"),
            "pre-commit must not gain the managed block; got:\n{commit_section}"
        );
        // pre-push has no user gate — it must still be wired.
        let push_section = &content[content.find("pre-push:").unwrap()..];
        assert!(
            push_section.contains("ah check"),
            "pre-push without a user gate must still be wired; got:\n{push_section}"
        );
    }

    // GH#47: the scan is per-stage — a user gate in pre-push must not
    // suppress the pre-commit wiring, and vice versa.
    #[test]
    fn init_user_gate_in_prepush_does_not_suppress_precommit_wiring() {
        let repo = make_repo(true);
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-push:\n  commands:\n    ah-check-gate:\n      run: ah check\n",
        )
        .unwrap();
        run_init(repo.path()).unwrap();
        let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
        assert_eq!(
            once_matches(&content, "ah:managed:start"),
            1,
            "only the pre-push-less stage (pre-commit) gains a managed block; got:\n{content}"
        );
        let push_end = content.find("\npre-commit:").unwrap_or(content.len());
        let push_section = &content[content.find("pre-push:").unwrap()..push_end];
        assert_eq!(
            once_matches(push_section, "run: ah check"),
            1,
            "pre-push user gate preserved without managed duplicate"
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
            2,
            "first init injects one managed block per stage (pre-commit + pre-push)"
        );
        assert_eq!(
            once_matches(&twice_after, "ah:managed:start"),
            2,
            "second init must not duplicate the managed blocks"
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
}
