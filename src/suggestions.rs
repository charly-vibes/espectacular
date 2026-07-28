use genesis::suggestions::{CommandRegistry, Suggestion, SuggestionEngine};

/// Build the registry of all valid `ah` commands.
pub fn ah_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();
    reg.register(
        "ah",
        vec![
            "check".into(),
            "doctor".into(),
            "init".into(),
            "report".into(),
            "archive".into(),
            "type".into(),
            "explain".into(),
            "upgrade".into(),
            "scenario".into(),
            "signals".into(),
            "completions".into(),
            "feedback".into(),
        ],
    );
    reg
}

/// Suggest a correction for an unknown command using genesis::suggestions.
///
/// Returns `None` if no close match is found.
pub fn suggest(unknown: &str) -> Option<Suggestion> {
    let engine = SuggestionEngine::new();
    let registry = ah_registry();
    engine.suggest_typo(unknown, &registry)
}
