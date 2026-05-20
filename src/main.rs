mod archive;
mod check;
mod config;
mod contracts;
mod doctor;
mod init;
mod openspec;
mod runner;
mod scenario;
mod upgrade;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ah", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check {
        #[arg(long = "changes")]
        changes: Vec<String>,
    },
    Doctor,
    Init,
    Archive {
        change: String,
    },
    Upgrade,
    Scenario {
        #[command(subcommand)]
        command: ScenarioCommand,
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

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(2);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { changes } => {
            let report = check::run_check(&std::env::current_dir()?, &changes)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            let exit_code = if report.findings.is_empty() { 0 } else { 1 };
            std::process::exit(exit_code);
        }
        Command::Doctor => {
            let report = doctor::run_doctor(&std::env::current_dir()?)?;
            if report.healthy {
                println!("healthy: all checks passed");
            } else {
                for d in &report.diagnostics {
                    eprintln!("{}: {}", d.kind, d.detail);
                }
                std::process::exit(1);
            }
            Ok(())
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
