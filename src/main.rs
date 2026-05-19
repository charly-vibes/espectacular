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
    let cli = Cli::parse();
    match cli.command {
        Command::Check => todo!("ah check not yet implemented"),
    }
}
