//! `stacy cache` command implementation
//!
//! Manages caches:
//! - Build cache for incremental builds
//! - Package cache for installed packages

use crate::cache::BuildCache;
use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CacheCleanOutput, CacheInfoOutput, CommandOutput};
use crate::error::Result;
use crate::packages::global_cache;
use crate::project::Project;
use clap::{Args, Subcommand};
use std::process;

#[derive(Args)]
#[command(about = "Manage the build cache", long_about = None)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommand,
}

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Remove cached build entries
    Clean(CleanArgs),
    /// Show build cache statistics
    Info(InfoArgs),
    /// Manage the global package cache
    Packages(PackagesArgs),
}

#[derive(Args)]
pub struct CleanArgs {
    /// Remove entries older than N days
    #[arg(long, value_name = "DAYS")]
    pub older_than: Option<u32>,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,

    /// Suppress output
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct InfoArgs {
    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

#[derive(Args)]
pub struct PackagesArgs {
    #[command(subcommand)]
    pub command: PackagesCommand,
}

#[derive(Subcommand)]
pub enum PackagesCommand {
    /// Show the global package cache path
    Path(PackagesPathArgs),
    /// List all cached packages
    List(PackagesListArgs),
    /// Clean the package cache
    Clean(PackagesCleanArgs),
}

#[derive(Args)]
pub struct PackagesPathArgs {
    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

#[derive(Args)]
pub struct PackagesListArgs {
    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

#[derive(Args)]
pub struct PackagesCleanArgs {
    /// Remove all cached packages (not just unused)
    #[arg(long)]
    pub all: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

/// Execute the cache command
pub fn execute(args: &CacheArgs) -> Result<()> {
    match &args.command {
        CacheCommand::Clean(clean_args) => execute_clean(clean_args),
        CacheCommand::Info(info_args) => execute_info(info_args),
        CacheCommand::Packages(pkg_args) => execute_packages(pkg_args),
    }
}

/// Execute `stacy cache packages` subcommands
fn execute_packages(args: &PackagesArgs) -> Result<()> {
    match &args.command {
        PackagesCommand::Path(path_args) => execute_packages_path(path_args),
        PackagesCommand::List(list_args) => execute_packages_list(list_args),
        PackagesCommand::Clean(clean_args) => execute_packages_clean(clean_args),
    }
}

/// Execute `stacy cache clean`
fn execute_clean(args: &CleanArgs) -> Result<()> {
    let project = Project::find()?;

    let project = match project {
        Some(p) => p,
        None => {
            if !args.quiet && args.format == OutputFormat::Human {
                eprintln!("Error: Not in a stacy project (no stacy.toml found)");
            }
            process::exit(10);
        }
    };

    let mut cache = BuildCache::load(&project.root)?;
    let initial_count = cache.len();
    let removed_count;

    if let Some(days) = args.older_than {
        // Remove entries older than N days
        removed_count = cache.remove_older_than(days);
        if removed_count > 0 {
            cache.save(&project.root)?;
        }
    } else {
        // Clear all entries
        removed_count = initial_count;
        cache.clear();
        // Delete the cache file entirely
        BuildCache::delete_file(&project.root)?;
    }

    let output = CacheCleanOutput {
        entries_removed: removed_count,
        entries_remaining: cache.len(),
        status: "success".to_string(),
    };

    match args.format {
        OutputFormat::Json => println!("{}", output.to_json()),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => {
            if !args.quiet {
                if removed_count == 0 {
                    println!("Cache is empty, nothing to remove.");
                } else if args.older_than.is_some() {
                    println!(
                        "Removed {} cached {} (older than {} days).",
                        removed_count,
                        if removed_count == 1 {
                            "entry"
                        } else {
                            "entries"
                        },
                        args.older_than.unwrap()
                    );
                    if !cache.is_empty() {
                        println!(
                            "{} {} remaining.",
                            cache.len(),
                            if cache.len() == 1 { "entry" } else { "entries" }
                        );
                    }
                } else {
                    println!(
                        "Removed {} cached {}.",
                        removed_count,
                        if removed_count == 1 {
                            "entry"
                        } else {
                            "entries"
                        }
                    );
                }
            }
        }
    }

    Ok(())
}

/// Execute `stacy cache info`
fn execute_info(args: &InfoArgs) -> Result<()> {
    let project = Project::find()?;

    let project = match project {
        Some(p) => p,
        None => {
            if args.format == OutputFormat::Human {
                eprintln!("Error: Not in a stacy project (no stacy.toml found)");
            }
            process::exit(10);
        }
    };

    let cache = BuildCache::load(&project.root)?;
    let cache_path = BuildCache::cache_path(&project.root);
    let cache_exists = cache_path.exists();

    // Calculate age statistics
    let mut oldest_age_secs: Option<u64> = None;
    let mut newest_age_secs: Option<u64> = None;

    for entry in cache.entries.values() {
        let age = entry.age_secs();
        oldest_age_secs = Some(oldest_age_secs.map_or(age, |o| o.max(age)));
        newest_age_secs = Some(newest_age_secs.map_or(age, |n| n.min(age)));
    }

    let output = CacheInfoOutput {
        entry_count: cache.len(),
        size_bytes: cache.size_bytes(),
        cache_path: cache_path.clone(),
        cache_exists,
        oldest_age_secs,
        newest_age_secs,
    };

    match args.format {
        OutputFormat::Json => println!("{}", output.to_json()),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => {
            println!("Build Cache Info");
            println!("────────────────────────────────────────");
            println!("  Location:  {}", cache_path.display());
            println!("  Exists:    {}", if cache_exists { "yes" } else { "no" });
            println!("  Entries:   {}", cache.len());

            if !cache.is_empty() {
                println!("  Size:      {}", format_bytes(cache.size_bytes()));

                if let Some(oldest) = oldest_age_secs {
                    println!("  Oldest:    {}", format_duration(oldest));
                }
                if let Some(newest) = newest_age_secs {
                    println!("  Newest:    {}", format_duration(newest));
                }
            }
        }
    }

    Ok(())
}

/// Format bytes in human-readable form
fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Format duration in human-readable form
fn format_duration(secs: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = MINUTE * 60;
    const DAY: u64 = HOUR * 24;

    if secs >= DAY {
        let days = secs / DAY;
        format!("{} {} ago", days, if days == 1 { "day" } else { "days" })
    } else if secs >= HOUR {
        let hours = secs / HOUR;
        format!(
            "{} {} ago",
            hours,
            if hours == 1 { "hour" } else { "hours" }
        )
    } else if secs >= MINUTE {
        let mins = secs / MINUTE;
        format!(
            "{} {} ago",
            mins,
            if mins == 1 { "minute" } else { "minutes" }
        )
    } else {
        format!(
            "{} {} ago",
            secs,
            if secs == 1 { "second" } else { "seconds" }
        )
    }
}

// =============================================================================
// Package Cache Commands
// =============================================================================

/// Execute `stacy cache packages path`
fn execute_packages_path(args: &PackagesPathArgs) -> Result<()> {
    let cache_path = global_cache::cache_dir()?;

    match args.format {
        OutputFormat::Human => {
            println!("{}", cache_path.display());
        }
        OutputFormat::Json => {
            use serde_json::json;
            let output = json!({
                "path": cache_path.display().to_string(),
                "exists": cache_path.exists(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Stata => {
            println!(
                "global stacy_cache_path \"{}\"",
                cache_path.display().to_string().replace('"', "'")
            );
        }
    }

    Ok(())
}

/// Execute `stacy cache packages list`
fn execute_packages_list(args: &PackagesListArgs) -> Result<()> {
    let packages = global_cache::list_cached_packages()?;

    match args.format {
        OutputFormat::Human => {
            if packages.is_empty() {
                println!("No packages cached.");
            } else {
                println!("Cached Packages");
                println!("───────────────────────────────────────────────────");
                for (name, version, path) in &packages {
                    println!("  {}@{}", name, version);
                    println!("    {}", path.display());
                }
                println!();
                println!("Total: {} package(s)", packages.len());

                // Show total size
                if let Ok(size) = global_cache::cache_size_bytes() {
                    println!("Size:  {}", format_bytes(size as usize));
                }
            }
        }
        OutputFormat::Json => {
            use serde_json::json;
            let pkg_list: Vec<_> = packages
                .iter()
                .map(|(name, version, path)| {
                    json!({
                        "name": name,
                        "version": version,
                        "path": path.display().to_string(),
                    })
                })
                .collect();

            let output = json!({
                "packages": pkg_list,
                "count": packages.len(),
                "size_bytes": global_cache::cache_size_bytes().unwrap_or(0),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Stata => {
            println!("scalar stacy_package_count = {}", packages.len());
            let names: Vec<_> = packages.iter().map(|(n, _, _)| n.as_str()).collect();
            let versions: Vec<_> = packages.iter().map(|(_, v, _)| v.as_str()).collect();
            println!(
                "global stacy_package_names \"{}\"",
                names.join(",").replace('"', "'")
            );
            println!(
                "global stacy_package_versions \"{}\"",
                versions.join(",").replace('"', "'")
            );
        }
    }

    Ok(())
}

/// Execute `stacy cache packages clean`
fn execute_packages_clean(args: &PackagesCleanArgs) -> Result<()> {
    if args.all {
        // Remove all cached packages
        let removed = global_cache::clean_cache()?;

        match args.format {
            OutputFormat::Human => {
                if removed == 0 {
                    println!("Package cache is empty, nothing to remove.");
                } else {
                    println!("Removed {} cached package(s).", removed);
                }
            }
            OutputFormat::Json => {
                use serde_json::json;
                let output = json!({
                    "status": "success",
                    "removed": removed,
                    "mode": "all",
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Stata => {
                println!("global stacy_status \"success\"");
                println!("scalar stacy_removed = {}", removed);
            }
        }
    } else {
        // Remove only unused packages (not referenced by any project)
        // For now, we just show a message since we'd need to scan for lockfiles
        match args.format {
            OutputFormat::Human => {
                println!("To remove all cached packages, use: stacy cache packages clean --all");
                println!();
                println!("Note: Packages are shared across projects. Use --all to clear the entire cache.");
            }
            OutputFormat::Json => {
                use serde_json::json;
                let output = json!({
                    "status": "info",
                    "message": "Use --all to clear the entire cache",
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Stata => {
                println!("global stacy_status \"info\"");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 2), "2.0 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30 seconds ago");
        assert_eq!(format_duration(1), "1 second ago");
        assert_eq!(format_duration(60), "1 minute ago");
        assert_eq!(format_duration(120), "2 minutes ago");
        assert_eq!(format_duration(3600), "1 hour ago");
        assert_eq!(format_duration(7200), "2 hours ago");
        assert_eq!(format_duration(86400), "1 day ago");
        assert_eq!(format_duration(172800), "2 days ago");
    }
}
