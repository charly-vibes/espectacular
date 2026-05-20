use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn ah_type_lists_all_archetypes() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["type"])
        .assert()
        .success()
        .stdout(contains("PF"))
        .stdout(contains("SA"))
        .stdout(contains("BP"))
        .stdout(contains("CE"))
        .stdout(contains("NR"));
}

#[test]
fn ah_type_list_includes_one_line_descriptions() {
    let output = Command::cargo_bin("ah")
        .unwrap()
        .args(["type"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    for code in ["PF", "SA", "BP", "CE", "NR"] {
        let line = text
            .lines()
            .find(|l| l.contains(code))
            .unwrap_or_else(|| panic!("no line containing {code} in output:\n{text}"));
        assert!(
            line.len() > code.len() + 5,
            "line for {code} has no description: {line}"
        );
    }
}

#[test]
fn ah_type_known_archetype_prints_full_body() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "PF"])
        .assert()
        .success()
        .stdout(contains("Pure Functional"))
        .stdout(contains("Typical test shapes"));
}

#[test]
fn ah_type_lookup_is_case_insensitive() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "pf"])
        .assert()
        .success()
        .stdout(contains("## PF — Pure Functional"));
}

#[test]
fn ah_type_unknown_archetype_exits_nonzero() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "ZZ"])
        .assert()
        .failure();
}

#[test]
fn ah_type_unknown_archetype_suggests_did_you_mean() {
    Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "n"])
        .assert()
        .failure()
        .stderr(contains("Did you mean"))
        .stderr(contains("NR"));
}
