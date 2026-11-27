//! `ah feedback` command handler: files an issue against the upstream
//! repo via gh (dry-run by default — preview before filing).

use genesis::feedback;
use std::path::PathBuf;

/// Handle `ah feedback`.
pub fn run_feedback_command(
    kind: String,
    dry_run: bool,
    from_last_error: bool,
) -> anyhow::Result<()> {
    let valid_kinds = ["bug", "feature", "chore"];
    if !valid_kinds.contains(&kind.as_str()) {
        print_kind_suggestion(&kind, &valid_kinds);
    }

    if !dry_run {
        eprintln!("Warning: this will file a real issue. Use --dry-run to preview first.");
    }

    let body = build_issue_body(from_last_error)?;
    let repo = std::env::current_dir()?;
    let target_repo = target_repo(&repo);
    create_feedback_issue(&kind, body, target_repo, dry_run)
}

/// File the issue (or preview it under --dry-run) and print the outcome.
fn create_feedback_issue(
    kind: &str,
    body: String,
    target_repo: String,
    dry_run: bool,
) -> anyhow::Result<()> {
    let opts = feedback::gh::CreateIssueOptions {
        repo: target_repo,
        title: format!("[ah] {kind}: auto-reported"),
        body: body.clone(),
        labels: labels_for(kind),
        dry_run,
    };

    if dry_run {
        eprintln!("{}", body);
    }

    match feedback::gh::create_issue(&opts) {
        Ok(result) => print_issue_result(&result),
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
    }
    Ok(())
}

/// Target repo: the `repository` field of the invoking project's
/// Cargo.toml, falling back to this tool's home repo.
///
/// Missing Cargo.toml = not a Rust workspace (espectacular-qyu, GH#28
/// item 3): degrade to the default target repo instead of erroring, so
/// non-Rust projects can still file feedback.
fn target_repo(repo: &std::path::Path) -> String {
    std::fs::read_to_string(repo.join("Cargo.toml"))
        .ok()
        .and_then(|manifest| extract_repository(&manifest))
        .unwrap_or_else(|| "charly-vibes/espectacular".to_string())
}

fn labels_for(kind: &str) -> Vec<String> {
    match kind {
        "bug" => vec!["agent-reported".into(), "bug".into(), "has-repro".into()],
        "feature" => vec!["agent-reported".into(), "enhancement".into()],
        "chore" => vec!["agent-reported".into(), "chore".into()],
        _ => vec!["agent-reported".into()],
    }
}

fn print_kind_suggestion(kind: &str, valid_kinds: &[&str]) -> ! {
    let mut reg = genesis::suggestions::CommandRegistry::new();
    reg.register("kind", valid_kinds.iter().map(|k| k.to_string()).collect());
    let engine = genesis::suggestions::SuggestionEngine::new();
    if let Some(suggestion) = engine.suggest_typo(kind, &reg) {
        eprintln!("{}", suggestion.message());
    } else {
        eprintln!(
            "unknown kind: {kind}. Valid kinds: {}",
            valid_kinds.join(", ")
        );
    }
    std::process::exit(2);
}

fn print_issue_result(result: &feedback::gh::GhResult) {
    match result {
        feedback::gh::GhResult::Created { url, number } => {
            println!("Created issue #{}: {}", number, url);
        }
        feedback::gh::GhResult::FallbackUrl(url) => {
            println!("Open this URL to file the issue: {}", url);
        }
        feedback::gh::GhResult::LocalFile(path) => {
            println!("Report written to: {}", path.display());
        }
    }
}

/// Build the issue body: last-error record (when requested), gathered
/// environment context, then redaction of sensitive info.
fn build_issue_body(from_last_error: bool) -> anyhow::Result<String> {
    let mut body_parts: Vec<String> = Vec::new();

    if from_last_error {
        body_parts.push(last_error_section()?);
    }

    // Gather environment context
    let cwd = std::env::current_dir().unwrap_or_default();
    let bundle =
        feedback::context::gather_context("ah", env!("CARGO_PKG_VERSION"), None, None, None, &cwd);
    body_parts.push(feedback::context::format_context_bundle(&bundle));

    // Redact sensitive info
    let home = std::env::var("HOME").ok().map(PathBuf::from);
    let git_remote = bundle.git_remote.clone();
    let body = feedback::redactor::redact(
        &body_parts.join("\n\n"),
        home.as_deref(),
        git_remote.as_deref(),
    );
    Ok(body)
}

fn last_error_section() -> anyhow::Result<String> {
    if let Some(record) = feedback::scratch::read_last_error("ah") {
        let mut section = format!(
            "## Error\n\n**Command:** `{}`\n**Exit code:** {}\n**Kind:** {}",
            record.argv.join(" "),
            record.exit,
            record.kind,
        );
        if let Some(ref footer) = record.footer {
            section.push_str(&format!("**Suggestion:** {footer}"));
        }
        Ok(section)
    } else {
        eprintln!("No recent error recorded. Run `ah check` or `ah doctor` first.");
        std::process::exit(1);
    }
}

/// Extract the `repository` field from a Cargo.toml manifest string,
/// reduced to `owner/repo` via `genesis::git::remote_key`.
fn extract_repository(manifest: &str) -> Option<String> {
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("repository = ") {
            let clean = val.trim_matches('"').trim_matches('\'');
            // `remote_key` keeps the host (`github.com/owner/repo`); the
            // repo's call sites expect `owner/repo`, so drop the known
            // github host here rather than reimplementing the reduction.
            let key = genesis::git::remote_key(clean);
            return Some(key.strip_prefix("github.com/").unwrap_or(&key).to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::extract_repository;

    /// Fixture set: each expected value is `genesis::git::remote_key`'s
    /// output with the known `github.com/` host dropped (the same
    /// mapping `extract_repository` applies at the call site). Covers
    /// plain https, scp/ssh, PAT-authenticated https, trailing `.git`,
    /// and a non-github URL.
    #[test]
    fn extract_repository_matches_remote_key_on_fixtures() {
        let fixtures: &[(&str, &str)] = &[
            ("https://github.com/owner/repo", "owner/repo"),
            ("https://github.com/owner/repo.git", "owner/repo"),
            ("git@github.com:owner/repo.git", "owner/repo"),
            ("https://pat@github.com/owner/repo.git", "owner/repo"),
            ("https://gitlab.com/owner/repo.git", "gitlab.com/owner/repo"),
        ];
        for (url, expected) in fixtures {
            let manifest = format!("repository = \"{url}\"\n");
            assert_eq!(
                Some(expected.to_string()),
                extract_repository(&manifest),
                "fixture {url}"
            );
        }
    }

    #[test]
    fn extract_repository_missing_field_is_none() {
        assert_eq!(None, extract_repository("[package]\nname = \"x\"\n"));
    }
}
