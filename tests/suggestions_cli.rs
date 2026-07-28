use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn ah_unknown_subcommand_prints_genesis_suggestion() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["doctr"])
        .assert()
        .failure()
        .stderr(contains("Did you mean 'doctor'"));
}

#[test]
fn ah_chekc_typo_prints_genesis_suggestion() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["chekc"])
        .assert()
        .failure()
        .stderr(contains("Did you mean 'check'"));
}
