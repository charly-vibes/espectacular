//! Small `ah` command handlers: doctor, report, lint, init, archive,
//! type, explain, signals, sync, completions, upgrade, scenario.
//!
//! Each is a thin delegate around its domain module, extracted from
//! `run()` (espectacular gate tidy) so the dispatch stays trivial.

use crate::cli::{to_json_envelope, to_json_envelope_with_ok, EnvelopeKind};
use crate::{
    adapters, archetypes, archive, doctor, init, lint, report, scenario, signals, sync, upgrade,
};
use std::io::Write;

/// `ah doctor` — diagnostics, or `--enable <capability>` config surgery.
pub fn run_doctor_command(enable: Option<String>, json: bool) -> anyhow::Result<()> {
    if json {
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
    print_doctor_report(&report)
}

fn print_doctor_report(report: &doctor::DoctorOutcome) -> anyhow::Result<()> {
    for det in &report.detections {
        println!(
            "framework: {} ({})",
            det.name,
            adapters::detection_source_label(det.detection_source)
        );
    }
    for rec in &report.suggestions {
        println!(
            "recommendation: {} — run: {}",
            rec.detail, rec.apply_command
        );
    }
    for s in &report.session_suggestions {
        println!("suggestion: {} — run: {}", s.detail, s.apply_command);
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

/// `ah report` — coverage matrix (spec-to-contract).
pub fn run_report_command(json: bool) -> anyhow::Result<()> {
    let report = report::run_report(&std::env::current_dir()?)?;
    if json {
        // espectacular-yr9: ok must agree with the exit code (same
        // drift class as the check fix) — gaps exit 1, so ok:false.
        let has_gaps = has_report_gaps(&report);
        println!(
            "{}",
            to_json_envelope_with_ok(EnvelopeKind::Ok, &report, !has_gaps)
        );
        std::process::exit(if has_gaps { 1 } else { 0 });
    } else {
        print_report_matrix(&report);
    }
    let has_gaps = has_report_gaps(&report);
    std::process::exit(if has_gaps { 1 } else { 0 });
}

fn has_report_gaps(report: &report::ReportOutput) -> bool {
    report.summary.missing > 0 || report.summary.failing > 0
}

fn print_report_matrix(report: &report::ReportOutput) {
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

/// `ah lint` — spec authoring-quality findings.
pub fn run_lint_command(
    root: Option<String>,
    changes: Option<String>,
    check: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let repo_root = match &root {
        Some(path) => cwd.join(path),
        None => cwd,
    };
    let specs_dir = lint::specs_dir_for(&repo_root)?;
    let mut output = lint::run_lint_query(&specs_dir, check.as_deref())?;
    if let Some(change) = &changes {
        let overlay = lint::run_change_overlay(&repo_root, change, check.as_deref())?;
        output = lint::merge_outputs(output, overlay);
    }
    // espectacular-yr9: ok must agree with the exit code — warning-
    // only findings exit 0 and stay ok:true; error findings exit 1
    // with ok:false (same drift class as the check fix).
    let lint_code = lint::exit_code(&output);
    if json {
        println!(
            "{}",
            to_json_envelope_with_ok(
                genesis::envelope::EnvelopeKind::Warning,
                &output,
                lint_code == 0
            )
        );
    } else {
        lint::print_report(&output);
    }
    std::io::stdout().flush().unwrap_or_default();
    // Record the completed run so `ah doctor` can suggest the first
    // lint (advisory-only; best-effort).
    doctor::record_lint_run(&repo_root);
    std::process::exit(lint_code);
}

/// `ah init` — project scaffolding.
pub fn run_init_command() -> anyhow::Result<()> {
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

/// `ah archive <change>` — move a deployed change into specs.
pub fn run_archive_command(change: &str) -> anyhow::Result<()> {
    let result = archive::run_archive(&std::env::current_dir()?, change)?;
    for item in &result.moved {
        println!("archived: {item}");
    }
    Ok(())
}

/// `ah type [code]` — archetype lookup.
pub fn run_type_command(name: Option<String>) -> anyhow::Result<()> {
    match name.as_deref() {
        None => {
            println!("{}", archetypes::list_archetypes());
        }
        Some(code) => {
            let upper = code.to_uppercase();
            match archetypes::lookup(&upper) {
                Some(a) => println!("{}", a.body),
                None => print_type_suggestion(code),
            }
        }
    }
    Ok(())
}

fn print_type_suggestion(code: &str) -> ! {
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

/// `ah explain [topic]` — playbook lookup.
pub fn run_explain_command(topic: Option<String>, list: bool, json: bool) -> anyhow::Result<()> {
    crate::explain::run_explain(topic.as_deref(), list, json)
}

/// `ah signals` — dont drift signals as JSON.
pub fn run_signals_command() -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let drift = signals::collect_drift_signals(&project_root);
    println!("{}", to_json_envelope(EnvelopeKind::Stats, &drift));
    Ok(())
}

/// `ah sync [--check]` — write property-derived contracts from lint-clean Properties rows.
pub fn run_sync_command(check: bool, json: bool) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let outcome = sync::run_sync("spk", &project_root, check)?;
    if json {
        // espectacular-yr9: ok must agree with the exit code (same
        // drift class as the check fix) — a failed --check exits 1,
        // so ok:false; a clean sync stays ok:true.
        println!(
            "{}",
            to_json_envelope_with_ok(EnvelopeKind::Ok, &outcome, outcome.is_ok())
        );
    } else {
        print_sync_outcome(&outcome);
    }
    // C-sync-refusal / C-sync-check: non-zero without further action.
    std::process::exit(if outcome.is_ok() { 0 } else { 1 });
}

fn print_sync_outcome(outcome: &sync::SyncOutcome) {
    if let Some(msg) = &outcome.spk_unavailable {
        eprintln!("spk-unavailable: {msg}");
    }
    for (file, reason) in &outcome.refusals {
        eprintln!("refused: {file}: {reason}");
    }
    for path in &outcome.missing {
        eprintln!("missing contract: {path}");
    }
    for path in &outcome.stale {
        eprintln!("contract stale: {path}");
    }
    for path in &outcome.fresh {
        println!("fresh: {path}");
    }
    for path in &outcome.wrote {
        println!("wrote: {path}");
    }
}

/// `ah completions <shell>`.
pub fn run_completions_command(shell: clap_complete::Shell) -> anyhow::Result<()> {
    use clap::CommandFactory;
    let mut cmd = crate::cli::Cli::command();
    genesis::cli::generate_completions(&mut cmd, shell).map_err(|e| anyhow::anyhow!("{}", e))
}

/// `ah upgrade` — align config tool_version with the binary.
pub fn run_upgrade_command() -> anyhow::Result<()> {
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

/// `ah scenario <new|supersede>`.
pub fn run_scenario_command(command: crate::cli::ScenarioCommand) -> anyhow::Result<()> {
    match command {
        crate::cli::ScenarioCommand::New {
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
        crate::cli::ScenarioCommand::Supersede {
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
    }
}
