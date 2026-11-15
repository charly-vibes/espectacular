use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;

fn ah() -> Command {
    Command::cargo_bin("ah").unwrap()
}

/// Builds a repo whose lefthook.yml names the stage in comment lines
/// before the real column-0 key.
fn comment_heavy_lefthook_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs/demo")).unwrap();
    fs::write(
        root.join("openspec/specs/demo/spec.md"),
        "## Purpose\ndemo\n#### Scenario: demo works\n- **WHEN** x\n- **THEN** y\n",
    )
    .unwrap();
    fs::write(
        root.join("lefthook.yml"),
        "# pre-commit hooks run before each commit\n\
         # see https://lefthook.dev for pre-commit docs\n\
         pre-commit:\n  commands:\n    lint:\n      run: echo linting\n",
    )
    .unwrap();
    dir
}

/// Runs `ah init` in `root`, asserting it accepts the comment-heavy file.
fn run_init_accepting_lefthook(root: &std::path::Path) {
    let assert = ah()
        .current_dir(root)
        .args(["init"])
        .env_remove("AH_")
        .assert()
        .success()
        .stderr(contains("unanchorable").not());
    assert_eq!(
        assert.get_output().status.code(),
        Some(0),
        "ah init must exit 0 for an anchorable lefthook.yml"
    );
}

/// Asserts the managed block shape for both wired stages.
fn assert_managed_block_shape(yml: &str) {
    // Block shape: start marker on its own line, ah-check as a real YAML
    // key (not swallowed by a glued comment line), end marker on its own
    // line — for both stages (init wires pre-commit AND pre-push).
    assert_eq!(
        yml.matches("# ah:managed:start\n").count(),
        2,
        "each stage must carry a block with the start marker on its own line:\n{yml}"
    );
    assert_eq!(
        yml.matches("\n    ah-check:\n      run: ah check\n")
            .count(),
        2,
        "ah-check must be a real YAML key under each stage's commands: mapping:
{yml}"
    );
    assert!(
        yml.contains("pre-commit:\n  commands:\n# ah:managed:start\n"),
        "block must nest inside the stage's commands: mapping — lefthook \
         silently ignores stage-level keys (genesis-r99 / GH#31):\n{yml}"
    );
}

/// Regression for espectacular-n4v (GH#28 item 1): a lefthook.yml whose
/// stage name appears in comment lines *before* the real column-0 key
/// must not be refused as "unanchorable". genesis 0.11.1 fixed the
/// upstream find_anchor false positive (genesis-tzw); this pins the
/// downstream e2e behavior: ah init succeeds and wires the block.
#[test]
fn ah_init_anchors_lefthook_with_comments_mentioning_stage() {
    let dir = comment_heavy_lefthook_fixture();
    let root = dir.path();
    run_init_accepting_lefthook(root);

    let yml = fs::read_to_string(root.join("lefthook.yml")).unwrap();
    assert_managed_block_shape(&yml);

    // idempotence: a second init must not duplicate the block or refuse
    run_init_accepting_lefthook(root);
    let yml2 = fs::read_to_string(root.join("lefthook.yml")).unwrap();
    assert_eq!(
        yml2.matches("ah:managed:start").count(),
        2, // pre-commit + pre-push, one block each
        "block must not duplicate on re-init:\n{yml2}"
    );
    assert_eq!(yml2, yml, "re-init must leave the file byte-identical");
}

#[test]
fn p_init() {
    ah_init_anchors_lefthook_with_comments_mentioning_stage();
    // Aggregator sanity: the init command surface the anchored case exercises
    // must exist on the built binary.
    let help = ah().args(["init", "--help"]).output().unwrap();
    assert!(
        help.status.success(),
        "ah init --help must exit 0, stderr: {}",
        String::from_utf8_lossy(&help.stderr)
    );
}
