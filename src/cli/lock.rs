//! `stacy lock` command implementation
//!
//! Generates or verifies the lockfile from stacy.toml dependencies.
//! Downloads packages to calculate checksums for reproducible installs.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, LockOutput};
use crate::error::{Error, Result};
use crate::packages::github::GitHubDownloader;
use crate::packages::lockfile::{
    add_package, create_lockfile, create_package_entry, load_lockfile, save_lockfile,
};
use crate::packages::ssc::SscDownloader;
use crate::project::config::load_config;
use crate::project::{PackageSource, Project};
use clap::Args;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy lock                              Generate/update lockfile
  stacy lock --check                      Verify lockfile is in sync
  stacy lock --refresh                    Recompute checksums from installed packages")]
pub struct LockArgs {
    /// Verify lockfile matches stacy.toml without updating (exit 1 if out of sync)
    #[arg(long, conflicts_with = "refresh")]
    pub check: bool,

    /// Recompute checksums from the packages installed in the global cache
    /// (repairs entries recorded by older stacy versions, see #68)
    #[arg(long)]
    pub refresh: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

pub fn execute(args: &LockArgs) -> Result<()> {
    let format = args.format;

    // Find project
    let project = Project::find()?.ok_or_else(|| {
        Error::Config("Not in a stacy project. Run 'stacy init' first.".to_string())
    })?;

    // Load config
    let config = load_config(&project.root)?
        .ok_or_else(|| Error::Config("No stacy.toml found. Run 'stacy init' first.".to_string()))?;

    // Load existing lockfile (or create new one)
    let existing_lockfile = load_lockfile(&project.root)?;
    let mut lockfile = existing_lockfile.clone().unwrap_or_else(create_lockfile);

    // Get all packages from config
    let config_packages: Vec<_> = config.packages.all_packages().collect();

    if args.check {
        // Verify mode: check if lockfile matches config
        let mut in_sync = true;
        let mut missing_in_lock = Vec::new();
        let mut extra_in_lock = Vec::new();

        // Check for packages in config but not in lockfile
        for (name, _spec, _group) in &config_packages {
            if !lockfile.packages.contains_key(*name) {
                in_sync = false;
                missing_in_lock.push(name.as_str());
            }
        }

        // Check for packages in lockfile but not in config
        let config_names: std::collections::HashSet<&str> =
            config_packages.iter().map(|(n, _, _)| n.as_str()).collect();
        for name in lockfile.packages.keys() {
            if !config_names.contains(name.as_str()) {
                in_sync = false;
                extra_in_lock.push(name.as_str());
            }
        }

        let output = LockOutput {
            status: if in_sync { "success" } else { "error" }.to_string(),
            package_count: lockfile.packages.len(),
            updated: false,
            in_sync,
            failed: 0,
            error: if in_sync {
                None
            } else {
                Some("Lockfile is out of sync with stacy.toml".to_string())
            },
        };

        match format {
            OutputFormat::Json => println!("{}", output.to_json()),
            OutputFormat::Stata => println!("{}", output.to_stata()),
            OutputFormat::Human => {
                if in_sync {
                    println!("Lockfile is in sync with stacy.toml.");
                } else {
                    println!("Lockfile is out of sync with stacy.toml:");
                    if !missing_in_lock.is_empty() {
                        println!();
                        println!("  Missing from stacy.lock:");
                        for name in &missing_in_lock {
                            println!("    - {}", name);
                        }
                    }
                    if !extra_in_lock.is_empty() {
                        println!();
                        println!("  Extra in stacy.lock (not in stacy.toml):");
                        for name in &extra_in_lock {
                            println!("    - {}", name);
                        }
                    }
                    println!();
                    println!("Run 'stacy lock' to update the lockfile.");
                }
            }
        }

        if !in_sync {
            std::process::exit(1);
        }

        return Ok(());
    }

    // Update mode: resolve dependencies and update lockfile
    if format == OutputFormat::Human {
        println!("Resolving dependencies from stacy.toml...");
        println!();
    }

    let downloader = SscDownloader::new();
    let mut updated = false;
    let mut added_count = 0;
    let mut removed_count = 0;
    // Packages in stacy.toml that could not be recorded in the lockfile.
    let mut failures: Vec<String> = Vec::new();

    // Add packages from config that aren't in lockfile
    let github_downloader = GitHubDownloader::new();

    for (name, spec, group) in &config_packages {
        if lockfile.packages.contains_key(*name) {
            continue;
        }

        let source_str = spec.source();
        let group_str = group.as_str();

        if source_str == "ssc" {
            // Resolve SSC package - download full package for checksum
            match downloader.download_package(name) {
                Ok(download) => {
                    let version = download
                        .manifest
                        .distribution_date
                        .clone()
                        .unwrap_or_else(crate::utils::date::today_yyyymmdd);

                    let source = PackageSource::SSC {
                        name: name.to_string(),
                    };
                    let entry = create_package_entry(
                        &version,
                        source,
                        &download.package_checksum,
                        group_str,
                    );

                    add_package(&mut lockfile, name, entry);
                    updated = true;
                    added_count += 1;

                    if format == OutputFormat::Human {
                        println!("  + {} ({})", name, version);
                    }
                }
                Err(e) => {
                    failures.push(name.to_string());
                    if format == OutputFormat::Human {
                        eprintln!("  x could not resolve {}: {}", name, e);
                    }
                }
            }
        } else if let Some(rest) = source_str.strip_prefix("github:") {
            // Parse github:user/repo[@ref]
            let (repo_part, git_ref) = if let Some(at_pos) = rest.find('@') {
                (&rest[..at_pos], Some(&rest[at_pos + 1..]))
            } else {
                (rest, None)
            };

            // Parse user/repo
            if let Some(slash_pos) = repo_part.find('/') {
                let user = &repo_part[..slash_pos];
                let repo = &repo_part[slash_pos + 1..];

                // Download full package for checksum
                match github_downloader.download_package(name, user, repo, git_ref) {
                    Ok(download) => {
                        // Resolve commit SHA for reproducibility
                        let commit_sha =
                            github_downloader.resolve_commit_sha(user, repo, &download.git_ref);

                        let version =
                            download
                                .manifest
                                .distribution_date
                                .clone()
                                .unwrap_or_else(|| {
                                    if let Some(ref sha) = commit_sha {
                                        sha[..8].to_string()
                                    } else {
                                        git_ref.unwrap_or("main").to_string()
                                    }
                                });

                        let source = PackageSource::GitHub {
                            repo: repo_part.to_string(),
                            tag: git_ref.unwrap_or("main").to_string(),
                            commit: commit_sha,
                        };
                        let entry = create_package_entry(
                            &version,
                            source,
                            &download.package_checksum,
                            group_str,
                        );

                        add_package(&mut lockfile, name, entry);
                        updated = true;
                        added_count += 1;

                        if format == OutputFormat::Human {
                            println!(
                                "  + {} (github:{}@{})",
                                name,
                                repo_part,
                                git_ref.unwrap_or("main")
                            );
                        }
                    }
                    Err(e) => {
                        failures.push(name.to_string());
                        if format == OutputFormat::Human {
                            eprintln!("  x could not resolve {}: {}", name, e);
                        }
                    }
                }
            } else {
                failures.push(name.to_string());
                if format == OutputFormat::Human {
                    eprintln!(
                        "  x could not resolve {}: invalid GitHub source '{}'. Use github:user/repo",
                        name, source_str
                    );
                }
            }
        } else {
            // net: and local: packages carry no resolvable version — they are
            // recorded in the lockfile by `stacy add`, not by `stacy lock`.
            failures.push(name.to_string());
            if format == OutputFormat::Human {
                eprintln!(
                    "  x could not resolve {}: '{}' sources are recorded by `stacy add {} --source {}`",
                    name, source_str, name, source_str
                );
            }
        }
    }

    // Remove packages from lockfile that aren't in config
    let config_names: std::collections::HashSet<&str> =
        config_packages.iter().map(|(n, _, _)| n.as_str()).collect();
    let to_remove: Vec<String> = lockfile
        .packages
        .keys()
        .filter(|n: &&String| !config_names.contains(n.as_str()))
        .cloned()
        .collect();

    for name in to_remove {
        lockfile.packages.remove(&name);
        updated = true;
        removed_count += 1;

        if format == OutputFormat::Human {
            println!("  - {}", name);
        }
    }

    // Refresh mode: recompute checksums from the global cache so the
    // lockfile matches what `stacy install` verifies against. Repairs
    // entries recorded before duplicate manifest entries were deduped (#68).
    let mut refreshed_count = 0;
    if args.refresh {
        for (name, entry) in lockfile.packages.iter_mut() {
            let Ok(cache_dir) = crate::packages::global_cache::package_path(name, &entry.version)
            else {
                continue;
            };
            let Some(actual) = crate::packages::global_cache::hash_package_dir(&cache_dir) else {
                if format == OutputFormat::Human {
                    eprintln!("  Warning: {} not in cache, checksum left unchanged", name);
                }
                continue;
            };
            let new_checksum = format!("sha256:{}", actual);
            if entry.checksum.as_deref() != Some(new_checksum.as_str()) {
                entry.checksum = Some(new_checksum);
                updated = true;
                refreshed_count += 1;
                if format == OutputFormat::Human {
                    println!("  ~ {} (checksum recomputed)", name);
                }
            }
        }
    }

    // Save lockfile if updated
    if updated {
        save_lockfile(&project.root, &lockfile)?;
    }

    let error_message = if failures.is_empty() {
        None
    } else {
        Some(format!(
            "{} package(s) could not be resolved: {}",
            failures.len(),
            failures.join(", ")
        ))
    };

    let output = LockOutput {
        status: if error_message.is_some() {
            "error".to_string()
        } else {
            "success".to_string()
        },
        package_count: lockfile.packages.len(),
        updated,
        in_sync: failures.is_empty(),
        failed: failures.len(),
        error: error_message.clone(),
    };

    match format {
        OutputFormat::Json => println!("{}", output.to_json()),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => {
            println!();
            if updated {
                let mut summary = Vec::new();
                if added_count > 0 {
                    summary.push(format!("{} added", added_count));
                }
                if removed_count > 0 {
                    summary.push(format!("{} removed", removed_count));
                }
                if refreshed_count > 0 {
                    summary.push(format!("{} checksum(s) recomputed", refreshed_count));
                }
                println!(
                    "Updated stacy.lock: {} ({} total packages)",
                    summary.join(", "),
                    lockfile.packages.len()
                );
            } else if failures.is_empty() {
                println!(
                    "Lockfile is up to date ({} packages)",
                    lockfile.packages.len()
                );
            }
        }
    }

    if let Some(msg) = error_message {
        return Err(Error::Config(msg));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Integration tests in tests/integration_cli.rs
}
