//! `stacy outdated` command implementation
//!
//! Checks for package updates by comparing installed versions with latest available.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, OutdatedOutput, OutdatedPackageInfo};
use crate::error::{Error, Result};
use crate::packages::github::GitHubDownloader;
use crate::packages::lockfile::load_lockfile;
use crate::packages::ssc::SscDownloader;
use crate::project::{PackageSource, Project};
use clap::Args;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy outdated                          Check for package updates")]
pub struct OutdatedArgs {
    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

/// Information about an outdated package for internal use
struct OutdatedInfo {
    name: String,
    current: String,
    latest: String,
    source: String,
}

pub fn execute(args: &OutdatedArgs) -> Result<()> {
    let format = args.format;

    // Find project
    let project = Project::find()?.ok_or_else(|| {
        Error::Config("Not in a stacy project. Run 'stacy init' first.".to_string())
    })?;

    // Load lockfile
    let lockfile = load_lockfile(&project.root)?.ok_or_else(|| {
        Error::Config("No stacy.lock found. Use 'stacy add <package>' to add packages.".to_string())
    })?;

    if lockfile.packages.is_empty() {
        let output = OutdatedOutput {
            status: "success".to_string(),
            outdated_count: 0,
            total_count: 0,
            packages: vec![],
        };

        match format {
            OutputFormat::Json => println!("{}", output.to_json()),
            OutputFormat::Stata => println!("{}", output.to_stata()),
            OutputFormat::Human => println!("No packages installed."),
        }
        return Ok(());
    }

    if format == OutputFormat::Human {
        println!("Checking for updates...");
        println!();
    }

    let ssc_downloader = SscDownloader::new();
    let github_downloader = GitHubDownloader::new();
    let mut outdated: Vec<OutdatedInfo> = Vec::new();
    let mut checked_count = 0;

    for (name, entry) in &lockfile.packages {
        match &entry.source {
            PackageSource::SSC { name: pkg_name } => {
                checked_count += 1;

                // Get latest version from SSC
                match ssc_downloader.get_manifest(pkg_name) {
                    Ok(manifest) => {
                        let latest_version = manifest
                            .distribution_date
                            .clone()
                            .unwrap_or_else(crate::utils::date::today_yyyymmdd);

                        // Compare versions (simple string comparison for dates)
                        if latest_version != entry.version {
                            outdated.push(OutdatedInfo {
                                name: name.clone(),
                                current: entry.version.clone(),
                                latest: latest_version,
                                source: "ssc".to_string(),
                            });
                        }
                    }
                    Err(e) => {
                        if format == OutputFormat::Human {
                            eprintln!("  Warning: Could not check {}: {}", name, e);
                        }
                    }
                }
            }
            PackageSource::GitHub { repo, tag, .. } => {
                checked_count += 1;

                // Parse user/repo from repo string
                if let Some(slash_pos) = repo.find('/') {
                    let user = &repo[..slash_pos];
                    let repo_name = &repo[slash_pos + 1..];

                    match github_downloader.check_for_updates(user, repo_name, tag) {
                        Ok(info) => {
                            if info.has_update {
                                if let Some(latest) = info.latest_tag {
                                    outdated.push(OutdatedInfo {
                                        name: name.clone(),
                                        current: tag.clone(),
                                        latest,
                                        source: format!("github:{}", repo),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            if format == OutputFormat::Human {
                                eprintln!("  Warning: Could not check {}: {}", name, e);
                            }
                        }
                    }
                } else if format == OutputFormat::Human {
                    eprintln!("  Warning: Invalid repo format for {}: {}", name, repo);
                }
            }
            PackageSource::Local { path } => {
                if format == OutputFormat::Human {
                    eprintln!("  Skipping {} (local package at {})", name, path);
                }
                checked_count += 1;
            }
        }
    }

    // Sort by name
    outdated.sort_by(|a, b| a.name.cmp(&b.name));

    // Build output
    let output_packages: Vec<OutdatedPackageInfo> = outdated
        .iter()
        .map(|p| OutdatedPackageInfo {
            name: p.name.clone(),
            current: p.current.clone(),
            latest: p.latest.clone(),
            source: p.source.clone(),
        })
        .collect();

    let output = OutdatedOutput {
        status: "success".to_string(),
        outdated_count: outdated.len(),
        total_count: checked_count,
        packages: output_packages,
    };

    match format {
        OutputFormat::Json => println!("{}", output.to_json()),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => {
            if outdated.is_empty() {
                println!("All packages are up to date.");
            } else {
                // Calculate column widths
                let name_width = outdated.iter().map(|p| p.name.len()).max().unwrap_or(10);
                let current_width = outdated.iter().map(|p| p.current.len()).max().unwrap_or(10);
                let latest_width = outdated.iter().map(|p| p.latest.len()).max().unwrap_or(10);

                println!(
                    "{:name_width$}  {:current_width$}  {:latest_width$}  Source",
                    "Package",
                    "Current",
                    "Latest",
                    name_width = name_width,
                    current_width = current_width,
                    latest_width = latest_width
                );
                println!(
                    "{:name_width$}  {:current_width$}  {:latest_width$}  ------",
                    "-------",
                    "-------",
                    "------",
                    name_width = name_width,
                    current_width = current_width,
                    latest_width = latest_width
                );

                for pkg in &outdated {
                    println!(
                        "{:name_width$}  {:current_width$}  {:latest_width$}  {}",
                        pkg.name,
                        pkg.current,
                        pkg.latest,
                        pkg.source,
                        name_width = name_width,
                        current_width = current_width,
                        latest_width = latest_width
                    );
                }

                println!();
                let pkg_word = if outdated.len() == 1 {
                    "package has"
                } else {
                    "packages have"
                };
                println!("{} {} updates available.", outdated.len(), pkg_word);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outdated_info_sorting() {
        let mut packages = vec![
            OutdatedInfo {
                name: "zebra".to_string(),
                current: "1.0".to_string(),
                latest: "2.0".to_string(),
                source: "ssc".to_string(),
            },
            OutdatedInfo {
                name: "alpha".to_string(),
                current: "1.0".to_string(),
                latest: "1.5".to_string(),
                source: "ssc".to_string(),
            },
        ];

        packages.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(packages[0].name, "alpha");
        assert_eq!(packages[1].name, "zebra");
    }
}
