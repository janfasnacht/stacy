//! xtask - Development tasks for stacy
//!
//! Run with: `cargo xtask <command>`
//!
//! Available commands:
//! - codegen: Generate Stata wrappers and help files from schema
//! - verify:  Verify generated files match schema

mod codegen;
mod schema;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Development tasks for stacy")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate Stata wrappers and help files from schema
    Codegen {
        /// Only check if files are up to date (don't write)
        #[arg(long)]
        check: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Verify generated files match schema
    Verify {
        /// Show diff on mismatch
        #[arg(long)]
        diff: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Codegen { check, verbose } => {
            codegen::run(check, verbose)?;
        }
        Commands::Verify { diff } => {
            codegen::verify(diff)?;
        }
    }

    Ok(())
}
