use assert_cmd::Command;
use std::fs;

fn create_simple_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("openspec/specs")).unwrap();
    fs::create_dir_all(root.join("openspec/changes")).unwrap();
    fs::create_dir_all(root.join(".espectacular")).unwrap();
    fs::write(
        root.join(".espectacular/config.toml"),
        format!(
            "tool_version = \"{}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    // Write an AGENTS.md with the managed block so doctor doesn't complain
    fs::write(
        root.join("AGENTS.md"),
        "# Project\n\n<!-- ah:managed:start -->\n<!-- ah:managed:end -->\n",
    )
    .unwrap();
    dir
}

#[test]
fn ah_check_json_emits_shared_envelope() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "check"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    // Shared envelope shape
    assert!(json.get("ok").is_some(), "must have 'ok'");
    assert!(
        json.get("envelope_version").is_some(),
        "must have 'envelope_version'"
    );
    assert!(json.get("cli_version").is_some(), "must have 'cli_version'");
    assert!(
        json.get("envelope_kind").is_some(),
        "must have 'envelope_kind'"
    );
    assert!(json.get("data").is_some(), "must have 'data'");
    assert!(json.get("warnings").is_some(), "must have 'warnings'");
    assert!(json.get("hints").is_some(), "must have 'hints'");
    assert!(json.get("meta").is_some(), "must have 'meta'");

    // Existing findings/summary nested under data
    let data = json.get("data").unwrap();
    assert!(data.get("findings").is_some(), "data must have 'findings'");
    assert!(data.get("summary").is_some(), "data must have 'summary'");
}

#[test]
fn ah_doctor_json_emits_shared_envelope() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "doctor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert!(json.get("ok").is_some(), "must have 'ok'");
    assert!(
        json.get("envelope_version").is_some(),
        "must have 'envelope_version'"
    );
    assert!(
        json.get("envelope_kind").is_some(),
        "must have 'envelope_kind'"
    );
    assert!(json.get("data").is_some(), "must have 'data'");
}

#[test]
fn ah_report_json_emits_shared_envelope() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "report"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert!(json.get("ok").is_some(), "must have 'ok'");
    assert!(
        json.get("envelope_version").is_some(),
        "must have 'envelope_version'"
    );
    assert!(
        json.get("envelope_kind").is_some(),
        "must have 'envelope_kind'"
    );
    assert!(json.get("data").is_some(), "must have 'data'");
}

#[test]
fn ah_signals_json_emits_shared_envelope() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "signals"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert!(json.get("ok").is_some(), "must have 'ok'");
    assert!(
        json.get("envelope_version").is_some(),
        "must have 'envelope_version'"
    );
    assert!(
        json.get("envelope_kind").is_some(),
        "must have 'envelope_kind'"
    );
    assert!(json.get("data").is_some(), "must have 'data'");
}

/// A check run WITH blocking findings must not lie to machine consumers:
/// the envelope's `ok` must agree with the process exit code. Regression
/// guard for the ok:true+exit-1 drift (evallerina-54f, round2-audit:
/// 8/12 native ah_check toolResults exited 1 with an ok:true envelope
/// passthrough) — evallerina's deterministic checks classify an
/// invocation by (ok, exit_code) and this pair satisfies neither an
/// ok-envelope nor an error-envelope contract.
#[test]
fn ah_check_json_blocking_findings_reports_ok_false() {
    let dir = create_simple_repo();
    // A contract whose scenario name matches no deployed spec scenario
    // produces a blocking structural finding (orphan contract).
    fs::create_dir_all(dir.path().join(".espectacular/contracts")).unwrap();
    fs::write(
        dir.path().join(".espectacular/contracts/orphan.toml"),
        "version = 1\n\n[[scenario]]\nname = \"No such spec scenario\"\nkind = \"unit\"\ncommand = \"true\"\n",
    )
    .unwrap();

    let assert = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "check"])
        .assert()
        .failure();

    let output = assert.get_output().stdout.clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json["ok"], false,
        "blocking findings must set ok:false to agree with the failing exit code"
    );
    let has_blocking = json["data"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["category"] == "structural" || f["category"] == "execution");
    assert!(
        has_blocking,
        "fixture must actually produce blocking findings"
    );
}

/// Positive control: a clean check run keeps ok:true and exits 0 — the
/// fix must not touch the happy path.
#[test]
fn ah_check_json_clean_run_keeps_ok_true() {
    let dir = create_simple_repo();
    let output = Command::cargo_bin("ah")
        .unwrap()
        .current_dir(dir.path())
        .args(["--json", "check"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["ok"], true, "clean run keeps ok:true");
}
