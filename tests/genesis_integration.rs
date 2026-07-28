use genesis::{
    envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry},
    feedback::{self, gh::CreateIssueOptions, scratch::ErrorRecord},
    managed_block::{BlockDef, BlockInjector, BlockRegistry},
    suggestions::{CommandRegistry, Suggestion, SuggestionEngine},
};

#[test]
fn genesis_modules_are_resolvable() {
    // ── Envelope ──────────────────────────────────────────────────────
    let _env: Envelope<&str> = Envelope::success(EnvelopeKind::Ok, "hello", vec![], vec![]);
    assert!(_env.ok);
    assert_eq!(_env.envelope_kind, EnvelopeKind::Ok);
    assert_eq!(_env.data, "hello");

    let err = ErrorResult::new(
        "E001",
        "something went wrong",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "just fix".into(),
            description: "run the fix".into(),
        }],
    )
    .unwrap();
    let _err_env = Envelope::error(err, vec![]);
    assert!(!_err_env.ok);

    // ── Suggestions ───────────────────────────────────────────────────
    let mut registry = CommandRegistry::new();
    registry.register(
        "ah",
        vec![
            "check".into(),
            "doctor".into(),
            "init".into(),
            "report".into(),
            "signals".into(),
            "explain".into(),
            "type".into(),
            "archive".into(),
            "scenario".into(),
            "feedback".into(),
        ],
    );
    let engine = SuggestionEngine::new();
    let suggestion = engine.suggest_typo("doctr", &registry);
    assert!(suggestion.is_some(), "should suggest for typo 'doctr'");
    assert_eq!(
        suggestion.unwrap(),
        Suggestion::DidYouMean {
            original: "doctr".to_string(),
            suggestion: "doctor".to_string(),
        }
    );

    // ── Managed block ─────────────────────────────────────────────────
    let mut block_registry = BlockRegistry::new();
    block_registry.register(BlockDef::new("ah"));
    let injector = BlockInjector::new(block_registry);
    assert!(injector.registry().has("ah"));

    // ── Feedback ──────────────────────────────────────────────────────
    let opts = CreateIssueOptions::new("owner/repo", "test bug", "body text");
    assert_eq!(opts.repo, "owner/repo");
    let dry_run_result = feedback::gh::create_issue(&CreateIssueOptions {
        dry_run: true,
        ..opts
    });
    assert!(dry_run_result.is_err());
    assert!(dry_run_result.unwrap_err().contains("DRY RUN"));

    // ── Feedback: scratch ─────────────────────────────────────────────
    let record = ErrorRecord {
        ts: "2026-07-28T00:00:00Z".to_string(),
        argv: vec!["ah".to_string(), "check".to_string()],
        exit: 1,
        footer: Some("→ Run: ah doctor".to_string()),
        kind: "Fix".to_string(),
    };
    assert_eq!(record.exit, 1);
    assert_eq!(record.argv, vec!["ah", "check"]);
}
