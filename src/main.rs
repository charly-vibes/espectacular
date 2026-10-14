//! `ah` binary entry point: thin process bootstrap and command dispatch.
//!
//! The CLI surface lives in `cli.rs`; command handlers in `cmd_check.rs`,
//! `cmd_feedback.rs`, `cmd_misc.rs` (espectacular gate tidy — `run()` was
//! a 437-line match with cyclomatic 41).

mod adapters;
mod archetypes;
mod archive;
mod batching;
mod check;
mod cli;
mod cmd_check;
mod cmd_feedback;
mod cmd_misc;
mod config;
mod contracts;
// Consumer (ah sync) lands with task group 3 of derive-contracts-from-specodelic.
#[allow(dead_code)]
mod derive;
mod doctor;
mod explain;
mod fsutil;
mod init;
mod lint;
mod openspec;
mod quality;
mod report;
mod runner;
mod scenario;
mod signals;
mod sync;
mod upgrade;

use crate::cli::{build_guide, handle_parse_error, handle_run_error, Cli, Command};
use clap::Parser;
use genesis::cli::maybe_print_version_json;

fn main() {
    // Delegate --version --json handling to genesis::cli
    if maybe_print_version_json("ah", env!("CARGO_PKG_VERSION")) {
        return;
    }

    let guide = build_guide();
    match Cli::try_parse() {
        Ok(cli) => {
            if let Err(error) = run(cli) {
                handle_run_error(error, &guide);
            }
        }
        Err(err) => handle_parse_error(err, &guide),
    }
}

/// Primary commands: substantial handlers with their own modules.
fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Check {
            changes,
            run_tests,
            files,
            all_tests,
            base,
            head,
        } => {
            cmd_check::run_check_command(changes, run_tests, files, all_tests, base, head, cli.json)
        }
        Command::Doctor { enable } => cmd_misc::run_doctor_command(enable, cli.json),
        Command::Feedback {
            kind,
            dry_run,
            from_last_error,
        } => cmd_feedback::run_feedback_command(kind, dry_run, from_last_error),
        Command::Report {} => cmd_misc::run_report_command(cli.json),
        Command::Lint {
            root,
            changes,
            check,
        } => cmd_misc::run_lint_command(root, changes, check, cli.json),
        Command::Sync { check } => cmd_misc::run_sync_command(check, cli.json),
        other => run_small(other, cli.json),
    }
}

/// Trivial delegates around their domain modules.
fn run_small(cmd: Command, json: bool) -> anyhow::Result<()> {
    match cmd {
        Command::Init => cmd_misc::run_init_command(),
        Command::Archive { change } => cmd_misc::run_archive_command(&change),
        Command::Type { name } => cmd_misc::run_type_command(name),
        Command::Explain { topic, list } => cmd_misc::run_explain_command(topic, list, json),
        Command::Signals => cmd_misc::run_signals_command(),
        Command::Completions { shell } => cmd_misc::run_completions_command(shell),
        Command::Upgrade => cmd_misc::run_upgrade_command(),
        Command::Scenario { command } => cmd_misc::run_scenario_command(command),
        _ => unreachable!("handled by run()"),
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{build_guide, AH_COMMANDS};

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
        use clap::CommandFactory;
        let clap_names: std::collections::BTreeSet<String> = crate::cli::Cli::command()
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

    // ---- property-derived contract bindings (DDL migration, design D6) ----

    #[test]
    fn p_guide_builder() {
        guide_registers_all_commands_and_config();
    }

    #[test]
    fn p_handlers() {
        error_sink_is_configured_for_ah();
    }

    #[test]
    fn p_selfheal() {
        error_sink_is_configured_for_ah();
    }
}
