//! xtask — workspace automation for the zai-rs 0.5.0 optimization plan.
//!
//! One command surface for every plan-defined check. Subcommands are added per
//! task (P00 establishes `contract`, `forbidden`, `public-api`, `module-size`,
//! `dep-budget`, `coverage`, `docs`, `version`, `examples`, `test-budget`,
//! `tests`, `fuzz`, `sbom`, `future-incompat`, `package`, `release`).
//!
//! Run with:
//!   cargo run --locked -p xtask -- <command> [args]
//!
//! Every command exits 0 on success and non-zero on failure, so they can be
//! wired directly into CI and the plan's verification scripts.

mod contract;
mod forbidden;
mod hash;
mod public_api;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        return ExitCode::FAILURE;
    }
    match args[0].as_str() {
        "contract" => match args.get(1).map(String::as_str) {
            Some("verify") => contract::verify(args.iter().any(|a| a == "--require-covered")),
            Some("generate") => contract::generate(),
            Some("check") => contract::check(),
            _ => {
                eprintln!("usage: xtask contract {{verify|generate|check}} [--require-covered]");
                ExitCode::FAILURE
            },
        },
        "forbidden" => match args.get(1).map(String::as_str) {
            Some("check") => {
                let phase = args.get(2).cloned().unwrap_or_else(|| "P10".to_string());
                forbidden::check(&phase)
            },
            _ => {
                eprintln!("usage: xtask forbidden check <phase>");
                ExitCode::FAILURE
            },
        },
        "public-api" => match args.get(1).map(String::as_str) {
            Some("check") => public_api::check(),
            _ => {
                eprintln!("usage: xtask public-api check");
                ExitCode::FAILURE
            },
        },
        "module-size" => match args.get(1).map(String::as_str) {
            Some("check") => not_yet("module-size check", "P10"),
            _ => {
                eprintln!("usage: xtask module-size check");
                ExitCode::FAILURE
            },
        },
        "dep-budget" => match args.get(1).map(String::as_str) {
            Some("check") => not_yet("dep-budget check", "P10"),
            _ => {
                eprintln!("usage: xtask dep-budget check");
                ExitCode::FAILURE
            },
        },
        "coverage" => match args.get(1).map(String::as_str) {
            Some("check") => not_yet("coverage check", "P11"),
            _ => {
                eprintln!("usage: xtask coverage check <json>");
                ExitCode::FAILURE
            },
        },
        "docs" => match args.get(1).map(String::as_str) {
            Some("check") => not_yet("docs check", "P13"),
            _ => {
                eprintln!("usage: xtask docs check");
                ExitCode::FAILURE
            },
        },
        "version" => match args.get(1).map(String::as_str) {
            Some("check") => not_yet("version check", "P13"),
            _ => {
                eprintln!("usage: xtask version check");
                ExitCode::FAILURE
            },
        },
        "examples" => match args.get(1).map(String::as_str) {
            Some("check") | Some("generate") => not_yet("examples", "P12"),
            _ => {
                eprintln!("usage: xtask examples {{check|generate}}");
                ExitCode::FAILURE
            },
        },
        "test-budget" => not_yet("test-budget", "P11"),
        "tests" => match args.get(1).map(String::as_str) {
            Some("check-no-ignore") => not_yet("tests check-no-ignore", "P11"),
            _ => {
                eprintln!("usage: xtask tests check-no-ignore");
                ExitCode::FAILURE
            },
        },
        "fuzz" => match args.get(1).map(String::as_str) {
            Some("smoke") => not_yet("fuzz smoke", "P11"),
            _ => {
                eprintln!("usage: xtask fuzz smoke --seconds <n>");
                ExitCode::FAILURE
            },
        },
        "sbom" => match args.get(1).map(String::as_str) {
            Some("generate") => not_yet("sbom generate", "P14"),
            _ => {
                eprintln!("usage: xtask sbom generate");
                ExitCode::FAILURE
            },
        },
        "future-incompat" => match args.get(1).map(String::as_str) {
            Some("check") => not_yet("future-incompat check", "P14"),
            _ => {
                eprintln!("usage: xtask future-incompat check");
                ExitCode::FAILURE
            },
        },
        "package" => match args.get(1).map(String::as_str) {
            Some("check") => not_yet("package check", "P14"),
            _ => {
                eprintln!("usage: xtask package check");
                ExitCode::FAILURE
            },
        },
        "release" => match args.get(1).map(String::as_str) {
            Some("verify") => not_yet("release verify", "P15"),
            _ => {
                eprintln!("usage: xtask release verify <version>");
                ExitCode::FAILURE
            },
        },
        other => {
            eprintln!("unknown command: {other}");
            usage();
            ExitCode::FAILURE
        },
    }
}

/// Placeholder for commands that belong to a later task. Returns a specific
/// non-failure exit code (2) so callers can distinguish "not implemented yet"
/// from a genuine failure (1) during early-task runs. The plan's verification
/// scripts for a given task only invoke that task's commands, so this only
/// fires if invoked out of order.
fn not_yet(name: &str, task: &str) -> ExitCode {
    eprintln!("{name}: not implemented until {task}");
    ExitCode::from(2)
}

fn usage() {
    eprintln!(
        "usage: xtask <command> [args]\n\
         commands:\n\
         contract {{verify|generate|check}} [--require-covered]\n\
         forbidden check <phase>\n\
         public-api check\n\
         module-size check\n\
         dep-budget check\n\
         coverage check <json>\n\
         docs check\n\
         version check\n\
         examples {{check|generate}}\n\
         test-budget\n\
         tests check-no-ignore\n\
         fuzz smoke --seconds <n>\n\
         sbom generate\n\
         future-incompat check\n\
         package check\n\
         release verify <version>"
    );
}
