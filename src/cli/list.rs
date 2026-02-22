//! `stacy list` command implementation
//!
//! Lists installed packages from the lockfile with their versions and sources.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, ListOutput, ListPackageInfo};
use crate::error::{Error, Result};
use crate::packages::lockfile::load_lockfile;
use crate::project::{PackageSource, Project};
use clap::Args;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy list                              List all packages
  stacy list --tree                       Group by dependency type")]
pub struct ListArgs {
    /// Group packages by dependency type (production, dev, test)
    #[arg(long)]
    pub tree: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

/// Package info for display
struct PackageInfo {
    name: String,
    version: String,
    source: String,
    group: String,
}

pub fn execute(args: &ListArgs) -> Result<()> {
    let format = args.format;

    // Find project
    let project = Project::find()?.ok_or_else(|| {
        Error::Config("Not in a stacy project. Run 'stacy init' first.".to_string())
    })?;

    // Load lockfile
    let lockfile = load_lockfile(&project.root)?;

    // Collect package info from lockfile
    let mut packages: Vec<PackageInfo> = Vec::new();

    if let Some(ref lf) = lockfile {
        for (name, entry) in &lf.packages {
            let source = match &entry.source {
                PackageSource::SSC { name: _ } => "ssc".to_string(),
                PackageSource::GitHub { repo, .. } => format!("github:{}", repo),
                PackageSource::Local { path } => format!("local:{}", path),
                PackageSource::Net { url } => format!("net:{}", url),
            };

            packages.push(PackageInfo {
                name: name.clone(),
                version: entry.version.clone(),
                source,
                group: entry.group.clone(),
            });
        }
    }

    // Sort by name
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    // Build output
    let output_packages: Vec<ListPackageInfo> = packages
        .iter()
        .map(|p| ListPackageInfo {
            name: p.name.clone(),
            version: p.version.clone(),
            source: p.source.clone(),
            group: p.group.as_str().to_string(),
        })
        .collect();

    let output = ListOutput {
        status: "success".to_string(),
        package_count: packages.len(),
        packages: output_packages,
    };

    match format {
        OutputFormat::Json => println!("{}", output.to_json()),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => {
            if packages.is_empty() {
                println!("No packages installed.");
                println!();
                println!("Use 'stacy add <package>' to add packages.");
            } else if args.tree {
                print_tree(&packages);
            } else {
                print_flat(&packages);
            }
        }
    }

    Ok(())
}

fn print_flat(packages: &[PackageInfo]) {
    // Calculate column widths
    let name_width = packages.iter().map(|p| p.name.len()).max().unwrap_or(10);
    let version_width = packages.iter().map(|p| p.version.len()).max().unwrap_or(10);

    for pkg in packages {
        let group_suffix = match pkg.group.as_str() {
            "production" => "",
            "dev" => " (dev)",
            "test" => " (test)",
            _ => "",
        };

        println!(
            "{:name_width$}  {:version_width$}  {}{}",
            pkg.name,
            pkg.version,
            pkg.source,
            group_suffix,
            name_width = name_width,
            version_width = version_width
        );
    }

    println!();
    println!("{} package(s) installed", packages.len());
}

fn print_tree(packages: &[PackageInfo]) {
    let mut prod: Vec<_> = packages
        .iter()
        .filter(|p| p.group == "production")
        .collect();
    let mut dev: Vec<_> = packages.iter().filter(|p| p.group == "dev").collect();
    let mut test: Vec<_> = packages.iter().filter(|p| p.group == "test").collect();

    prod.sort_by(|a, b| a.name.cmp(&b.name));
    dev.sort_by(|a, b| a.name.cmp(&b.name));
    test.sort_by(|a, b| a.name.cmp(&b.name));

    // Calculate column widths across all packages
    let name_width = packages.iter().map(|p| p.name.len()).max().unwrap_or(10);
    let version_width = packages.iter().map(|p| p.version.len()).max().unwrap_or(10);

    if !prod.is_empty() {
        let label = if prod.len() == 1 {
            "package"
        } else {
            "packages"
        };
        println!("production ({} {}):", prod.len(), label);
        for pkg in &prod {
            println!(
                "  {:name_width$}  {:version_width$}  {}",
                pkg.name,
                pkg.version,
                pkg.source,
                name_width = name_width,
                version_width = version_width
            );
        }
    }

    if !dev.is_empty() {
        if !prod.is_empty() {
            println!();
        }
        let label = if dev.len() == 1 {
            "package"
        } else {
            "packages"
        };
        println!("dev ({} {}):", dev.len(), label);
        for pkg in &dev {
            println!(
                "  {:name_width$}  {:version_width$}  {}",
                pkg.name,
                pkg.version,
                pkg.source,
                name_width = name_width,
                version_width = version_width
            );
        }
    }

    if !test.is_empty() {
        if !prod.is_empty() || !dev.is_empty() {
            println!();
        }
        let label = if test.len() == 1 {
            "package"
        } else {
            "packages"
        };
        println!("test ({} {}):", test.len(), label);
        for pkg in &test {
            println!(
                "  {:name_width$}  {:version_width$}  {}",
                pkg.name,
                pkg.version,
                pkg.source,
                name_width = name_width,
                version_width = version_width
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_info_sorting() {
        let mut packages = vec![
            PackageInfo {
                name: "zebra".to_string(),
                version: "1.0".to_string(),
                source: "ssc".to_string(),
                group: "production".to_string(),
            },
            PackageInfo {
                name: "alpha".to_string(),
                version: "2.0".to_string(),
                source: "ssc".to_string(),
                group: "production".to_string(),
            },
        ];

        packages.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(packages[0].name, "alpha");
        assert_eq!(packages[1].name, "zebra");
    }
}
