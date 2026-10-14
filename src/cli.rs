//! CLI surface for `ah`: the clap definitions, the genesis `Guide`
//! scaffold, shared JSON-envelope helpers, and top-level error handling.
//!
//! Split out of `main.rs` (espectacular gate tidy): command handlers live
//! in `cmd_*.rs` modules; this module owns only the arg surface and the
//! process-level error paths.

use clap::{Parser, Subcommand};
use genesis::envelope::Envelope;
pub use genesis::envelope::EnvelopeKind;
use genesis::guide::Guide;

use crate::config;

#[derive(Parser)]
#[command(name = "ah", version)]
pub struct Cli {
    /// Output machine-readable JSON
    #[arg(short = 'j', long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Validate spec-test correspondence (structural analysis by default, execution with --run-tests)
    Check {
        #[arg(long = "changes")]
        changes: Vec<String>,
        /// Run contract tests (shell, custom, property, snapshot entries).
        /// By default, ah check only performs fast structural analysis
        /// (spec/contract correspondence).
        #[arg(long = "run-tests")]
        run_tests: bool,
        /// Restrict contract-test execution to capabilities owning these
        /// changed files (repo-relative, comma-separated). Defaults to the
        /// worktree git diff when omitted; --all-tests overrides.
        #[arg(long = "files", value_delimiter = ',')]
        files: Vec<String>,
        /// Run every declared contract test, bypassing selection.
        #[arg(long = "all-tests")]
        all_tests: bool,
        /// Derive the changed-file set from the commit range
        /// git diff --name-only <base> <head> instead of the worktree
        /// diff (two-dot snapshot diff, testaruda-jdw5). Only applies
        /// with --run-tests; a range that yields no files or a ref that
        /// fails to resolve runs every declared contract.
        #[arg(long = "base")]
        base: Option<String>,
        /// Head ref for the --base range (defaults to HEAD).
        #[arg(long = "head", requires = "base")]
        head: Option<String>,
    },
    /// Run diagnostic checks or enable a capability
    Doctor {
        #[arg(long)]
        enable: Option<String>,
    },
    /// Initialize espectacular in the project
    Init,
    /// Statically analyze spec files for authoring-quality findings
    Lint {
        /// Optional repo root to lint (defaults to the current directory)
        root: Option<String>,
        /// Lint a change's spec overlay in addition to deployed specs
        #[arg(long)]
        changes: Option<String>,
        /// Run only a single check category (e.g. vague-qualifier)
        #[arg(long)]
        check: Option<String>,
    },
    /// Generate a coverage report (spec-to-contract matrix)
    Report {},
    /// Archive a deployed change into specs
    Archive { change: String },
    /// Look up archetype definitions
    Type { name: Option<String> },
    /// Explain capabilities, archetypes, or list available topics
    Explain {
        topic: Option<String>,
        #[arg(long)]
        list: bool,
    },
    /// Update tool_version in config to match binary
    Upgrade,
    /// Manage scenarios (new, supersede)
    Scenario {
        #[command(subcommand)]
        command: ScenarioCommand,
    },
    /// Read dont rejection events and emit drift signals as JSON.
    Signals,
    /// Create or refresh property-derived contracts from lint-clean Properties rows
    Sync {
        /// CI mode: write nothing, exit non-zero on drift or missing contracts
        #[arg(long)]
        check: bool,
    },
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
pub enum ScenarioCommand {
    /// Create a new scenario under a change
    New {
        change: String,
        spec: String,
        #[arg(long)]
        requirement: String,
        heading: String,
    },
    /// Supersede an old scenario with a new one
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
pub const AH_COMMANDS: &[&str] = &[
    "check",
    "doctor",
    "init",
    "lint",
    "report",
    "archive",
    "type",
    "explain",
    "upgrade",
    "scenario",
    "signals",
    "sync",
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
pub fn build_guide() -> Guide {
    Guide::builder("ah", env!("CARGO_PKG_VERSION"))
        .about("Behavioral verification layer enforcing spec-test correspondence")
        .commands(AH_COMMANDS)
        .config::<config::Config>()
        .build()
}

/// Wrap any serializable data in a shared genesis envelope.
pub fn to_json_envelope<T: serde::Serialize>(kind: EnvelopeKind, data: T) -> String {
    let env: Envelope<T> = Envelope::success(env!("CARGO_PKG_VERSION"), kind, data, vec![], vec![]);
    serde_json::to_string(&env).expect("envelope serialization")
}

/// Like [`to_json_envelope`], but with an explicit `ok` override.
///
/// Needed for commands whose *result data carries a verdict* rather than
/// the command's own success: `ah check` with blocking findings exits 1
/// even though the command itself ran fine, and `Envelope::success`
/// hardcodes `ok: true`. Machine consumers classify an invocation by the
/// `(ok, exit_code)` pair — emitting `ok: true` with exit 1 satisfies
/// neither an ok-envelope nor an error-envelope contract (the
/// ok:true+exit-1 drift, evallerina-54f; round2-audit: 8/12 native
/// ah_check toolResults). `ok` must agree with the exit code.
pub fn to_json_envelope_with_ok<T: serde::Serialize>(
    kind: EnvelopeKind,
    data: T,
    ok: bool,
) -> String {
    let mut env: Envelope<T> =
        Envelope::success(env!("CARGO_PKG_VERSION"), kind, data, vec![], vec![]);
    env.ok = ok;
    serde_json::to_string(&env).expect("envelope serialization")
}

/// Wrapper that renders an anyhow error with its full context chain (`:#`)
/// as its `Display` output, so `ErrorSink::handle` prints the rich message
/// while genesis owns the scratch + footer mechanics.
pub struct FormattedError(pub String);

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

/// Route a handler error through the genesis `ErrorSink`: it owns the
/// error message, the self-healing footer, and the best-effort scratch
/// write for `--from-last-error`. We suppress the generic
/// "-> Run: ah doctor" footer (`with_suggest(false)`) because we provide
/// the feedback footer instead (`with_feedback`); enabling suggestions
/// would print a duplicate footer.
pub fn handle_run_error(error: anyhow::Error, guide: &Guide) -> ! {
    let sink = guide
        .error_sink()
        .with_suggest(false)
        .with_feedback(Some("feedback"));
    let mut stderr = std::io::stderr();
    sink.handle(&FormattedError(format!("{error:#}")), &mut stderr);
    // Exit 1 matches the `exit` field ErrorSink writes into the scratch
    // record, so `feedback --from-last-error` reports a truthful exit
    // code. (The typo-subcommand path below keeps exit 2 to match clap's
    // invalid-subcommand convention.)
    std::process::exit(1);
}

/// Handle a clap parse error: suggest the closest subcommand on a typo
/// (exit 2, clap's invalid-subcommand convention), otherwise let clap
/// print its own usage error and exit.
pub fn handle_parse_error(err: clap::error::Error, guide: &Guide) -> ! {
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
    err.exit()
}
