//! `stacy remove` command implementation
//!
//! Removes packages from stacy.toml and lockfile.
//! Packages remain in the global cache for potential reuse by other projects.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, RemoveOutput};
use crate::error::{Error, Result};
use crate::packages::lockfile::{load_lockfile, remove_package as lockfile_remove, save_lockfile};
use crate::project::config::{load_config, write_config};
use crate::project::Project;
use clap::Args;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy remove estout                     Remove a package
  stacy remove reghdfe ftools             Remove multiple packages")]
pub struct RemoveArgs {
    /// Package names to remove
    #[arg(value_name = "PACKAGE", required = true)]
    pub packages: Vec<String>,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

/// Result of removing a single package
#[derive(Debug)]
struct RemovedPackage {
    name: String,
    removed_from_config: bool,
    removed_from_lockfile: bool,
}

pub fn execute(args: &RemoveArgs) -> Result<()> {
    let format = args.format;

    // Find project (must exist for remove)
    let project = Project::find()?.ok_or_else(|| {
        Error::Config("Not in a stacy project. Run 'stacy init' first.".to_string())
    })?;

    // Load config
    let mut config = load_config(&project.root)?
        .ok_or_else(|| Error::Config("No stacy.toml found. Run 'stacy init' first.".to_string()))?;

    // Load lockfile (may not exist)
    let mut lockfile =
        load_lockfile(&project.root)?.unwrap_or_else(crate::packages::lockfile::create_lockfile);

    if format == OutputFormat::Human {
        println!("Removing {} package(s)...", args.packages.len());
        println!();
    }

    let mut results: Vec<RemovedPackage> = Vec::new();

    for package in &args.packages {
        let package_lower = package.to_lowercase();

        let mut result = RemovedPackage {
            name: package_lower.clone(),
            removed_from_config: false,
            removed_from_lockfile: false,
        };

        // Remove from config (dependencies or dev)
        if config.packages.remove_dependency(&package_lower).is_some() {
            result.removed_from_config = true;
        }

        // Remove from lockfile
        if lockfile_remove(&mut lockfile, &package_lower).is_some() {
            result.removed_from_lockfile = true;
        }

        // Note: We don't delete files from the global cache - they may be used by other projects.
        // Use `stacy cache packages clean` to remove unused cached packages.

        let was_found = result.removed_from_config || result.removed_from_lockfile;

        if format == OutputFormat::Human {
            if was_found {
                println!(
                    "  - {} (config: {}, lock: {})",
                    package_lower,
                    if result.removed_from_config {
                        "yes"
                    } else {
                        "no"
                    },
                    if result.removed_from_lockfile {
                        "yes"
                    } else {
                        "no"
                    }
                );
            } else {
                println!("  ? {} not found", package_lower);
            }
        }

        results.push(result);
    }

    // Write updated config
    write_config(&config, &project.root)?;

    // Write updated lockfile
    save_lockfile(&project.root, &lockfile)?;

    // Calculate summary
    let removed_count = results
        .iter()
        .filter(|r| r.removed_from_config || r.removed_from_lockfile)
        .count() as i32;
    let not_found_count = results
        .iter()
        .filter(|r| !r.removed_from_config && !r.removed_from_lockfile)
        .count() as i32;

    let status = if removed_count == 0 {
        "error"
    } else {
        "success"
    };

    let output = RemoveOutput {
        status: status.to_string(),
        removed: removed_count,
        not_found: not_found_count,
        total: results.len() as i32,
    };

    // Output results
    match format {
        OutputFormat::Json => print_json_output(&results, &output),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => print_human_summary(&output),
    }

    if removed_count == 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn print_json_output(results: &[RemovedPackage], output: &RemoveOutput) {
    use serde_json::json;

    let packages: Vec<_> = results
        .iter()
        .map(|r| {
            let found = r.removed_from_config || r.removed_from_lockfile;
            json!({
                "name": r.name,
                "found": found,
                "removed_from_config": r.removed_from_config,
                "removed_from_lockfile": r.removed_from_lockfile,
            })
        })
        .collect();

    let json_output = json!({
        "status": output.status,
        "packages": packages,
        "summary": {
            "removed": output.removed,
            "not_found": output.not_found,
            "total": output.total,
        }
    });

    println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
}

fn print_human_summary(output: &RemoveOutput) {
    println!();
    let mut summary = Vec::new();
    if output.removed > 0 {
        summary.push(format!("{} removed", output.removed));
    }
    if output.not_found > 0 {
        summary.push(format!("{} not found", output.not_found));
    }

    if summary.is_empty() {
        println!("No packages removed.");
    } else {
        println!("Complete: {}", summary.join(", "));
    }
}

#[cfg(test)]
mod tests {
    // Tests for remove command will be added in integration tests
}
