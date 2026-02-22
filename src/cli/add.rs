//! `stacy add` command implementation
//!
//! Adds packages to stacy.toml dependencies, installs them, and updates the lockfile.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{AddOutput, CommandOutput};
use crate::error::{Error, Result};
use crate::packages::hints;
use crate::packages::installer::{
    install_from_local, install_from_net, install_from_ssc, install_package_github,
};
use crate::project::config::{load_config, write_config, DependencyGroup, PackageSpec};
use crate::project::Project;
use clap::Args;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy add estout                        Add package from SSC
  stacy add reghdfe ftools                Add multiple packages
  stacy add rdrobust --source github:rdpackages/rdrobust
                                          Add from GitHub
  stacy add grc1leg --source net:http://www.stata.com/users/vwiggins/
                                          Add from URL (net install)
  stacy add myutils --source local:./lib/myutils/
                                          Add from local directory
  stacy add texdoc --dev                  Add as dev dependency")]
pub struct AddArgs {
    /// Package names to add
    #[arg(value_name = "PACKAGE", required = true)]
    pub packages: Vec<String>,

    /// Package source: `ssc` (default), `github:user/repo[@ref]`, `net:URL`, or `local:path`
    #[arg(long, default_value = "ssc")]
    pub source: String,

    /// Add as development dependency
    #[arg(long, conflicts_with = "test")]
    pub dev: bool,

    /// Add as test dependency
    #[arg(long, conflicts_with = "dev")]
    pub test: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

/// Parsed package source for internal use
#[derive(Debug)]
enum ParsedSource {
    SSC,
    GitHub {
        user: String,
        repo: String,
        git_ref: Option<String>,
    },
    Net {
        url: String,
    },
    Local {
        path: String,
    },
}

/// Result of adding a single package
#[derive(Debug)]
struct AddedPackage {
    name: String,
    version: String,
    source: String,
    success: bool,
    error: Option<String>,
}

pub fn execute(args: &AddArgs) -> Result<()> {
    let format = args.format;

    // Find project (must exist for add)
    let project = Project::find()?.ok_or_else(|| {
        Error::Config("Not in a stacy project. Run 'stacy init' first.".to_string())
    })?;

    // Load config
    let mut config = load_config(&project.root)?
        .ok_or_else(|| Error::Config("No stacy.toml found. Run 'stacy init' first.".to_string()))?;

    // Parse source
    let source = parse_source(&args.source)?;
    let source_str = args.source.clone();

    // Determine dependency group
    let group = if args.test {
        DependencyGroup::Test
    } else if args.dev {
        DependencyGroup::Dev
    } else {
        DependencyGroup::Production
    };

    if format == OutputFormat::Human {
        let dep_type = group.as_str();
        println!(
            "Adding {} package(s) as {} dependency...",
            args.packages.len(),
            dep_type
        );
        println!();
    }

    let mut results: Vec<AddedPackage> = Vec::new();

    for package in &args.packages {
        let package_lower = package.to_lowercase();

        // Check if already in config
        if config.packages.has_package(&package_lower) {
            if format == OutputFormat::Human {
                println!("  {} is already in dependencies, skipping", package_lower);
            }
            results.push(AddedPackage {
                name: package_lower,
                version: "existing".to_string(),
                source: source_str.clone(),
                success: true,
                error: Some("Already in dependencies".to_string()),
            });
            continue;
        }

        // Install the package
        let install_result = match &source {
            ParsedSource::SSC => install_from_ssc(&package_lower, &project.root, group.as_str()),
            ParsedSource::GitHub {
                user,
                repo,
                git_ref,
            } => install_package_github(
                &package_lower,
                user,
                repo,
                git_ref.as_deref(),
                &project.root,
                group.as_str(),
            ),
            ParsedSource::Net { url } => {
                install_from_net(&package_lower, url, &project.root, group.as_str())
            }
            ParsedSource::Local { path } => {
                install_from_local(&package_lower, path, &project.root, group.as_str())
            }
        };

        match install_result {
            Ok(result) => {
                // Add to config
                let spec = PackageSpec::simple(source_str.clone());
                config
                    .packages
                    .add_dependency(package_lower.clone(), spec, group);

                if format == OutputFormat::Human {
                    println!("  + {} ({})", package_lower, result.version);
                    if let Some(hint) = hints::get_hint(&package_lower) {
                        println!("    hint: {}", hint);
                    }
                }

                results.push(AddedPackage {
                    name: package_lower,
                    version: result.version,
                    source: source_str.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                if format == OutputFormat::Human {
                    eprintln!("  x {} failed: {}", package_lower, e);
                }
                results.push(AddedPackage {
                    name: package_lower,
                    version: "".to_string(),
                    source: source_str.clone(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Write updated config
    write_config(&config, &project.root)?;

    // Calculate summary
    let added_count = results
        .iter()
        .filter(|r| r.success && r.error.is_none())
        .count() as i32;
    let skipped_count = results
        .iter()
        .filter(|r| r.success && r.error.is_some())
        .count() as i32;
    let failed_count = results.iter().filter(|r| !r.success).count() as i32;

    let status = if failed_count > 0 && added_count == 0 {
        "error"
    } else if failed_count > 0 {
        "partial"
    } else {
        "success"
    };

    let output = AddOutput {
        status: status.to_string(),
        added: added_count,
        skipped: skipped_count,
        failed: failed_count,
        total: results.len() as i32,
        group: group.as_str().to_string(),
    };

    // Output results
    match format {
        OutputFormat::Json => print_json_output(&results, &output),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => print_human_summary(&output),
    }

    if failed_count > 0 && added_count == 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn parse_source(source: &str) -> Result<ParsedSource> {
    let source_lower = source.to_lowercase();

    if source_lower == "ssc" {
        return Ok(ParsedSource::SSC);
    }

    if source_lower.starts_with("github:") {
        let rest = &source[7..]; // Skip "github:"

        // Check for @ref suffix
        let (repo_part, git_ref) = if let Some(at_pos) = rest.find('@') {
            let repo = &rest[..at_pos];
            let ref_part = &rest[at_pos + 1..];
            if ref_part.is_empty() {
                return Err(Error::Config(
                    "Empty git ref after @. Use github:user/repo or github:user/repo@tag"
                        .to_string(),
                ));
            }
            (repo, Some(ref_part.to_string()))
        } else {
            (rest, None)
        };

        // Parse user/repo
        if let Some(slash_pos) = repo_part.find('/') {
            let user = &repo_part[..slash_pos];
            let repo = &repo_part[slash_pos + 1..];

            if user.is_empty() || repo.is_empty() {
                return Err(Error::Config(format!(
                    "Invalid GitHub source: {}. Use github:user/repo",
                    source
                )));
            }

            return Ok(ParsedSource::GitHub {
                user: user.to_string(),
                repo: repo.to_string(),
                git_ref,
            });
        } else {
            return Err(Error::Config(format!(
                "Invalid GitHub source: {}. Use github:user/repo",
                source
            )));
        }
    }

    if source_lower.starts_with("net:") {
        let url = &source[4..]; // Skip "net:"
        if url.is_empty() {
            return Err(Error::Config(
                "Empty URL after net:. Use net:http://example.com/stata/".to_string(),
            ));
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(Error::Config(format!(
                "Invalid net source URL: {}. Must start with http:// or https://",
                url
            )));
        }
        return Ok(ParsedSource::Net {
            url: url.to_string(),
        });
    }

    if source_lower.starts_with("local:") {
        let path = &source[6..]; // Skip "local:"
        if path.is_empty() {
            return Err(Error::Config(
                "Empty path after local:. Use local:./lib/myutils/".to_string(),
            ));
        }
        return Ok(ParsedSource::Local {
            path: path.to_string(),
        });
    }

    Err(Error::Config(format!(
        "Unknown package source: '{}'. Use 'ssc', 'github:user/repo', 'net:URL', or 'local:path'",
        source
    )))
}

fn print_json_output(results: &[AddedPackage], output: &AddOutput) {
    use serde_json::json;

    let packages: Vec<_> = results
        .iter()
        .map(|r| {
            json!({
                "name": r.name,
                "version": r.version,
                "source": r.source,
                "success": r.success,
                "error": r.error,
            })
        })
        .collect();

    let json_output = json!({
        "status": output.status,
        "packages": packages,
        "summary": {
            "added": output.added,
            "skipped": output.skipped,
            "failed": output.failed,
            "total": output.total,
            "group": output.group,
        }
    });

    println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
}

fn print_human_summary(output: &AddOutput) {
    println!();
    let mut summary = Vec::new();
    if output.added > 0 {
        summary.push(format!("{} added", output.added));
    }
    if output.skipped > 0 {
        summary.push(format!("{} already present", output.skipped));
    }
    if output.failed > 0 {
        summary.push(format!("{} failed", output.failed));
    }

    if summary.is_empty() {
        println!("No packages added.");
    } else {
        let dep_type = format!("{} dependencies", output.group);
        println!("Updated {}: {}", dep_type, summary.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_source_ssc() {
        let result = parse_source("ssc").unwrap();
        assert!(matches!(result, ParsedSource::SSC));
    }

    #[test]
    fn test_parse_source_github() {
        let result = parse_source("github:sergiocorreia/reghdfe").unwrap();
        match result {
            ParsedSource::GitHub {
                user,
                repo,
                git_ref,
            } => {
                assert_eq!(user, "sergiocorreia");
                assert_eq!(repo, "reghdfe");
                assert!(git_ref.is_none());
            }
            _ => panic!("Expected GitHub source"),
        }
    }

    #[test]
    fn test_parse_source_github_with_ref() {
        let result = parse_source("github:sergiocorreia/reghdfe@v6.0.0").unwrap();
        match result {
            ParsedSource::GitHub {
                user,
                repo,
                git_ref,
            } => {
                assert_eq!(user, "sergiocorreia");
                assert_eq!(repo, "reghdfe");
                assert_eq!(git_ref, Some("v6.0.0".to_string()));
            }
            _ => panic!("Expected GitHub source"),
        }
    }

    #[test]
    fn test_parse_source_invalid() {
        assert!(parse_source("unknown").is_err());
        assert!(parse_source("github:invalid").is_err());
    }

    #[test]
    fn test_parse_source_net() {
        let result = parse_source("net:http://www.stata.com/users/vwiggins/").unwrap();
        match result {
            ParsedSource::Net { url } => {
                assert_eq!(url, "http://www.stata.com/users/vwiggins/");
            }
            _ => panic!("Expected Net source"),
        }
    }

    #[test]
    fn test_parse_source_net_https() {
        let result = parse_source("net:https://example.com/stata/").unwrap();
        match result {
            ParsedSource::Net { url } => {
                assert_eq!(url, "https://example.com/stata/");
            }
            _ => panic!("Expected Net source"),
        }
    }

    #[test]
    fn test_parse_source_net_invalid() {
        // Reject non-http/https
        assert!(parse_source("net:ftp://example.com/").is_err());
        // Reject empty URL
        assert!(parse_source("net:").is_err());
    }

    #[test]
    fn test_parse_source_local() {
        let result = parse_source("local:./lib/myutils/").unwrap();
        match result {
            ParsedSource::Local { path } => {
                assert_eq!(path, "./lib/myutils/");
            }
            _ => panic!("Expected Local source"),
        }
    }

    #[test]
    fn test_parse_source_local_absolute() {
        let result = parse_source("local:/tmp/stata-packages/").unwrap();
        match result {
            ParsedSource::Local { path } => {
                assert_eq!(path, "/tmp/stata-packages/");
            }
            _ => panic!("Expected Local source"),
        }
    }

    #[test]
    fn test_parse_source_local_invalid() {
        // Reject empty path
        assert!(parse_source("local:").is_err());
    }

    #[test]
    fn test_parse_source_error_message_includes_all_sources() {
        let err = parse_source("unknown").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("net:"));
        assert!(msg.contains("local:"));
        assert!(msg.contains("github:"));
        assert!(msg.contains("ssc"));
    }
}
