mod adapters;
mod archetypes;
mod archive;
mod check;
mod config;
mod contracts;
mod doctor;
mod explain;
mod fsutil;
mod init;
mod openspec;
mod quality;
mod report;
mod runner;
mod scenario;
mod signals;
mod upgrade;

use anyhow::Context;
use clap::{CommandFactory, Parser, Subcommand};
use genesis::cli::{generate_completions, maybe_print_version_json};
use genesis::envelope::{Envelope, EnvelopeKind};
use genesis::guide::Guide;
use std::fs;
use std::io::Write;

/// Wrap any serializable data in a shared genesis envelope.
fn to_json_envelope<T: serde::Serialize>(kind: EnvelopeKind, data: T) -> String {
    let env: Envelope<T> = Envelope::success(env!("CARGO_PKG_VERSION"), kind, data, vec![], vec![]);
    serde_json::to_string(&env).expect("envelope serialization")
}

/// Extract the `repository` field from a Cargo.toml manifest string.
fn extract_repository(manifest: &str) -> Option<String> {
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("repository = ") {
            // Strip quotes
            let clean = val.trim_matches('"').trim_matches('\'');
            // Reduce to owner/repo format
            let clean = clean
                .strip_prefix("https://github.com/")
                .or_else(|| clean.strip_prefix("git@github.com:"))
                .unwrap_or(clean);
            // Strip trailing .git and slashes
            let clean = clean
                .trim_end_matches(".git")
                .trim_end_matches('/')
                .to_string();
            return Some(clean);
        }
    }
    None
}

#[derive(Parser)]
#[command(name = "ah", version)]
struct Cli {
    /// Output machine-readable JSON
    #[arg(short = 'j', long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check {
        #[arg(long = "changes")]
        changes: Vec<String>,
        /// Run contract tests (shell, custom, property, snapshot entries).
        /// By default, ah check only performs fast structural analysis
        /// (spec/contract correspondence).
        #[arg(long = "run-tests")]
        run_tests: bool,
    },
    Doctor {
        #[arg(long)]
        enable: Option<String>,
    },
    Init,
    Report {},
    Archive {
        change: String,
    },
    Type {
        name: Option<String>,
    },
    Explain {
        topic: Option<String>,
        #[arg(long)]
        list: bool,
    },
    Upgrade,
    Scenario {
        #[command(subcommand)]
        command: ScenarioCommand,
    },
    /// Read dont rejection events and emit drift signals as JSON.
    Signals,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell, elvish)
        shell: clap_complete::Shell,
    },
    /// File an issue against the upstream repo via gh
    Feedback {
        kind: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        from_last_error: bool,
    },
}

#[derive(Subcommand)]
enum ScenarioCommand {
    New {
        change: String,
        spec: String,
        #[arg(long)]
        requirement: String,
        heading: String,
    },
    Supersede {
        spec: String,
        old_id: String,
        #[arg(long = "with")]
        with: String,
        #[arg(long = "in-change")]
        in_change: String,
    },
}

/// Commands the `ah` CLI accepts — sourced from `Guide::builder` so the
/// guide owns command registration for typo detection.
const AH_COMMANDS: &[&str] = &[
    "check",
    "doctor",
    "init",
    "report",
    "archive",
    "type",
    "explain",
    "upgrade",
    "scenario",
    "signals",
    "completions",
    "feedback",
];

/// Assemble the genesis `Guide` scaffold for `ah`.
///
/// The guide owns command registration (for typo detection), the
/// `ErrorSink` used for self-healing error output, and marks adoption of
/// `genesis::config` via `.config::<Config>()`.
///
/// Note: `.config::<Config>()` is a forward-compatible intent marker —
/// in genesis v0.2.0 it only sets an (unused) `has_config` flag and has
/// no runtime effect. It's kept so a future `Guide` that drives config
/// wiring picks espectacular up automatically.
fn build_guide() -> Guide {
    Guide::builder("ah", env!("CARGO_PKG_VERSION"))
        .about("Behavioral verification layer enforcing spec-test correspondence")
        .commands(AH_COMMANDS)
        .config::<config::Config>()
        .build()
}

/// Wrapper that renders an anyhow error with its full context chain (`:#`)
/// as its `Display` output, so `ErrorSink::handle` prints the rich message
/// while genesis owns the scratch + footer mechanics.
struct FormattedError(String);

impl std::fmt::Display for FormattedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for FormattedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl std::error::Error for FormattedError {}

fn main() {
    // Delegate --version --json handling to genesis::cli
    if maybe_print_version_json("ah", env!("CARGO_PKG_VERSION")) {
        return;
    }

    let guide = build_guide();

    match Cli::try_parse() {
        Ok(cli) => {
            if let Err(error) = run(cli) {
                // ErrorSink owns the error message, the self-healing footer,
                // and the best-effort scratch write for `--from-last-error`.
                // We suppress the generic "-> Run: ah doctor" footer
                // (`with_suggest(false)`) because we provide the feedback
                // footer instead (`with_feedback`); enabling suggestions
                // would print a duplicate footer.
                let sink = guide
                    .error_sink()
                    .with_suggest(false)
                    .with_feedback(Some("feedback"));
                let mut stderr = std::io::stderr();
                sink.handle(&FormattedError(format!("{error:#}")), &mut stderr);
                // Exit 1 matches the `exit` field ErrorSink writes into the
                // scratch record, so `feedback --from-last-error` reports a
                // truthful exit code. (The typo-subcommand path below keeps
                // exit 2 to match clap's invalid-subcommand convention.)
                std::process::exit(1);
            }
        }
        Err(err) => {
            if err.kind() == clap::error::ErrorKind::InvalidSubcommand {
                if let Some(clap::error::ContextValue::String(bad_cmd)) =
                    err.get(clap::error::ContextKind::InvalidSubcommand)
                {
                    let engine = genesis::suggestions::SuggestionEngine::new();
                    if let Some(suggestion) = engine.suggest_typo(bad_cmd, guide.registry()) {
                        eprintln!("{}", suggestion.message());
                        std::process::exit(2);
                    }
                }
            }
            err.exit();
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Check { changes, run_tests } => {
            let report = check::run_check(&std::env::current_dir()?, &changes, run_tests)?;
            if cli.json {
                let has_blocking = report
                    .findings
                    .iter()
                    .any(|f| f.category == "structural" || f.category == "execution");
                println!("{}", to_json_envelope(EnvelopeKind::Check, &report));
                std::io::stdout().flush().unwrap_or_default();
                std::process::exit(if has_blocking { 1 } else { 0 });
            } else {
                print_check_report(&report, run_tests);
            }
            let has_blocking = report
                .findings
                .iter()
                .any(|f| f.category == "structural" || f.category == "execution");
            std::io::stdout().flush().unwrap_or_default();
            std::process::exit(if has_blocking { 1 } else { 0 });
        }
        Command::Doctor { enable } => {
            if cli.json {
                let report = doctor::run_doctor(&std::env::current_dir()?)?;
                println!("{}", doctor::doctor_to_envelope(&report));
                return Ok(());
            }
            if let Some(capability) = enable {
                match doctor::run_doctor_enable(&std::env::current_dir()?, &capability)? {
                    doctor::DoctorEnableResult::Written { path, table_name } => {
                        println!("enabled: {table_name} in {path}");
                    }
                    doctor::DoctorEnableResult::AlreadyEnabled => {
                        println!("already-enabled: {capability}");
                    }
                }
                return Ok(());
            }
            let report = doctor::run_doctor(&std::env::current_dir()?)?;
            for det in &report.detections {
                println!(
                    "framework: {} ({})",
                    det.name,
                    crate::adapters::detection_source_label(det.detection_source)
                );
            }
            for rec in &report.suggestions {
                println!(
                    "recommendation: {} — run: {}",
                    rec.detail, rec.apply_command
                );
            }
            if report.genesis_report.is_healthy() {
                println!("healthy: all checks passed");
            } else {
                for entry in report
                    .genesis_report
                    .checks
                    .iter()
                    .filter(|c| !c.status.is_pass())
                {
                    eprintln!("{}: {}", entry.name, entry.message);
                }
                std::process::exit(1);
            }
            Ok(())
        }
        Command::Report {} => {
            let report = report::run_report(&std::env::current_dir()?)?;
            if cli.json {
                let has_gaps = report.summary.missing > 0 || report.summary.failing > 0;
                println!("{}", to_json_envelope(EnvelopeKind::Ok, &report));
                std::process::exit(if has_gaps { 1 } else { 0 });
            } else {
                println!(
                    "{:<20} {:<10} {:>8} {:>8} {:>8} {:>8}",
                    "spec", "archetype", "covered", "missing", "failing", "total"
                );
                for row in &report.matrix {
                    println!(
                        "{:<20} {:<10} {:>8} {:>8} {:>8} {:>8}",
                        row.spec, row.archetype, row.covered, row.missing, row.failing, row.total
                    );
                }
                println!();
                println!(
                    "covered: {} | missing: {} | failing: {} | total: {}",
                    report.summary.covered,
                    report.summary.missing,
                    report.summary.failing,
                    report.summary.total_scenarios
                );
            }
            let has_gaps = report.summary.missing > 0 || report.summary.failing > 0;
            std::process::exit(if has_gaps { 1 } else { 0 });
        }
        Command::Init => {
            let result = init::run_init(&std::env::current_dir()?)?;
            for path in &result.created {
                println!("created: {path}");
            }
            for path in &result.refreshed {
                println!("refreshed: {path}");
            }
            for contract in &result.stubbed_contracts {
                println!("stubbed: {contract}");
            }
            for concern in &result.concerns {
                eprintln!("concern: {concern}");
            }
            Ok(())
        }
        Command::Archive { change } => {
            let result = archive::run_archive(&std::env::current_dir()?, &change)?;
            for item in &result.moved {
                println!("archived: {item}");
            }
            Ok(())
        }
        Command::Type { name } => {
            match name.as_deref() {
                None => {
                    println!("{}", archetypes::list_archetypes());
                }
                Some(code) => {
                    let upper = code.to_uppercase();
                    match archetypes::lookup(&upper) {
                        Some(a) => println!("{}", a.body),
                        None => {
                            let suggestions = archetypes::did_you_mean(code);
                            if suggestions.is_empty() {
                                eprintln!(
                                    "unknown archetype: {code}. Known: {}",
                                    archetypes::known_codes().join(", ")
                                );
                            } else {
                                eprintln!(
                                    "unknown archetype: {code}. Did you mean: {}?",
                                    suggestions.join(", ")
                                );
                            }
                            std::process::exit(1);
                        }
                    }
                }
            }
            Ok(())
        }
        Command::Explain { topic, list } => explain::run_explain(topic.as_deref(), list, cli.json),
        Command::Signals => {
            let project_root = std::env::current_dir()?;
            let drift = signals::collect_drift_signals(&project_root);
            println!("{}", to_json_envelope(EnvelopeKind::Stats, &drift));
            Ok(())
        }
        Command::Feedback {
            kind,
            dry_run,
            from_last_error,
        } => {
            // Validate kind with suggestions
            let valid_kinds = ["bug", "feature", "chore"];
            if !valid_kinds.contains(&kind.as_str()) {
                // Use genesis suggestions to find close matches
                let mut reg = genesis::suggestions::CommandRegistry::new();
                reg.register("kind", valid_kinds.iter().map(|k| k.to_string()).collect());
                let engine = genesis::suggestions::SuggestionEngine::new();
                if let Some(suggestion) = engine.suggest_typo(&kind, &reg) {
                    eprintln!("{}", suggestion.message());
                } else {
                    eprintln!(
                        "unknown kind: {kind}. Valid kinds: {}",
                        valid_kinds.join(", ")
                    );
                }
                std::process::exit(2);
            }

            if !dry_run {
                eprintln!("Warning: this will file a real issue. Use --dry-run to preview first.");
            }

            let repo = std::env::current_dir()?;
            let manifest_path = repo.join("Cargo.toml");
            let manifest = fs::read_to_string(&manifest_path).context("cannot read Cargo.toml")?;
            let target_repo = extract_repository(&manifest)
                .unwrap_or_else(|| "charly-vibes/espectacular".to_string());

            // Build issue body
            let mut body_parts: Vec<String> = Vec::new();

            if from_last_error {
                if let Some(record) = genesis::feedback::scratch::read_last_error("ah") {
                    body_parts.push(format!(
                        "## Error\n\n**Command:** `{}`\n**Exit code:** {}\n**Kind:** {}",
                        record.argv.join(" "),
                        record.exit,
                        record.kind,
                    ));
                    if let Some(ref footer) = record.footer {
                        body_parts.push(format!("**Suggestion:** {}", footer));
                    }
                } else {
                    eprintln!("No recent error recorded. Run `ah check` or `ah doctor` first.");
                    std::process::exit(1);
                }
            }

            // Gather environment context
            let cwd = std::env::current_dir().unwrap_or_default();
            let bundle = genesis::feedback::context::gather_context(
                "ah",
                env!("CARGO_PKG_VERSION"),
                None,
                None,
                None,
                &cwd,
            );
            body_parts.push(genesis::feedback::context::format_context_bundle(&bundle));

            // Redact sensitive info
            let home = std::env::var("HOME").ok().map(std::path::PathBuf::from);
            let git_remote = bundle.git_remote.clone();
            let body = genesis::feedback::redactor::redact(
                &body_parts.join("\n\n"),
                home.as_deref(),
                git_remote.as_deref(),
            );

            let labels = match kind.as_str() {
                "bug" => vec!["agent-reported".into(), "bug".into(), "has-repro".into()],
                "feature" => vec!["agent-reported".into(), "enhancement".into()],
                "chore" => vec!["agent-reported".into(), "chore".into()],
                _ => vec!["agent-reported".into()],
            };

            let opts = genesis::feedback::gh::CreateIssueOptions {
                repo: target_repo,
                title: format!("[ah] {}: auto-reported", kind),
                body: body.clone(),
                labels,
                dry_run,
            };

            if dry_run {
                eprintln!("{}", body);
            }

            match genesis::feedback::gh::create_issue(&opts) {
                Ok(result) => match result {
                    genesis::feedback::gh::GhResult::Created { url, number } => {
                        println!("Created issue #{}: {}", number, url);
                    }
                    genesis::feedback::gh::GhResult::FallbackUrl(url) => {
                        println!("Open this URL to file the issue: {}", url);
                    }
                    genesis::feedback::gh::GhResult::LocalFile(path) => {
                        println!("Report written to: {}", path.display());
                    }
                },
                Err(msg) => {
                    eprintln!("{}", msg);
                    std::process::exit(1);
                }
            }
            Ok(())
        }
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            generate_completions(&mut cmd, shell).map_err(|e| anyhow::anyhow!("{}", e))
        }
        Command::Upgrade => {
            let report = upgrade::run_upgrade(&std::env::current_dir()?)?;
            if report.drift {
                println!(
                    "upgraded: tool_version {} → {}",
                    report.config_version, report.binary_version
                );
                std::process::exit(1);
            } else {
                println!("up to date: tool_version {}", report.binary_version);
            }
            Ok(())
        }
        Command::Scenario { command } => match command {
            ScenarioCommand::New {
                change,
                spec,
                requirement,
                heading,
            } => {
                let result = scenario::run_scenario_new(
                    &std::env::current_dir()?,
                    &change,
                    &spec,
                    &requirement,
                    &heading,
                )?;
                println!("scenario: {}", result.scenario_path);
                println!("contract: {}", result.contract_path);
                Ok(())
            }
            ScenarioCommand::Supersede {
                spec,
                old_id,
                with,
                in_change,
            } => {
                let result = scenario::run_scenario_supersede(
                    &std::env::current_dir()?,
                    &spec,
                    &old_id,
                    &with,
                    &in_change,
                )?;
                println!("superseded: {}", result.contract_path);
                Ok(())
            }
        },
    }
}

fn print_check_report(report: &check::CheckOutput, run_tests: bool) {
    let total = report.findings.len() + report.quality_findings.len();

    if total == 0 {
        if report.summary.passed == 0 && !run_tests {
            println!("no issues — 0 structural, 0 execution (contract tests skipped; run `ah check --run-tests` to execute)");
        } else if report.summary.passed == 0 {
            println!("no scenarios to check — 0 passed, 0 findings");
        } else {
            println!("OK — {} passed, 0 findings", report.summary.passed);
        }
        return;
    }

    println!("found {total} issue(s):");

    for finding in &report.findings {
        let spec = &finding.spec;
        let scenario_id = &finding.scenario.id;
        let kind = &finding.kind;
        let category = &finding.category;
        let msg = finding.message.as_deref().unwrap_or("");
        if msg.is_empty() {
            println!("{category}: {spec}/{scenario_id} — {kind}");
        } else {
            println!("{category}: {spec}/{scenario_id} — {kind}: {msg}");
        }
        if let Some(test) = &finding.test {
            let lines: Vec<&str> = test.stdout_tail.lines().collect();
            for line in lines.iter().take(5) {
                println!("  stdout: {line}");
            }
            if lines.len() > 5 {
                println!("  stdout: ... ({} more lines)", lines.len() - 5);
            }
            let lines: Vec<&str> = test.stderr_tail.lines().collect();
            for line in lines.iter().take(5) {
                println!("  stderr: {line}");
            }
            if lines.len() > 5 {
                println!("  stderr: ... ({} more lines)", lines.len() - 5);
            }
        }
    }

    if !report.quality_findings.is_empty() {
        println!();
        for qf in &report.quality_findings {
            println!("quality: {} — {}", qf.kind, qf.message);
            if let Some(kr) = qf.kill_rate {
                println!("  kill_rate: {:.1}%", kr * 100.0);
            }
            if let Some(th) = qf.threshold {
                println!("  threshold: {:.1}%", th * 100.0);
            }
        }
    }

    let summary = &report.summary;
    println!();
    println!(
        "summary: {} passed, {} structural, {} execution, {} quality",
        summary.passed,
        summary.structural,
        summary.execution,
        report.quality_findings.len()
    );
    if !summary.counts_by_kind.is_empty() {
        for (kind, count) in &summary.counts_by_kind {
            println!("  {kind}: {count}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_registers_all_commands_and_config() {
        let guide = build_guide();
        assert_eq!(guide.name(), "ah");
        assert_eq!(guide.version(), env!("CARGO_PKG_VERSION"));

        let all = guide.registry().all();
        for cmd in AH_COMMANDS {
            assert!(all.contains(cmd), "guide registry missing command {cmd}");
        }
    }

    #[test]
    fn error_sink_is_configured_for_ah() {
        let guide = build_guide();
        let sink = guide.error_sink();
        assert_eq!(sink.tool_name, "ah");
        assert!(sink.scratch);
    }

    /// `AH_COMMANDS` and the clap `Command` enum must list the same
    /// subcommands, otherwise typo detection silently drops a command.
    #[test]
    fn ah_commands_match_clap_subcommands() {
        let clap_names: std::collections::BTreeSet<String> = Cli::command()
            .get_subcommands()
            .map(|c| c.get_name().to_string())
            .collect();
        let listed: std::collections::BTreeSet<String> =
            AH_COMMANDS.iter().map(|s| s.to_string()).collect();

        let missing_from_listed: Vec<_> = clap_names.difference(&listed).collect();
        let missing_from_clap: Vec<_> = listed.difference(&clap_names).collect();
        assert!(
            missing_from_listed.is_empty(),
            "clap subcommands not in AH_COMMANDS (typo detection would miss them): {missing_from_listed:?}"
        );
        assert!(
            missing_from_clap.is_empty(),
            "AH_COMMANDS entries not in clap subcommands (stale typo targets): {missing_from_clap:?}"
        );
    }
}
