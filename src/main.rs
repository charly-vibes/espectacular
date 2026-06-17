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
mod runner;
mod scenario;
mod signals;
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
    Doctor {
        #[arg(long)]
        enable: Option<String>,
    },
    Init,
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
        #[arg(long)]
        json: bool,
    },
    Upgrade,
    Scenario {
        #[command(subcommand)]
        command: ScenarioCommand,
    },
    /// Read dont rejection events and emit drift signals as JSON.
    Signals,
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
            let has_blocking = report
                .findings
                .iter()
                .any(|f| f.category == "structural" || f.category == "execution");
            std::process::exit(if has_blocking { 1 } else { 0 });
        }
        Command::Doctor { enable } => {
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
            for rec in &report.recommendations {
                println!(
                    "recommendation: {} — run: {}",
                    rec.detail, rec.apply_command
                );
            }
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
        Command::Explain { topic, list, json } => {
            explain::run_explain(topic.as_deref(), list, json)
        }
        Command::Signals => {
            let project_root = std::env::current_dir()?;
            let drift = signals::collect_drift_signals(&project_root);
            println!("{}", serde_json::to_string_pretty(&drift)?);
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
