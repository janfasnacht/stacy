//! `stacy doctor` command implementation
//!
//! Runs system diagnostics to check for common issues:
//! - Stata installation and binary detection
//! - Project detection and configuration
//! - Write permissions
//! - Environment variables
//! - Error code cache status

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, DoctorOutput};
use crate::error::error_db::ErrorCodeCache;
use crate::error::Result;
use crate::executor::binary::detect_stata_binary;
use crate::packages::global_cache;
use crate::project::Project;
use crate::update_check;
use clap::Args;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy doctor                            Run system diagnostics
  stacy doctor --refresh                  Re-extract error codes from Stata")]
pub struct DoctorArgs {
    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,

    /// Re-extract error codes from Stata
    #[arg(long)]
    pub refresh: bool,
}

/// Result of a diagnostic check
struct DiagnosticResult {
    name: String,
    status: CheckStatus,
    message: String,
    suggestion: Option<String>,
}

#[derive(Clone, Copy)]
enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckStatus {
    fn as_str(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "pass",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
        }
    }
}

pub fn execute(args: &DoctorArgs) -> Result<()> {
    let format = args.format;

    // Handle --refresh before running checks
    if args.refresh {
        refresh_error_codes(format)?;
    }

    let checks = run_all_checks()?;

    // Build output struct
    let passed = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Pass))
        .count() as i32;
    let warnings = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Warn))
        .count() as i32;
    let failed = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Fail))
        .count() as i32;

    let output = DoctorOutput {
        ready: failed == 0,
        passed,
        warnings,
        failed,
        check_count: checks.len(),
    };

    match format {
        OutputFormat::Human => print_human_output(&checks),
        OutputFormat::Json => print_json_output(&checks),
        OutputFormat::Stata => println!("{}", output.to_stata()),
    }

    // Exit with error if any check failed
    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Run --refresh: extract error codes from Stata and save to cache
fn refresh_error_codes(format: OutputFormat) -> Result<()> {
    let binary = detect_stata_binary(None).map_err(|_| {
        crate::error::Error::Config(
            "Cannot refresh error codes: Stata binary not found".to_string(),
        )
    })?;

    if matches!(format, OutputFormat::Human) {
        eprintln!("Extracting error codes from Stata...");
    }

    let db = crate::error::extraction::extract_error_codes(&binary)?;
    let count = db.len();
    let version = db.stata_version.clone().unwrap_or_default();

    ErrorCodeCache::save(&db)?;

    if matches!(format, OutputFormat::Human) {
        eprintln!("Extracted {} error codes from Stata {}", count, version);
        eprintln!();
    }

    Ok(())
}

fn run_all_checks() -> Result<Vec<DiagnosticResult>> {
    let checks = vec![
        check_stata_binary(),
        check_project(),
        check_config(),
        check_cache_dir(),
        check_error_codes(),
        check_write_permissions(),
        check_env_vars(),
        check_update_status(),
    ];

    Ok(checks)
}

fn check_stata_binary() -> DiagnosticResult {
    match detect_stata_binary(None) {
        Ok(binary) => DiagnosticResult {
            name: "Stata Installation".to_string(),
            status: CheckStatus::Pass,
            message: format!("Found: {}", binary),
            suggestion: None,
        },
        Err(_) => DiagnosticResult {
            name: "Stata Installation".to_string(),
            status: CheckStatus::Fail,
            message: "Stata binary not found".to_string(),
            suggestion: Some(
                "Install Stata, set $STATA_BINARY env var, or configure ~/.config/stacy/config.toml"
                    .to_string(),
            ),
        },
    }
}

fn check_project() -> DiagnosticResult {
    match Project::find() {
        Ok(Some(project)) => DiagnosticResult {
            name: "Project Detection".to_string(),
            status: CheckStatus::Pass,
            message: format!("Project root: {}", project.root.display()),
            suggestion: None,
        },
        Ok(None) => DiagnosticResult {
            name: "Project Detection".to_string(),
            status: CheckStatus::Warn,
            message: "Not in a stacy project".to_string(),
            suggestion: Some("Run 'stacy init' to create a project".to_string()),
        },
        Err(e) => DiagnosticResult {
            name: "Project Detection".to_string(),
            status: CheckStatus::Fail,
            message: format!("Error: {}", e),
            suggestion: None,
        },
    }
}

fn check_config() -> DiagnosticResult {
    match Project::find() {
        Ok(Some(project)) => {
            if project.config.is_some() {
                DiagnosticResult {
                    name: "Configuration".to_string(),
                    status: CheckStatus::Pass,
                    message: "stacy.toml found and valid".to_string(),
                    suggestion: None,
                }
            } else {
                let config_path = project.root.join("stacy.toml");
                if config_path.exists() {
                    DiagnosticResult {
                        name: "Configuration".to_string(),
                        status: CheckStatus::Warn,
                        message: "stacy.toml exists but may be empty".to_string(),
                        suggestion: None,
                    }
                } else {
                    DiagnosticResult {
                        name: "Configuration".to_string(),
                        status: CheckStatus::Pass,
                        message: "No stacy.toml (using defaults)".to_string(),
                        suggestion: None,
                    }
                }
            }
        }
        Ok(None) => DiagnosticResult {
            name: "Configuration".to_string(),
            status: CheckStatus::Warn,
            message: "No project found (configuration not applicable)".to_string(),
            suggestion: None,
        },
        Err(e) => DiagnosticResult {
            name: "Configuration".to_string(),
            status: CheckStatus::Fail,
            message: format!("Error loading config: {}", e),
            suggestion: Some("Check stacy.toml syntax".to_string()),
        },
    }
}

fn check_cache_dir() -> DiagnosticResult {
    match global_cache::cache_dir() {
        Ok(cache_dir) => {
            if cache_dir.exists() {
                // Check if we can list packages
                match global_cache::list_cached_packages() {
                    Ok(packages) => DiagnosticResult {
                        name: "Package Cache".to_string(),
                        status: CheckStatus::Pass,
                        message: format!(
                            "{} ({} packages cached)",
                            cache_dir.display(),
                            packages.len()
                        ),
                        suggestion: None,
                    },
                    Err(_) => DiagnosticResult {
                        name: "Package Cache".to_string(),
                        status: CheckStatus::Warn,
                        message: format!("{} (unreadable)", cache_dir.display()),
                        suggestion: Some("Check cache directory permissions".to_string()),
                    },
                }
            } else {
                DiagnosticResult {
                    name: "Package Cache".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("{} (will be created on first install)", cache_dir.display()),
                    suggestion: None,
                }
            }
        }
        Err(_) => DiagnosticResult {
            name: "Package Cache".to_string(),
            status: CheckStatus::Warn,
            message: "Could not determine cache directory".to_string(),
            suggestion: Some("Home directory may not be set".to_string()),
        },
    }
}

fn check_error_codes() -> DiagnosticResult {
    match ErrorCodeCache::load() {
        Ok(Some(db)) => {
            let version_info = db.stata_version.as_deref().unwrap_or("unknown");
            DiagnosticResult {
                name: "Error Codes".to_string(),
                status: CheckStatus::Pass,
                message: format!(
                    "{} error codes (Stata {}, extracted {})",
                    db.len(),
                    version_info,
                    db.extracted_at
                ),
                suggestion: None,
            }
        }
        Ok(None) => DiagnosticResult {
            name: "Error Codes".to_string(),
            status: CheckStatus::Warn,
            message: "Not extracted yet".to_string(),
            suggestion: Some(
                "Run 'stacy doctor --refresh' to extract error codes from Stata".to_string(),
            ),
        },
        Err(e) => DiagnosticResult {
            name: "Error Codes".to_string(),
            status: CheckStatus::Warn,
            message: format!("Cache unreadable: {}", e),
            suggestion: Some("Run 'stacy doctor --refresh' to re-extract".to_string()),
        },
    }
}

fn check_write_permissions() -> DiagnosticResult {
    let cwd = std::env::current_dir().unwrap_or_default();
    let test_file = cwd.join(".stacy_write_test");

    match std::fs::write(&test_file, "test") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test_file);
            DiagnosticResult {
                name: "Write Permissions".to_string(),
                status: CheckStatus::Pass,
                message: "Can write to current directory".to_string(),
                suggestion: None,
            }
        }
        Err(_) => DiagnosticResult {
            name: "Write Permissions".to_string(),
            status: CheckStatus::Fail,
            message: "Cannot write to current directory".to_string(),
            suggestion: Some("Check directory permissions".to_string()),
        },
    }
}

fn check_env_vars() -> DiagnosticResult {
    let stata_engine = std::env::var("STATA_ENGINE").ok();

    if let Some(engine) = stata_engine {
        if std::path::Path::new(&engine).exists() {
            DiagnosticResult {
                name: "Environment".to_string(),
                status: CheckStatus::Pass,
                message: format!("STATA_ENGINE={} (exists)", engine),
                suggestion: None,
            }
        } else {
            DiagnosticResult {
                name: "Environment".to_string(),
                status: CheckStatus::Warn,
                message: format!("STATA_ENGINE={} (file not found)", engine),
                suggestion: Some("Check if the path is correct".to_string()),
            }
        }
    } else {
        DiagnosticResult {
            name: "Environment".to_string(),
            status: CheckStatus::Pass,
            message: "STATA_ENGINE not set (using auto-detection)".to_string(),
            suggestion: None,
        }
    }
}

fn check_update_status() -> DiagnosticResult {
    match update_check::load_cached_update() {
        Some(cache) if cache.update_available => {
            let method = update_check::detect_install_method();
            let instruction = update_check::upgrade_instruction(&method);
            DiagnosticResult {
                name: "Update Status".to_string(),
                status: CheckStatus::Warn,
                message: format!(
                    "v{} available (current: v{})",
                    cache.latest_version, cache.current_version
                ),
                suggestion: Some(format!("Run '{instruction}' to update")),
            }
        }
        Some(cache) => DiagnosticResult {
            name: "Update Status".to_string(),
            status: CheckStatus::Pass,
            message: format!("v{} (latest)", cache.current_version),
            suggestion: None,
        },
        None => DiagnosticResult {
            name: "Update Status".to_string(),
            status: CheckStatus::Pass,
            message: format!(
                "v{} (update check not yet cached)",
                env!("CARGO_PKG_VERSION")
            ),
            suggestion: None,
        },
    }
}

fn print_human_output(checks: &[DiagnosticResult]) {
    println!("stacy System Diagnostics");
    println!("======================");
    println!();

    for check in checks {
        println!("[{}] {}", check.status.icon(), check.name);
        println!("      {}", check.message);
        if let Some(ref suggestion) = check.suggestion {
            println!("      Tip: {}", suggestion);
        }
        println!();
    }

    // Summary
    let passed = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Pass))
        .count();
    let warned = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Warn))
        .count();
    let failed = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Fail))
        .count();

    println!(
        "Summary: {} passed, {} warnings, {} failed",
        passed, warned, failed
    );

    if failed == 0 {
        println!();
        println!("stacy is ready to use.");
    }
}

fn print_json_output(checks: &[DiagnosticResult]) {
    use serde_json::json;

    let results: Vec<_> = checks
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "status": c.status.as_str(),
                "message": c.message,
                "suggestion": c.suggestion,
            })
        })
        .collect();

    let passed = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Pass))
        .count();
    let warned = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Warn))
        .count();
    let failed = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Fail))
        .count();

    let output = json!({
        "checks": results,
        "summary": {
            "passed": passed,
            "warnings": warned,
            "failed": failed,
        },
        "ready": failed == 0,
        "check_count": checks.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
