#![allow(dead_code)] // Allow unused code during early development
#![allow(clippy::enum_variant_names)] // Error types have Error suffix intentionally
#![allow(clippy::upper_case_acronyms)] // SSC, JSON, etc. are standard acronyms

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Parser, Subcommand};
use std::process;

mod cache;
mod cli;
mod deps;
mod error;
mod executor;
mod metrics;
mod packages;
mod project;
mod task;
mod test;
mod update_check;
mod utils;

#[derive(Parser)]
#[command(name = "stacy")]
#[command(version)]
#[command(before_help = concat!("\u{25b8} stacy ", env!("CARGO_PKG_VERSION")))]
#[command(about = "Reproducible Stata execution with proper error detection")]
#[command(
    long_about = "stacy executes Stata scripts with proper error detection and exit codes, \
and manages project dependencies via lockfiles."
)]
#[command(after_help = "\
Getting started:
  stacy init                     Create stacy.toml in current directory
  stacy add estout               Add a package from SSC
  stacy run analysis.do          Run script with error detection
  stacy doctor                   Check system configuration

Docs: https://stacy.janfasnacht.com")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // === Execution (1-9) ===
    /// Run a Stata script with proper error detection
    #[command(display_order = 1)]
    Run(cli::run::RunArgs),
    /// Run a defined task from stacy.toml
    #[command(display_order = 2)]
    Task(cli::task::TaskArgs),
    /// Run tests by convention
    #[command(display_order = 3)]
    Test(cli::test::TestArgs),
    // === Project (10-19) ===
    /// Initialize a new stacy project
    #[command(display_order = 10)]
    Init(cli::init::InitArgs),
    /// Show dependency tree for a script
    #[command(display_order = 11)]
    Deps(cli::deps::DepsArgs),

    // === Packages (20-29) ===
    /// Add packages to stacy.toml and install them
    #[command(display_order = 20)]
    Add(cli::add::AddArgs),
    /// Remove packages from stacy.toml and uninstall them
    #[command(display_order = 21)]
    Remove(cli::remove::RemoveArgs),
    /// Install all packages from lockfile
    #[command(display_order = 22)]
    Install(cli::install::InstallArgs),
    /// Update packages to latest versions
    #[command(display_order = 23)]
    Update(cli::update::UpdateArgs),
    /// List installed packages
    #[command(display_order = 24)]
    List(cli::list::ListArgs),
    /// Check for outdated packages
    #[command(display_order = 25)]
    Outdated(cli::outdated::OutdatedArgs),
    /// Generate or verify lockfile from stacy.toml
    #[command(display_order = 26)]
    Lock(cli::lock::LockArgs),

    // === Info (30-39) ===
    /// Show current environment configuration
    #[command(display_order = 30)]
    Env(cli::env::EnvArgs),
    /// Run diagnostics and check system configuration
    #[command(display_order = 31)]
    Doctor(cli::doctor::DoctorArgs),
    /// Look up Stata error code details
    #[command(display_order = 32)]
    Explain(cli::explain::ExplainArgs),

    // === Advanced (40-49) ===
    /// Manage the build cache
    #[command(display_order = 40)]
    Cache(cli::cache::CacheArgs),
    /// Benchmark script execution
    #[command(display_order = 41)]
    Bench(cli::bench::BenchArgs),
}

/// Handle clap parse errors with custom suggestions for common mistakes
fn handle_parse_error(mut err: clap::Error) -> ! {
    match err.kind() {
        ErrorKind::UnknownArgument => {
            // Check for common flag mistakes
            if let Some(ContextValue::String(arg)) = err.get(ContextKind::InvalidArg) {
                let suggestions = match arg.as_str() {
                    // -e is common in Perl, Ruby, Rscript
                    "-e" | "--exec" | "--execute" | "--expression" => Some(vec![
                        "use '-c' for inline code: stacy run -c 'display 1'".into(),
                    ]),
                    _ => None,
                };
                if let Some(suggestions) = suggestions {
                    err.insert(
                        ContextKind::Suggested,
                        ContextValue::StyledStrs(suggestions),
                    );
                }
            }
        }
        ErrorKind::InvalidSubcommand => {
            // Check for common subcommand mistakes
            if let Some(ContextValue::String(cmd)) = err.get(ContextKind::InvalidSubcommand) {
                let suggestions = match cmd.as_str() {
                    // "do" is Stata's own command name
                    "do" | "execute" | "exec" => Some(vec![
                        "use 'stacy run' to execute scripts: stacy run script.do".into(),
                    ]),
                    // Shorthand attempt
                    "e" => Some(vec![
                        "use 'stacy env' to show configuration: stacy env".into(),
                        "use 'stacy explain' for error codes: stacy explain 111".into(),
                    ]),
                    "r" => Some(vec![
                        "use 'stacy run' to execute scripts: stacy run script.do".into(),
                        "use 'stacy remove' to uninstall packages: stacy remove estout".into(),
                    ]),
                    _ => None,
                };
                if let Some(suggestions) = suggestions {
                    err.insert(
                        ContextKind::Suggested,
                        ContextValue::StyledStrs(suggestions),
                    );
                }
            }
        }
        _ => {}
    }
    err.exit()
}

fn main() {
    update_check::maybe_notify_and_spawn();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => handle_parse_error(e),
    };

    let result = match &cli.command {
        Commands::Run(args) => cli::run::execute(args),

        Commands::Init(args) => cli::init::execute(args),
        Commands::Add(args) => cli::add::execute(args),
        Commands::Remove(args) => cli::remove::execute(args),
        Commands::Update(args) => cli::update::execute(args),
        Commands::Install(args) => cli::install::execute(args),
        Commands::List(args) => cli::list::execute(args),
        Commands::Outdated(args) => cli::outdated::execute(args),
        Commands::Lock(args) => cli::lock::execute(args),
        Commands::Deps(args) => cli::deps::execute(args),
        Commands::Env(args) => cli::env::execute(args),
        Commands::Doctor(args) => cli::doctor::execute(args),
        Commands::Explain(args) => cli::explain::execute(args),
        Commands::Task(args) => cli::task::execute(args),
        Commands::Test(args) => cli::test::execute(args),
        Commands::Cache(args) => cli::cache::execute(args),
        Commands::Bench(args) => cli::bench::execute(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
