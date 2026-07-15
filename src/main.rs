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

use clap::{CommandFactory, Parser, Subcommand};

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
    // Handle --version --json before clap processes it
    let args: Vec<String> = std::env::args().collect();
    let has_version = args.iter().any(|a| {
        a == "--version"
            || a == "-V"
            || (a.starts_with("-") && !a.starts_with("--") && a.contains('V') && !a.contains('h'))
    });
    if has_version
        && !args
            .iter()
            .any(|a| a == "--help" || a == "-h" || a == "-jh")
    {
        let has_json = args.iter().any(|a| {
            a == "--json"
                || a == "-j"
                || (a.starts_with("-j") && !a.starts_with("--") && !a.contains('h'))
        });
        if has_json {
            let envelope = serde_json::json!({
                "ok": true,
                "envelope_version": "0.2",
                "cli_version": env!("CARGO_PKG_VERSION"),
                "envelope_kind": "version",
                "data": {
                    "name": "ah",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "warnings": [],
                "hints": [],
                "meta": {
                    "duration_ms": 0,
                    "tx": serde_json::Value::Null,
                    "request_id": serde_json::Value::Null
                }
            });
            println!("{}", serde_json::to_string(&envelope).unwrap());
            return;
        }
    }

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
            println!("{}", serde_json::to_string(&report)?);
            let has_blocking = report
                .findings
                .iter()
                .any(|f| f.category == "structural" || f.category == "execution");
            std::process::exit(if has_blocking { 1 } else { 0 });
        }
        Command::Doctor { enable } => {
            if cli.json {
                let report = doctor::run_doctor(&std::env::current_dir()?)?;
                let output = doctor::doctor_to_json(&report);
                println!("{}", serde_json::to_string(&output)?);
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
        Command::Report {} => {
            let report = report::run_report(&std::env::current_dir()?)?;
            if cli.json {
                println!("{}", serde_json::to_string(&report)?);
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
            println!("{}", serde_json::to_string(&drift)?);
            Ok(())
        }
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, &name, &mut std::io::stdout());
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
