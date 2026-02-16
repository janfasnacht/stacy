//! `stacy init` command implementation
//!
//! Creates a minimal stacy project:
//! - stacy.toml (configuration with commented defaults)
//! - .gitignore (ignore Stata-generated files)
//!
//! Other files (stacy.lock, ado/) are created on demand by `stacy install`.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, InitOutput};
use crate::error::Result;
use crate::project::structure::{
    create_project_structure, create_project_structure_with_metadata, has_project_markers,
    PackageSource, PackageToInstall, ProjectMetadata,
};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy init                              Initialize in current directory
  stacy init myproject                    Create new project directory
  stacy init --interactive                Interactive mode with prompts")]
pub struct InitArgs {
    /// Directory to initialize (default: current directory)
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Overwrite existing project files
    #[arg(long)]
    pub force: bool,

    /// Interactive mode: prompt for project details and packages
    #[arg(short, long)]
    pub interactive: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

pub fn execute(args: &InitArgs) -> Result<()> {
    let format = args.format;

    // Determine target path
    let path = args
        .path
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new("."));

    // Canonicalize if exists, otherwise use as-is
    let path = if path.exists() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        // For new directories, canonicalize the parent if it exists
        if let Some(parent) = path.parent() {
            if parent.exists() {
                parent
                    .canonicalize()
                    .map(|p| p.join(path.file_name().unwrap_or_default()))
                    .unwrap_or_else(|_| path.to_path_buf())
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        }
    };

    // Check if project already exists
    if !args.force && has_project_markers(&path) {
        let output = InitOutput {
            status: "error".to_string(),
            path: path.clone(),
            created_count: 0,
            package_count: 0,
        };

        match format {
            OutputFormat::Json => {
                println!(
                    r#"{{"status":"error","message":"Project already exists at {}. Use --force to overwrite.","path":"{}"}}"#,
                    path.display(),
                    path.display()
                );
            }
            OutputFormat::Stata => println!("{}", output.to_stata()),
            OutputFormat::Human => {
                eprintln!("Error: Project already exists at {}", path.display());
                eprintln!();
                eprintln!("Existing project markers found (stacy.toml, stacy.lock, or ado/).");
                eprintln!("Use --force to overwrite existing project files.");
            }
        }
        std::process::exit(1);
    }

    // Interactive or standard mode
    if args.interactive && format == OutputFormat::Human {
        execute_interactive(&path, args.force)
    } else {
        execute_standard(&path, args)
    }
}

fn execute_standard(path: &std::path::Path, args: &InitArgs) -> Result<()> {
    let format = args.format;

    // Create project structure
    let created = create_project_structure(path, args.force)?;

    // Build output struct
    let output = InitOutput {
        status: "success".to_string(),
        path: path.to_path_buf(),
        created_count: created.len(),
        package_count: 0,
    };

    // Output result
    match format {
        OutputFormat::Json => print_json_output(path, &created, &[]),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => print_human_output(path, &created, &[]),
    }

    Ok(())
}

fn execute_interactive(path: &std::path::Path, force: bool) -> Result<()> {
    use dialoguer::{Confirm, Input};

    println!("Initializing stacy project...\n");

    // Get default project name from directory
    let default_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-project")
        .to_string();

    // Prompt for project name
    let name: String = Input::new()
        .with_prompt("Project name")
        .default(default_name)
        .interact_text()
        .unwrap_or_default();

    // Prompt for authors (multiple)
    let mut authors = Vec::new();
    println!("\nAdd project authors (press Enter with empty input to finish):");
    loop {
        let prompt = if authors.is_empty() {
            "Author"
        } else {
            "Another author"
        };

        let author: String = Input::new()
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()
            .unwrap_or_default();

        if author.is_empty() {
            break;
        }
        authors.push(author);
    }

    // Prompt for packages
    let mut packages = Vec::new();
    println!();
    let add_packages = Confirm::new()
        .with_prompt("Add packages now?")
        .default(false)
        .interact()
        .unwrap_or(false);

    if add_packages {
        println!("\nAdd packages (press Enter with empty input to finish):");
        loop {
            let pkg_name: String = Input::new()
                .with_prompt("Package name")
                .allow_empty(true)
                .interact_text()
                .unwrap_or_default();

            if pkg_name.is_empty() {
                break;
            }

            // Ask for source
            let source_str: String = Input::new()
                .with_prompt("Source [ssc/github:user/repo]")
                .default("ssc".to_string())
                .interact_text()
                .unwrap_or_else(|_| "ssc".to_string());

            let source = parse_package_source(&source_str);
            let source_display = format_source_display(&source);

            packages.push(PackageToInstall {
                name: pkg_name.clone(),
                source,
            });

            println!("  Added {} ({})", pkg_name, source_display);
        }
    }

    // Build metadata
    let metadata = ProjectMetadata {
        name: if name.is_empty() { None } else { Some(name) },
        authors,
        description: None,
        url: None,
        packages,
    };

    // Create project structure with metadata
    let created = create_project_structure_with_metadata(path, &metadata, force)?;

    println!();
    print_human_output(path, &created, &metadata.packages);

    // If packages were added, offer to install them now
    if !metadata.packages.is_empty() {
        println!();
        let install_now = Confirm::new()
            .with_prompt("Install packages now?")
            .default(true)
            .interact()
            .unwrap_or(false);

        if install_now {
            println!();
            install_packages(path, &metadata.packages)?;
        }
    }

    Ok(())
}

fn install_packages(project_root: &std::path::Path, packages: &[PackageToInstall]) -> Result<()> {
    use crate::packages::installer::{install_from_ssc, install_package_github};
    use std::io::{self, Write};

    for pkg in packages {
        print!("Installing {}... ", pkg.name);
        io::stdout().flush().ok();

        let result = match &pkg.source {
            PackageSource::Ssc => install_from_ssc(&pkg.name, project_root, "production"),
            PackageSource::GitHub {
                user,
                repo,
                git_ref,
            } => install_package_github(
                &pkg.name,
                user,
                repo,
                git_ref.as_deref(),
                project_root,
                "production",
            ),
        };

        match result {
            Ok(install_result) => {
                println!(
                    "installed {} files (v{})",
                    install_result.files_installed.len(),
                    install_result.version
                );
            }
            Err(e) => {
                println!("failed: {}", e);
            }
        }
    }

    println!();
    println!("Packages installed to ado/ and recorded in stacy.lock");

    Ok(())
}

fn parse_package_source(source: &str) -> PackageSource {
    let source_lower = source.to_lowercase();

    if source_lower == "ssc" {
        return PackageSource::Ssc;
    }

    if source_lower.starts_with("github:") {
        let rest = &source[7..]; // Skip "github:"

        // Check for @ref suffix
        let (repo_part, git_ref) = if let Some(at_pos) = rest.find('@') {
            let repo = &rest[..at_pos];
            let ref_part = &rest[at_pos + 1..];
            (
                repo,
                if ref_part.is_empty() {
                    None
                } else {
                    Some(ref_part.to_string())
                },
            )
        } else {
            (rest, None)
        };

        // Parse user/repo
        if let Some(slash_pos) = repo_part.find('/') {
            let user = &repo_part[..slash_pos];
            let repo = &repo_part[slash_pos + 1..];

            if !user.is_empty() && !repo.is_empty() {
                return PackageSource::GitHub {
                    user: user.to_string(),
                    repo: repo.to_string(),
                    git_ref,
                };
            }
        }
    }

    // Default to SSC
    PackageSource::Ssc
}

fn format_source_display(source: &PackageSource) -> String {
    match source {
        PackageSource::Ssc => "SSC".to_string(),
        PackageSource::GitHub {
            user,
            repo,
            git_ref,
        } => {
            if let Some(ref r) = git_ref {
                format!("GitHub: {}/{}@{}", user, repo, r)
            } else {
                format!("GitHub: {}/{}", user, repo)
            }
        }
    }
}

fn print_human_output(path: &std::path::Path, created: &[String], packages: &[PackageToInstall]) {
    println!("Initialized stacy project in: {}", path.display());
    println!();

    if created.is_empty() {
        println!("All project files already exist.");
    } else {
        println!("Created:");
        for item in created {
            let description = match item.as_str() {
                "stacy.toml" => "Project configuration",
                ".gitignore" => "Git ignore rules",
                _ => "",
            };
            if description.is_empty() {
                println!("  {}", item);
            } else {
                println!("  {:12} - {}", item, description);
            }
        }
    }

    println!();
    println!("Next steps:");

    if !packages.is_empty() {
        println!(
            "  stacy install           - Install {} package(s) from config",
            packages.len()
        );
    }
    println!("  stacy run <script.do>   - Run a Stata script");
    println!("  stacy install <package> - Install a package (creates ado/, stacy.lock)");
    println!("  stacy env               - Show environment");
}

fn print_json_output(path: &std::path::Path, created: &[String], packages: &[PackageToInstall]) {
    use serde_json::json;

    let pkg_list: Vec<_> = packages
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "source": format_source_display(&p.source),
            })
        })
        .collect();

    let output = json!({
        "status": "success",
        "path": path.display().to_string(),
        "created": created,
        "created_count": created.len(),
        "packages": pkg_list,
        "package_count": pkg_list.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_init_minimal() {
        let temp = TempDir::new().unwrap();
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            force: false,
            interactive: false,
            format: OutputFormat::Human,
        };

        execute(&args).unwrap();

        // Verify only minimal files created
        assert!(temp.path().join("stacy.toml").exists());
        assert!(temp.path().join(".gitignore").exists());

        // Verify other files are NOT created (created on demand by install)
        assert!(!temp.path().join("stacy.lock").exists());
        assert!(!temp.path().join("ado").exists());
    }

    #[test]
    fn test_init_creates_new_directory() {
        let temp = TempDir::new().unwrap();
        let new_dir = temp.path().join("new-project");

        let args = InitArgs {
            path: Some(new_dir.clone()),
            force: false,
            interactive: false,
            format: OutputFormat::Human,
        };

        execute(&args).unwrap();

        assert!(new_dir.join("stacy.toml").exists());
        assert!(new_dir.join(".gitignore").exists());
    }

    #[test]
    fn test_init_force_overwrites() {
        let temp = TempDir::new().unwrap();

        // Create initial structure
        let args1 = InitArgs {
            path: Some(temp.path().to_path_buf()),
            force: false,
            interactive: false,
            format: OutputFormat::Human,
        };
        execute(&args1).unwrap();

        // Modify stacy.toml
        fs::write(temp.path().join("stacy.toml"), "modified").unwrap();

        // Re-init with force
        let args2 = InitArgs {
            path: Some(temp.path().to_path_buf()),
            force: true,
            interactive: false,
            format: OutputFormat::Human,
        };
        execute(&args2).unwrap();

        // Verify content was restored
        let content = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
        assert!(content.contains("[project]"));
    }

    #[test]
    fn test_parse_package_source_ssc() {
        let source = parse_package_source("ssc");
        assert!(matches!(source, PackageSource::Ssc));
    }

    #[test]
    fn test_parse_package_source_github() {
        let source = parse_package_source("github:sergiocorreia/reghdfe");
        match source {
            PackageSource::GitHub {
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
    fn test_parse_package_source_github_with_ref() {
        let source = parse_package_source("github:sergiocorreia/reghdfe@v6.0.0");
        match source {
            PackageSource::GitHub {
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
}
