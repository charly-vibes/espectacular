mod check;
mod config;
mod contracts;
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
    Check,
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
        Command::Check => {
            let report = check::run_check(&std::env::current_dir()?)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            let exit_code = if report.findings.is_empty() { 0 } else { 1 };
            std::process::exit(exit_code);
        }
    }
}
