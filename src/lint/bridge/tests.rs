//! Bridge tests: shim-based tests substitute a fake `spk` program so the
//! suite is deterministic without the real binary; the real-spk end-to-end
//! test (lint.rs) skips gracefully when spk is not installed.

use super::is_dual_format;
use super::relay::{relay_dual_format, relay_with};
use crate::lint::walker;
use std::path::PathBuf;

fn spec_with_raw(name: &str, raw: &str) -> walker::SpecFile {
    let mut spec = walker::parse_spec(raw, name);
    spec.source_path = PathBuf::from("/virtual/spec.md");
    spec
}

fn write_shim(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    // Write+rename instead of in-place write: exec'ing a file that was just
    // written in place can race into Text-file-busy (ETXTBSY, os error 26)
    // when the suite runs its many process-spawning tests in parallel —
    // the renamed path is never concurrently open for write at exec time.
    let staged = dir.join(format!("{name}.staged"));
    std::fs::write(&staged, body).unwrap();
    std::fs::rename(&staged, &path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

const VALID_ENVELOPE: &str = r#"#!/bin/sh
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"files_linted":1,"issues":[{"file":"spec.md","message":"intent statement has no imperative","rule_id":"linter.ears_syntax","rule_semantics":"statement must match an EARS pattern"}]},"warnings":[],"hints":[],"meta":{}}'
"#;

const GARBAGE: &str = "#!/bin/sh\necho 'not json at all'\n";

const DUAL_RAW: &str = "---\nid: spec\nkind: intent\nstatement: \"do it\"\n---\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

/// Revision 18 spelling: the id value is the parent-dir-derived real id,
/// not the legacy `spec` placeholder.
const REAL_ID_RAW: &str = "---\nid: auth.tokens\nkind: intent\nstatement: \"do it\"\n---\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

const PLAIN_RAW: &str =
    "# Capability: auth\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

const INTENT_MISMATCH_RAW: &str = "---\nid: wrong\nkind: intent\nstatement: \"do it\"\n---\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

const INTENT_NO_ID_RAW: &str = "---\nkind: intent\nstatement: \"do it\"\n---\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

const GRAPH_ENVELOPE: &str = r#"#!/bin/sh
if [ "$1" = "graph" ]; then
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"nodes":2,"files":1,"edges":[],"dangling":["spec → [[spec.C-b]]"],"violations":[{"edge_kind":"transitions.guard","from":"spec.t-a","to":"spec.P-a","reason":"guard must resolve to an invariant Constraint"}],"fan_in":{},"fan_out":{},"external_boundaries":[],"supersedes_cycles":[]},"warnings":[],"hints":[],"meta":{}}'
else
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"files_linted":1,"issues":[]},"warnings":[],"hints":[],"meta":{}}'
fi
"#;

const GRAPH_CLEAN_ENVELOPE: &str = r#"#!/bin/sh
if [ "$1" = "graph" ]; then
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"nodes":2,"files":1,"edges":[],"dangling":[],"violations":[],"fan_in":{},"fan_out":{},"external_boundaries":[],"supersedes_cycles":[]},"warnings":[],"hints":[],"meta":{}}'
else
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"files_linted":1,"issues":[]},"warnings":[],"hints":[],"meta":{}}'
fi
"#;

// ---- task 5.1 — spk-frontmatter-mismatch closes the bare-continue gap ----

#[test]
fn id_bearing_kind_intent_file_routes_to_spk_even_when_id_value_differs() {
    // An id VALUE that differs from the naming law is spk's
    // linter.id_matches_file to report — espectacular must still ROUTE
    // the file to spk. A nonexistent spk proves routing happened (the
    // old behavior flagged a mismatch WITHOUT any invocation).
    let specs = vec![spec_with_raw("auth", INTENT_MISMATCH_RAW)];
    let mut findings = Vec::new();
    relay_with("/nonexistent/path/spk", &specs, &mut findings);
    assert_eq!(findings.len(), 1, "exactly one routing finding");
    assert_eq!(findings[0].kind, "spk-unavailable");
    assert_eq!(findings[0].severity, crate::lint::Severity::Warning);
    assert_eq!(findings[0].spec_path, "auth");
}

#[test]
fn kind_intent_file_without_id_emits_mismatch_naming_expected_id() {
    let specs = vec![spec_with_raw("auth", INTENT_NO_ID_RAW)];
    let mut findings = Vec::new();
    relay_with("/nonexistent/path/spk", &specs, &mut findings);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, "spk-frontmatter-mismatch");
    let msg = &findings[0].message;
    assert!(msg.contains("no frontmatter id"), "names the gap: {msg}");
}

// ---- Revision 18: any frontmatter id makes a file dual-format ------------

#[test]
fn real_id_frontmatter_is_dual_format() {
    assert!(is_dual_format(REAL_ID_RAW));
    assert!(is_dual_format(DUAL_RAW));
    assert!(!is_dual_format(INTENT_NO_ID_RAW));
    assert!(!is_dual_format(PLAIN_RAW));
}

#[test]
fn plain_and_dual_format_files_emit_no_mismatch() {
    // Plain (no frontmatter) stays untouched; dual-format is relayed and
    // must not double-report a mismatch.
    for raw in [PLAIN_RAW, DUAL_RAW] {
        let specs = vec![spec_with_raw("auth", raw)];
        let mut findings = Vec::new();
        relay_with("/nonexistent/path/spk", &specs, &mut findings);
        assert!(
            findings
                .iter()
                .all(|f| f.kind != "spk-frontmatter-mismatch"),
            "no mismatch finding for plain/dual file: {findings:?}"
        );
    }
}

// ---- task 5.2 — spk graph relay -----------------------------------------

#[test]
fn graph_typing_violation_and_dangling_ref_are_relayed() {
    let dir = tempfile::tempdir().unwrap();
    let shim = write_shim(dir.path(), "spk", GRAPH_ENVELOPE);
    let specs = vec![spec_with_raw("auth", DUAL_RAW)];
    let mut findings = Vec::new();
    relay_with(shim.to_str().unwrap(), &specs, &mut findings);
    let kinds: Vec<&str> = findings.iter().map(|f| f.kind.as_str()).collect();
    assert!(
        kinds.contains(&"spk.graph.typing"),
        "typing violation relayed: kinds {kinds:?}, findings {findings:?}"
    );
    assert!(
        kinds.contains(&"spk.graph.dangling"),
        "dangling ref relayed: {kinds:?}"
    );
    for f in findings.iter().filter(|f| f.kind.starts_with("spk.graph.")) {
        assert_eq!(f.severity, crate::lint::Severity::Warning);
        assert_eq!(f.spec_path, "auth");
    }
    let typing = findings
        .iter()
        .find(|f| f.kind == "spk.graph.typing")
        .unwrap();
    assert!(
        typing.message.contains("spec.t-a"),
        "names the edge: {typing:?}"
    );
    let dangling = findings
        .iter()
        .find(|f| f.kind == "spk.graph.dangling")
        .unwrap();
    assert!(
        dangling.message.contains("spec.C-b"),
        "names the ref: {dangling:?}"
    );
}

#[test]
fn clean_graph_output_emits_no_graph_findings() {
    let dir = tempfile::tempdir().unwrap();
    let shim = write_shim(dir.path(), "spk", GRAPH_CLEAN_ENVELOPE);
    let specs = vec![spec_with_raw("auth", DUAL_RAW)];
    let mut findings = Vec::new();
    relay_with(shim.to_str().unwrap(), &specs, &mut findings);
    assert!(findings.is_empty(), "clean corpus: {findings:?}");
}

#[test]
fn graph_failure_degrades_silently() {
    // Broken shim: the lint relay already surfaces spk-bridge-failure;
    // the graph relay must not add a duplicate noise finding.
    let dir = tempfile::tempdir().unwrap();
    let shim = write_shim(dir.path(), "spk", GARBAGE);
    let specs = vec![spec_with_raw("auth", DUAL_RAW)];
    let mut findings = Vec::new();
    relay_with(shim.to_str().unwrap(), &specs, &mut findings);
    assert!(
        findings.iter().all(|f| !f.kind.starts_with("spk.graph.")),
        "graph errors never surface: {findings:?}"
    );
}

#[test]
fn graph_is_inert_for_plain_openspec_files() {
    let specs = vec![spec_with_raw("auth", PLAIN_RAW)];
    let mut findings = Vec::new();
    relay_with("/nonexistent/path/spk", &specs, &mut findings);
    assert!(findings.is_empty());
}

// ---- real-spk end-to-end (skips gracefully without spk) ------------------

#[test]
fn real_spk_graph_relay_if_installed() {
    if std::process::Command::new("spk")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("spk not installed; skipping");
        return;
    }
    // Typing violation (guard cites a Property) + dangling derives_from.
    let raw = "---\nid: spec\nkind: intent\nstatement: \"WHEN x THE system SHALL y.\"\n---\n\n# Spec\n\n## Constraints\n\n| id | kind | expr | traces_to |\n|----|------|------|-----------|\n| C-a | invariant | foo | [[spec]] |\n\n## Model\n\n### States\n\n- s1\n\n### Transitions\n\n| id | from | to | guard |\n|----|------|----|-------|\n| t-a | s1 | s1 | [[spec.P-a]] |\n\n## Properties\n\n| id | kind | derives_from | generator | predicate |\n|----|------|--------------|-----------|-----------|\n| P-a | unit | [[spec.C-missing]] | input | output |\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.md");
    std::fs::write(&path, raw).unwrap();
    let mut spec = walker::parse_spec(raw, "auth");
    spec.source_path = path.clone();
    let mut findings = Vec::new();
    relay_dual_format(std::slice::from_ref(&spec), &mut findings);
    let kinds: Vec<&str> = findings.iter().map(|f| f.kind.as_str()).collect();
    assert!(
        kinds.contains(&"spk.graph.typing"),
        "typing violation relayed from real spk: {kinds:?}"
    );
    assert!(
        kinds.contains(&"spk.graph.dangling"),
        "dangling ref relayed from real spk: {kinds:?}"
    );
}

#[test]
fn is_dual_format_detects_id_spec_frontmatter() {
    assert!(is_dual_format(DUAL_RAW));
    assert!(is_dual_format(REAL_ID_RAW));
}

#[test]
fn relay_maps_spk_issues_to_spk_prefixed_warning_findings() {
    let dir = tempfile::tempdir().unwrap();
    let shim = write_shim(dir.path(), "spk", VALID_ENVELOPE);
    let specs = vec![spec_with_raw("auth", DUAL_RAW)];
    let mut findings = Vec::new();
    relay_with(shim.to_str().unwrap(), &specs, &mut findings);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.kind, "spk.linter.ears_syntax");
    assert_eq!(f.severity, crate::lint::Severity::Warning);
    assert_eq!(f.spec_path, "auth");
    assert!(f.message.contains("no imperative"));
}

#[test]
fn relay_emits_single_advisory_when_spk_is_missing() {
    let specs = vec![spec_with_raw("auth", DUAL_RAW)];
    let mut findings = Vec::new();
    relay_with("/nonexistent/path/spk", &specs, &mut findings);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, "spk-unavailable");
    assert_eq!(findings[0].severity, crate::lint::Severity::Warning);
}

#[test]
fn relay_emits_advisory_on_unparseable_output() {
    let dir = tempfile::tempdir().unwrap();
    let shim = write_shim(dir.path(), "spk", GARBAGE);
    let specs = vec![spec_with_raw("auth", DUAL_RAW)];
    let mut findings = Vec::new();
    relay_with(shim.to_str().unwrap(), &specs, &mut findings);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, "spk-bridge-failure");
    assert_eq!(findings[0].severity, crate::lint::Severity::Warning);
}

#[test]
fn relay_is_inert_for_plain_openspec_files() {
    // No frontmatter → no spk invocation: even a nonexistent program must
    // produce zero findings (proves the bridge never shelled out).
    let specs = vec![spec_with_raw("auth", PLAIN_RAW)];
    let mut findings = Vec::new();
    relay_with("/nonexistent/path/spk", &specs, &mut findings);
    assert!(findings.is_empty());
}

#[test]
fn is_dual_format_requires_frontmatter_with_an_id_line() {
    // Revision 18: the id VALUE is spk's naming law (parent-dir derived);
    // espectacular only routes frontmattered specs. What is NOT dual-
    // format: no frontmatter at all, frontmatter without an id, or a
    // bare id line outside frontmatter.
    assert!(!is_dual_format(PLAIN_RAW));
    assert!(!is_dual_format(INTENT_NO_ID_RAW));
    assert!(!is_dual_format("id: spec\n"));
    assert!(!is_dual_format("no frontmatter\n---\nid: spec\n"));
}

// ---- property-derived contract bindings (DDL pilot, design D6) ----------

#[test]
fn p_bridge() {
    id_bearing_kind_intent_file_routes_to_spk_even_when_id_value_differs();
    graph_typing_violation_and_dangling_ref_are_relayed();
    relay_maps_spk_issues_to_spk_prefixed_warning_findings();
    relay_emits_single_advisory_when_spk_is_missing();
    relay_emits_advisory_on_unparseable_output();
    relay_is_inert_for_plain_openspec_files();
}
