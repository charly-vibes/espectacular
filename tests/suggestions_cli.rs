use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn ah_unknown_subcommand_prints_genesis_suggestion() {
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["doctr"])
        .assert()
        .failure()
        .stderr(contains("Did you mean 'doctor'"));

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("Did you mean 'doctor'"),
        "typo must suggest the near-miss subcommand: {stderr}"
    );
}

#[test]
fn ah_chekc_typo_prints_genesis_suggestion() {
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["chekc"])
        .assert()
        .failure()
        .stderr(contains("Did you mean 'check'"));

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("Did you mean 'check'"),
        "typo must suggest the near-miss subcommand: {stderr}"
    );
}
