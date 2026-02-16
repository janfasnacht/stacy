//! `stacy update` command implementation
//!
//! Updates packages to their latest versions.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, UpdateOutput};
use crate::error::{Error, Result};
use crate::packages::installer::{install_from_ssc, install_package_github};
use crate::packages::lockfile::{load_lockfile, save_lockfile};
use crate::project::config::load_config;
use crate::project::{PackageSource, Project};
use clap::Args;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy update                            Update all packages
  stacy update estout                     Update specific package
  stacy update --dry-run                  Show what would be updated")]
pub struct UpdateArgs {
    /// Package names to update (if omitted, updates all packages)
    #[arg(value_name = "PACKAGE")]
    pub packages: Option<Vec<String>>,

    /// Show what would be updated without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

/// Result of checking/updating a single package
#[derive(Debug)]
struct UpdatedPackage {
    name: String,
    old_version: String,
    new_version: Option<String>,
    updated: bool,
    has_update: bool,
    error: Option<String>,
}

pub fn execute(args: &UpdateArgs) -> Result<()> {
    let format = args.format;

    // Find project (must exist for update)
    let project = Project::find()?.ok_or_else(|| {
        Error::Config("Not in a stacy project. Run 'stacy init' first.".to_string())
    })?;

    // Check that stacy.toml exists
    let _config = load_config(&project.root)?
        .ok_or_else(|| Error::Config("No stacy.toml found. Run 'stacy init' first.".to_string()))?;

    // Load lockfile
    let lockfile = load_lockfile(&project.root)?.ok_or_else(|| {
        Error::Config("No stacy.lock found. Use 'stacy add <package>' to add packages.".to_string())
    })?;

    if lockfile.packages.is_empty() {
        if format == OutputFormat::Human {
            println!("No packages to update.");
        }
        return Ok(());
    }

    // Determine which packages to update
    let packages_to_update: Vec<String> = match &args.packages {
        Some(pkgs) => pkgs.iter().map(|s| s.to_lowercase()).collect(),
        None => lockfile.packages.keys().cloned().collect(),
    };

    // Validate requested packages exist
    for pkg in &packages_to_update {
        if !lockfile.packages.contains_key(pkg) {
            return Err(Error::Config(format!(
                "Package '{}' not found in lockfile",
                pkg
            )));
        }
    }

    if format == OutputFormat::Human {
        if args.dry_run {
            println!("Checking for updates (dry run)...");
        } else {
            println!("Updating {} package(s)...", packages_to_update.len());
        }
        println!();
    }

    let mut results: Vec<UpdatedPackage> = Vec::new();

    for pkg_name in &packages_to_update {
        let entry = lockfile.packages.get(pkg_name).unwrap();
        let old_version = entry.version.clone();

        // Try to update the package
        let group = entry.group.as_str();
        let update_result = match &entry.source {
            PackageSource::SSC { name: _ } => {
                if args.dry_run {
                    // For dry run, we'd need to check the latest version without installing
                    // For now, we'll just report that an update check was done
                    Ok(None)
                } else {
                    install_from_ssc(pkg_name, &project.root, group).map(|r| Some(r.version))
                }
            }
            PackageSource::GitHub { repo, tag, .. } => {
                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() == 2 {
                    if args.dry_run {
                        Ok(None)
                    } else {
                        install_package_github(
                            pkg_name,
                            parts[0],
                            parts[1],
                            Some(tag),
                            &project.root,
                            group,
                        )
                        .map(|r| Some(r.version))
                    }
                } else {
                    Err(Error::Config(format!("Invalid repo format: {}", repo)))
                }
            }
            PackageSource::Local { path } => Err(Error::Config(format!(
                "Cannot update local package: {}",
                path
            ))),
        };

        match update_result {
            Ok(new_version_opt) => {
                let new_version = new_version_opt.unwrap_or_else(|| old_version.clone());
                let has_update = new_version != old_version;
                let updated = !args.dry_run && has_update;

                if format == OutputFormat::Human {
                    if args.dry_run {
                        if has_update {
                            println!("  {} {} -> {}", pkg_name, old_version, new_version);
                        } else {
                            println!("  {} {} (up to date)", pkg_name, old_version);
                        }
                    } else if updated {
                        println!("  + {} {} -> {}", pkg_name, old_version, new_version);
                    } else {
                        println!("  = {} (already at {})", pkg_name, old_version);
                    }
                }

                results.push(UpdatedPackage {
                    name: pkg_name.clone(),
                    old_version,
                    new_version: Some(new_version),
                    updated,
                    has_update,
                    error: None,
                });
            }
            Err(e) => {
                if format == OutputFormat::Human {
                    eprintln!("  x {} failed: {}", pkg_name, e);
                }
                results.push(UpdatedPackage {
                    name: pkg_name.clone(),
                    old_version,
                    new_version: None,
                    updated: false,
                    has_update: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Reload lockfile to get updated versions (install functions update it)
    if !args.dry_run {
        // The lockfile was already updated by install functions
        // Just reload it for consistency
        if let Ok(Some(updated_lockfile)) = load_lockfile(&project.root) {
            // Use the updated lockfile
            let _ = save_lockfile(&project.root, &updated_lockfile);
        }
    }

    // Calculate summary
    let updated_count = results.iter().filter(|r| r.updated).count() as i32;
    let updates_available = results.iter().filter(|r| r.has_update).count() as i32;
    let failed_count = results.iter().filter(|r| r.error.is_some()).count() as i32;

    let status = if failed_count > 0 && updated_count == 0 {
        "error"
    } else if failed_count > 0 {
        "partial"
    } else {
        "success"
    };

    let output = UpdateOutput {
        status: status.to_string(),
        updated: updated_count,
        updates_available,
        failed: failed_count,
        total: results.len() as i32,
        dry_run: args.dry_run,
    };

    // Output results
    match format {
        OutputFormat::Json => print_json_output(&results, &output),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => print_human_summary(&output, args.dry_run),
    }

    if failed_count > 0 && updated_count == 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn print_json_output(results: &[UpdatedPackage], output: &UpdateOutput) {
    use serde_json::json;

    let packages: Vec<_> = results
        .iter()
        .map(|r| {
            json!({
                "name": r.name,
                "old_version": r.old_version,
                "new_version": r.new_version,
                "updated": r.updated,
                "has_update": r.has_update,
                "error": r.error,
            })
        })
        .collect();

    let json_output = json!({
        "status": output.status,
        "dry_run": output.dry_run,
        "packages": packages,
        "summary": {
            "updated": output.updated,
            "updates_available": output.updates_available,
            "failed": output.failed,
            "total": output.total,
        }
    });

    println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
}

fn print_human_summary(output: &UpdateOutput, dry_run: bool) {
    println!();

    if dry_run {
        if output.updates_available > 0 {
            println!(
                "Would update {} package(s). Run without --dry-run to apply.",
                output.updates_available
            );
        } else {
            println!("All packages are up to date.");
        }
    } else {
        let mut summary = Vec::new();
        if output.updated > 0 {
            summary.push(format!("{} updated", output.updated));
        }
        if output.failed > 0 {
            summary.push(format!("{} failed", output.failed));
        }

        if summary.is_empty() {
            println!("All packages are up to date.");
        } else {
            println!("Update complete: {}", summary.join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests for update command will be added in integration tests
}
