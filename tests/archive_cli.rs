use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn make_repo(dir: &TempDir) {
    fs::create_dir_all(dir.path().join("openspec/specs")).unwrap();
    fs::create_dir_all(dir.path().join(".espectacular")).unwrap();
}

fn add_deployed_spec(dir: &TempDir, spec: &str, heading: &str) {
    let spec_dir = dir.path().join("openspec/specs").join(spec);
    fs::create_dir_all(&spec_dir).unwrap();
    let content = format!(
        "# Cap\n\n### Requirement: Core\n\n#### Scenario: {heading}\n- **WHEN** x\n- **THEN** y\n"
    );
    fs::write(spec_dir.join("spec.md"), content).unwrap();
}

fn add_staged_contract(dir: &TempDir, change: &str, spec: &str, slug: &str) {
    let staged_dir = dir
        .path()
        .join(".espectacular/changes")
        .join(change)
        .join(spec);
    fs::create_dir_all(&staged_dir).unwrap();
    let content = format!(
        "id = \"{slug}\"\ndescription = \"x\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n"
    );
    fs::write(staged_dir.join(format!("{slug}.toml")), content).unwrap();
}

#[test]
fn ah_archive_exits_zero_and_prints_archived_path() {
    let dir = TempDir::new().unwrap();
    make_repo(&dir);
    add_deployed_spec(&dir, "compiler", "Empty input rejected");
    add_staged_contract(&dir, "s5", "compiler", "empty-input-rejected");

    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["archive", "s5"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "archived: compiler/empty-input-rejected",
        ));
}

#[test]
fn ah_archive_exits_nonzero_on_collision() {
    let dir = TempDir::new().unwrap();
    make_repo(&dir);
    add_deployed_spec(&dir, "compiler", "Empty input rejected");
    add_staged_contract(&dir, "s5", "compiler", "empty-input-rejected");
    // Add active base contract → collision
    let base_dir = dir.path().join(".espectacular/compiler");
    fs::create_dir_all(&base_dir).unwrap();
    fs::write(
        base_dir.join("empty-input-rejected.toml"),
        "id = \"empty-input-rejected\"\ndescription = \"x\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n",
    ).unwrap();

    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["archive", "s5"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("collision"));
}

#[test]
fn ah_archive_exits_nonzero_when_no_staged_change() {
    let dir = TempDir::new().unwrap();
    make_repo(&dir);

    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["archive", "s5"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("no staged contracts"));
}
