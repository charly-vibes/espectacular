//! Check registry modules. Shape checks (espectacular-aar) and future check
//! families append their submodule here additively. Shared heuristic helpers
//! (lowercase token matching) live here too, one definition per semantics.

pub mod flow;
pub mod shape;

/// Lowercase tokenization shared by line-level matchers: keeps hyphenated
/// terms (`user-friendly`) intact and strips punctuation (`fast.` → `fast`).
pub(crate) fn tokens(line: &str) -> impl Iterator<Item = String> {
    line.split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect::<Vec<_>>()
        .into_iter()
}

/// Term containment on a line: multi-word / hyphenated markers match by
/// lowercase substring; single words match whole tokens only, so
/// "redirect" ≠ "red".
pub(crate) fn line_contains_term(line: &str, term: &str) -> bool {
    let lower = line.to_lowercase();
    if term.contains('-') || term.contains(' ') {
        return lower.contains(term);
    }
    tokens(line).any(|t| t == term)
}
