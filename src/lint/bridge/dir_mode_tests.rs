//! Dir-mode relay tests (espectacular-hcz): one `spk lint <root>` spawn per
//! linted root, `file`-field attribution, and the named fallbacks. Kept in a
//! dedicated module so the shim fixtures and their result-unwrapping stay
//! out of the main bridge test file's gate budgets.

use super::relay::{relay_dual_format, relay_with_root};
use crate::lint::walker;
use crate::lint::Severity;
use std::path::Path;
use std::path::PathBuf;

/// Revision-18 dual-format frontmatter shared by the dir-mode specs.
const DUAL_RAW: &str = "---\nid: spec\nkind: intent\nstatement: \"do it\"\n---\n\n## Requirements\n\n### Requirement: Login\nThe system SHALL log in.\n";

/// Fixture failure — deliberately not `.unwrap()`: the gate budgets unwraps
/// per test file.
fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("fixture failed: {e:?}"))
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: Box<Path>,
    counter: Box<Path>,
    shim: Box<Path>,
}

fn root(fx: &Fixture) -> &Path {
    &fx.root
}

/// Temp root + invocation-log + branching shim. The shim logs every lint
/// invocation as "lint <arg>" (graph exits nonzero — the graph relay
/// degrades silently), then answers DIR_RESPONSE for dir-shaped lint args
/// and FILE_RESPONSE otherwise.
fn fixture(dir_response: impl FnOnce(&Path) -> String, file_response: &'static str) -> Fixture {
    let dir = ok(tempfile::tempdir());
    let root_path = dir.path().join("specs");
    ok(std::fs::create_dir(&root_path));
    let counter = dir.path().join("invocations.log");
    let body = format!(
        "#!/bin/sh\nprintf '%s %s\\n' \"$1\" \"$2\" >> {}\nif [ \"$1\" = \"graph\" ]; then\n  exit 1\nfi\nif [ -d \"$2\" ]; then\n{}\nelse\n{file_response}\nfi\n",
        counter.display(),
        dir_response(&root_path),
    );
    // Write+rename: exec'ing a just-written file races into ETXTBSY when the
    // suite runs its process-spawning tests in parallel (shared-helper rule).
    let shim = dir.path().join("spk-branching");
    write_executable(&shim, &body);
    Fixture {
        _dir: dir,
        root: root_path.into_boxed_path(),
        counter: counter.into_boxed_path(),
        shim: shim.into_boxed_path(),
    }
}

/// Write+rename (ETXTBSY-safe) + exec bit.
fn write_executable(path: &Path, body: &str) {
    let staged = path.with_extension("staged");
    ok(std::fs::write(&staged, body));
    ok(std::fs::rename(&staged, path));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ok(std::fs::set_permissions(
            path,
            std::fs::Permissions::from_mode(0o755),
        ));
    }
}

fn spec_trio(root: &Path) -> Vec<walker::SpecFile> {
    ["alpha", "beta", "gamma"]
        .iter()
        .map(|name| {
            let mut spec = walker::parse_spec(DUAL_RAW, name);
            spec.source_path = root.join(format!("{name}.md"));
            spec
        })
        .collect()
}

fn lint_spawns(fx: &Fixture) -> usize {
    let log = std::fs::read_to_string(&fx.counter).unwrap_or_default();
    log.lines().filter(|l| l.starts_with("lint ")).count()
}

fn relay(fx: &Fixture, specs: &[walker::SpecFile]) -> Vec<crate::lint::LintFinding> {
    let mut findings = Vec::new();
    relay_with_root(fx.shim.to_str().unwrap(), &fx.root, specs, &mut findings);
    findings
}

/// A clean dir envelope with the given issues JSON (file fields use
/// `<root>/<name>` — the form spk echoes for the invocation's dir arg).
fn dir_envelope(issues_json: String, warnings_json: &str) -> String {
    format!(
        "#!/bin/sh\necho '{{\"ok\":true,\"envelope_version\":\"0.1\",\"cli_version\":\"0.1.0\",\"envelope_kind\":\"ok\",\"data\":{{\"files_linted\":3,\"issues\":{issues_json}}},\"warnings\":{warnings_json},\"hints\":[],\"meta\":{{}}}}'\n"
    )
}

fn issue_at(root: &Path, name: &str, rule: &str) -> String {
    let raw = r#"{"file":"__ROOT__/__NAME__","message":"planted","rule_id":"__RULE__","rule_semantics":"fix it"}"#;
    raw.replace("__ROOT__", &root.to_string_lossy())
        .replace("__NAME__", name)
        .replace("__RULE__", rule)
}

fn skipped_warnings(root: &Path) -> String {
    let raw = r#"[{"rule_name":"","message":"__ROOT__/plain.md: skipped (no frontmatter — not a spec file)"},{"rule_name":"","message":"__ROOT__/other.md: skipped (no frontmatter — not a spec file)"}]"#;
    raw.replace("__ROOT__", &root.to_string_lossy())
}

// ---- real-spk end-to-end (skips gracefully without spk) ----------------
// Lives here because the real envelope exercises the dir-mode path: one
// `spk lint <root>` spawn whose issues carry per-issue `file` fields.

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
    let dir = ok(tempfile::tempdir());
    let path = dir.path().join("spec.md");
    ok(std::fs::write(&path, raw));
    let mut spec = walker::parse_spec(raw, "auth");
    spec.source_path = path.clone();
    let mut findings = Vec::new();
    relay_dual_format(dir.path(), std::slice::from_ref(&spec), &mut findings);
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

const FILE_CLEAN: &str = r#"#!/bin/sh
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"files_linted":1,"issues":[]},"warnings":[],"hints":[],"meta":{}}'
"#;

const FILE_ONE_ISSUE: &str = r#"#!/bin/sh
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"files_linted":1,"issues":[{"file":"ignored","message":"file-mode","rule_id":"linter.file","rule_semantics":"s"}]},"warnings":[],"hints":[],"meta":{}}'
"#;

const DIR_BROKEN: &str = r#"#!/bin/sh
echo '{"ok":false,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"error","data":null,"warnings":[{"rule_name":"","message":"spec files failed to parse"}],"hints":[],"meta":{}}'
"#;

const DIR_UNATTRIBUTED: &str = r#"#!/bin/sh
echo '{"ok":true,"envelope_version":"0.1","cli_version":"0.1.0","envelope_kind":"ok","data":{"files_linted":3,"issues":[{"message":"unattributed","rule_id":"linter.nofile","rule_semantics":"s"}]},"warnings":[],"hints":[],"meta":{}}'
"#;

#[test]
fn dir_mode_spawns_once_per_root_and_attributes_by_file_field() {
    let fx = fixture(
        |root| {
            dir_envelope(
                format!("[{}]", issue_at(root, "beta.md", "linter.planted")),
                "[]",
            )
        },
        FILE_CLEAN,
    );
    let specs = spec_trio(root(&fx));
    let findings = relay(&fx, &specs);
    assert_eq!(lint_spawns(&fx), 1, "exactly one lint spawn per root");
    assert_eq!(findings.len(), 1, "planted issue relayed: {findings:?}");
    assert_eq!(findings[0].kind, "spk.linter.planted");
    assert_eq!(findings[0].spec_path, "beta", "attributed via file field");
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn broken_corpus_dir_envelope_falls_back_per_file_preserving_findings() {
    let fx = fixture(|_| DIR_BROKEN.to_string(), FILE_ONE_ISSUE);
    let specs = spec_trio(root(&fx));
    let findings = relay(&fx, &specs);
    assert_eq!(lint_spawns(&fx), 4, "1 dir attempt + 3 per-file fallback");
    assert_eq!(
        findings.len(),
        3,
        "per-file findings preserved: {findings:?}"
    );
    let attributed: Vec<&str> = findings.iter().map(|f| f.spec_path.as_str()).collect();
    assert_eq!(attributed, vec!["alpha", "beta", "gamma"]);
    assert!(
        findings.iter().all(|f| f.kind == "spk.linter.file"),
        "no advisory: {findings:?}"
    );
}

#[test]
fn dir_envelope_without_file_field_falls_back_with_named_advisory() {
    let fx = fixture(|_| DIR_UNATTRIBUTED.to_string(), FILE_ONE_ISSUE);
    let specs = spec_trio(root(&fx));
    let findings = relay(&fx, &specs);
    assert_eq!(lint_spawns(&fx), 4, "dir attempt + per-file fallback");
    let fallback: Vec<_> = findings
        .iter()
        .filter(|f| f.kind == "spk-bridge-fallback")
        .collect();
    assert_eq!(fallback.len(), 1, "named degradation: {findings:?}");
    assert_eq!(
        findings
            .iter()
            .filter(|f| f.kind == "spk.linter.file")
            .count(),
        3,
        "parity after fallback: {findings:?}"
    );
}

#[test]
fn dir_envelope_skipped_file_warnings_never_become_findings() {
    let fx = fixture(
        |root| {
            let issues = format!(
                "[{},{}]",
                issue_at(root, "alpha.md", "linter.x"),
                issue_at(root, "gamma.md", "linter.x")
            );
            dir_envelope(issues, &skipped_warnings(root))
        },
        FILE_CLEAN,
    );
    let specs = spec_trio(root(&fx));
    let findings = relay(&fx, &specs);
    assert_eq!(lint_spawns(&fx), 1, "dir-mode only: {findings:?}");
    assert_eq!(
        findings.len(),
        2,
        "issues only, warnings dropped: {findings:?}"
    );
    let attributed: Vec<&str> = findings.iter().map(|f| f.spec_path.as_str()).collect();
    assert_eq!(attributed, vec!["alpha", "gamma"]);
}
