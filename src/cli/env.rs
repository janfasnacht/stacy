//! `stacy env` command implementation
//!
//! Shows the current environment configuration including:
//! - Stata binary location and how it was detected
//! - Project root and configuration
//! - Global cache path
//! - Adopath search order (constructed from lockfile)

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, EnvOutput};
use crate::error::Result;
use crate::executor::binary::detect_stata_binary;
use crate::packages::global_cache;
use crate::packages::lockfile::load_lockfile;
use crate::project::Project;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy env                               Show environment configuration")]
pub struct EnvArgs {
    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

/// Gathered environment information
struct EnvironmentInfo {
    stata_binary: Option<String>,
    stata_source: String,
    project_root: Option<PathBuf>,
    config_file: Option<PathBuf>,
    has_config: bool,
    cache_dir: PathBuf,
    log_dir: PathBuf,
    show_progress: bool,
    adopath: Vec<String>,
    package_count: usize,
}

pub fn execute(args: &EnvArgs) -> Result<()> {
    let format = args.format;
    let info = gather_environment_info()?;

    // Build output struct
    let output = EnvOutput {
        has_config: info.has_config,
        show_progress: info.show_progress,
        adopath_count: info.adopath.len(),
        cache_dir: info.cache_dir.clone(),
        log_dir: info.log_dir.clone(),
        project_root: info.project_root.clone(),
        stata_binary: info.stata_binary.as_ref().map(PathBuf::from),
        stata_source: info.stata_source.clone(),
    };

    match format {
        OutputFormat::Human => print_human_output(&info),
        OutputFormat::Json => print_json_output(&info),
        OutputFormat::Stata => println!("{}", output.to_stata()),
    }

    Ok(())
}

fn gather_environment_info() -> Result<EnvironmentInfo> {
    // Find project
    let project = Project::find()?;
    let config = project.as_ref().and_then(|p| p.config.as_ref());

    // Detect Stata binary with source tracking
    let (stata_binary, stata_source) = detect_stata_with_source();

    // Get global cache directory
    let cache_dir =
        global_cache::cache_dir().unwrap_or_else(|_| PathBuf::from("~/.cache/stacy/packages"));

    let log_dir = config
        .map(|c| c.run.log_dir.clone())
        .unwrap_or_else(|| PathBuf::from("logs"));

    let show_progress = config.map(|c| c.run.show_progress).unwrap_or(true);

    // Build adopath list from lockfile
    let (adopath, package_count) =
        build_adopath_from_lockfile(&project.as_ref().map(|p| p.root.clone()));

    // Check for config file
    let config_file = project.as_ref().map(|p| p.root.join("stacy.toml"));
    let has_config = config_file.as_ref().map(|p| p.exists()).unwrap_or(false);

    Ok(EnvironmentInfo {
        stata_binary,
        stata_source,
        project_root: project.as_ref().map(|p| p.root.clone()),
        config_file,
        has_config,
        cache_dir,
        log_dir,
        show_progress,
        adopath,
        package_count,
    })
}

fn detect_stata_with_source() -> (Option<String>, String) {
    // Check ENV first ($STATA_BINARY)
    if let Ok(binary) = std::env::var("STATA_BINARY") {
        if std::path::Path::new(&binary).exists() {
            return (Some(binary), "$STATA_BINARY env var".to_string());
        }
    }

    // Check user config (~/.config/stacy/config.toml)
    if let Ok(Some(user_config)) = crate::project::user_config::load_user_config() {
        if let Some(binary) = user_config.stata_binary {
            if std::path::Path::new(&binary).exists() {
                return (Some(binary), "~/.config/stacy/config.toml".to_string());
            }
        }
    }

    // Auto-detect
    match detect_stata_binary(None) {
        Ok(binary) => (Some(binary), "auto-detected".to_string()),
        Err(_) => (None, "not found".to_string()),
    }
}

fn build_adopath_from_lockfile(project_root: &Option<PathBuf>) -> (Vec<String>, usize) {
    let mut paths = Vec::new();
    let mut package_count = 0;

    // Load lockfile and add package paths from global cache.
    // Errors are shown as warnings since `stacy env` is informational (not executing Stata).
    if let Some(root) = project_root {
        match load_lockfile(root) {
            Ok(Some(lockfile)) => {
                // Sort packages alphabetically for deterministic output
                let mut sorted_packages: Vec<_> = lockfile.packages.iter().collect();
                sorted_packages.sort_by(|(a, _), (b, _)| a.cmp(b));

                for (name, entry) in sorted_packages {
                    if let Ok(pkg_path) = global_cache::package_path(name, &entry.version) {
                        paths.push(format!("{} ({})", pkg_path.display(), name));
                        package_count += 1;
                    }
                }
            }
            Ok(None) => {} // No lockfile — no packages to display
            Err(e) => {
                eprintln!("Warning: failed to load stacy.lock: {}", e);
            }
        }
    }

    // Reflects strict mode (the default for `stacy run`) — only BASE, no SITE/PERSONAL/PLUS/OLDPLACE.
    paths.push("BASE".to_string());

    (paths, package_count)
}

fn print_human_output(info: &EnvironmentInfo) {
    println!("stacy Environment");
    println!("===============");
    println!();

    // Stata section
    println!("Stata:");
    if let Some(ref binary) = info.stata_binary {
        println!("  Binary: {}", binary);
        println!("  Source: {}", info.stata_source);
    } else {
        println!("  Binary: NOT FOUND");
        println!("  Tip: Install Stata or set STATA_ENGINE environment variable");
    }
    println!();

    // Project section
    println!("Project:");
    if let Some(ref root) = info.project_root {
        println!("  Root: {}", root.display());
        if info.has_config {
            println!("  Config: stacy.toml (found)");
        } else {
            println!("  Config: stacy.toml (not found, using defaults)");
        }
        println!("  Packages: {} installed", info.package_count);
    } else {
        println!("  Root: (not in a project)");
        println!("  Tip: Run 'stacy init' to create a project");
    }
    println!();

    // Paths section
    println!("Paths:");
    println!("  Cache: {}", info.cache_dir.display());
    println!("  Logs: {}", info.log_dir.display());
    println!();

    // Adopath section
    println!("S_ADO (package search path):");
    for (i, path) in info.adopath.iter().enumerate() {
        println!("  {}. {}", i + 1, path);
    }
    println!();

    // Settings section
    println!("Settings:");
    println!("  Show progress: {}", info.show_progress);
    println!();

    println!("Run 'stacy doctor' for system diagnostics.");
}

fn print_json_output(info: &EnvironmentInfo) {
    use serde_json::json;

    let output = json!({
        "stata": {
            "binary": info.stata_binary,
            "source": info.stata_source,
        },
        "project": {
            "root": info.project_root.as_ref().map(|p| p.display().to_string()),
            "config_file": info.config_file.as_ref().map(|p| p.display().to_string()),
            "has_config": info.has_config,
            "package_count": info.package_count,
        },
        "paths": {
            "cache": info.cache_dir.display().to_string(),
            "logs": info.log_dir.display().to_string(),
        },
        "settings": {
            "show_progress": info.show_progress,
        },
        "s_ado": info.adopath,
        "adopath_count": info.adopath.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
