use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn make_repo_with_config(dir: &TempDir, tool_version: &str) {
    fs::create_dir_all(dir.path().join(".espectacular")).unwrap();
    let config = format!(
        "tool_version = \"{tool_version}\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncargo = [\"cargo\", \"test\"]\n"
    );
    fs::write(dir.path().join(".espectacular/config.toml"), config).unwrap();
}

#[test]
fn ah_upgrade_exits_zero_when_config_matches_binary() {
    let dir = TempDir::new().unwrap();
    make_repo_with_config(&dir, env!("CARGO_PKG_VERSION"));

    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["upgrade"])
        .assert()
        .success()
        .stdout(predicates::str::contains("up to date"));
}

#[test]
fn ah_upgrade_exits_nonzero_on_version_drift() {
    let dir = TempDir::new().unwrap();
    make_repo_with_config(&dir, "0.0.9");

    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["upgrade"])
        .assert()
        .failure();
}

#[test]
fn ah_upgrade_reports_old_and_new_version_on_drift() {
    let dir = TempDir::new().unwrap();
    make_repo_with_config(&dir, "0.0.9");

    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["upgrade"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("0.0.9"))
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn ah_upgrade_updates_config_tool_version_on_drift() {
    let dir = TempDir::new().unwrap();
    make_repo_with_config(&dir, "0.0.9");

    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["upgrade"])
        .assert()
        .failure();

    let config_text = fs::read_to_string(dir.path().join(".espectacular/config.toml")).unwrap();
    assert!(
        config_text.contains(&format!("tool_version = \"{}\"", env!("CARGO_PKG_VERSION"))),
        "config should be updated to binary version: {config_text}"
    );
}

#[test]
fn ah_upgrade_does_not_modify_authored_with_in_contracts() {
    let dir = TempDir::new().unwrap();
    make_repo_with_config(&dir, "0.0.9");

    fs::create_dir_all(dir.path().join(".espectacular/compiler")).unwrap();
    let contract = "id = \"my-scenario\"\ndescription = \"x\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.0.9\"\n";
    fs::write(
        dir.path().join(".espectacular/compiler/my-scenario.toml"),
        contract,
    )
    .unwrap();

    Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["upgrade"])
        .assert()
        .failure();

    let contract_text =
        fs::read_to_string(dir.path().join(".espectacular/compiler/my-scenario.toml")).unwrap();
    assert!(
        contract_text.contains("authored_with = \"0.0.9\""),
        "authored_with must not be modified by upgrade: {contract_text}"
    );
}
