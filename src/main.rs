mod check;
mod config;
mod contracts;
mod doctor;
mod init;
mod openspec;
mod runner;

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
    }
}
