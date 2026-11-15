use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn ah_type_lists_all_archetypes() {
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["type"])
        .assert()
        .success()
        .stdout(contains("PF"))
        .stdout(contains("SA"))
        .stdout(contains("BP"))
        .stdout(contains("CE"))
        .stdout(contains("NR"));

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    for code in ["PF", "SA", "BP", "CE", "NR"] {
        assert!(
            stdout.contains(code),
            "type listing must include archetype {code}: {stdout}"
        );
    }
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
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "PF"])
        .assert()
        .success()
        .stdout(contains("Pure Functional"))
        .stdout(contains("Typical test shapes"));

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("Pure Functional"),
        "type PF must print the archetype name: {stdout}"
    );
    assert!(
        stdout.contains("Typical test shapes"),
        "type PF must print the test-shape guidance: {stdout}"
    );
}

#[test]
fn ah_type_lookup_is_case_insensitive() {
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "pf"])
        .assert()
        .success()
        .stdout(contains("## PF — Pure Functional"));

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.starts_with("## PF — Pure Functional"),
        "lowercase lookup must print the canonical PF heading: {stdout}"
    );
}

#[test]
fn ah_type_unknown_archetype_exits_nonzero() {
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "ZZ"])
        .assert()
        .failure();

    let out = assert.get_output();
    assert_ne!(
        out.status.code(),
        Some(0),
        "unknown archetype must exit nonzero"
    );
    assert!(
        !out.stderr.is_empty(),
        "unknown archetype must explain the failure on stderr"
    );
}

#[test]
fn ah_type_unknown_archetype_suggests_did_you_mean() {
    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "n"])
        .assert()
        .failure()
        .stderr(contains("Did you mean"))
        .stderr(contains("NR"));

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("Did you mean") && stderr.contains("NR"),
        "unknown archetype must suggest the closest match: {stderr}"
    );
}

#[test]
fn p_type() {
    ah_type_lists_all_archetypes();
    ah_type_known_archetype_prints_full_body();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .args(["type", "SA"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.starts_with("## SA — Stateful API"),
        "type SA must print the canonical SA heading: {stdout}"
    );
}
