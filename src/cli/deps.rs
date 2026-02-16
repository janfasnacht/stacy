//! `stacy deps` command implementation
//!
//! Analyzes Stata scripts for dependencies (do/run/include statements)
//! and displays them as a tree or flat list.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{CommandOutput, DepsOutput};
use crate::deps::tree::{analyze_dependencies, DependencyTree};
use crate::error::Result;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy deps main.do                      Show dependency tree
  stacy deps main.do --flat               Show as flat list")]
pub struct DepsArgs {
    /// Stata script to analyze
    #[arg(value_name = "SCRIPT")]
    pub script: PathBuf,

    /// Show flat list instead of tree
    #[arg(long)]
    pub flat: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

pub fn execute(args: &DepsArgs) -> Result<()> {
    let format = args.format;

    // Verify script exists
    if !args.script.exists() {
        let output = DepsOutput {
            script: args.script.clone(),
            unique_count: 0,
            has_circular: false,
            has_missing: true,
            circular_count: 0,
            missing_count: 1,
        };

        match format {
            OutputFormat::Json => {
                println!(
                    r#"{{"error": "Script not found: {}"}}"#,
                    args.script.display()
                );
            }
            OutputFormat::Stata => println!("{}", output.to_stata()),
            OutputFormat::Human => eprintln!("Error: Script not found: {}", args.script.display()),
        }
        std::process::exit(3); // File error exit code
    }

    // Analyze dependencies
    let analysis = analyze_dependencies(&args.script)?;

    // Build output struct
    let output = DepsOutput {
        script: args.script.clone(),
        unique_count: analysis.tree.unique_count() as i32,
        has_circular: analysis.has_circular,
        has_missing: analysis.has_missing,
        circular_count: analysis.circular_paths.len(),
        missing_count: analysis.missing_paths.len(),
    };

    // Output result
    match format {
        OutputFormat::Json => print_json_output(&analysis.tree, &args.script)?,
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => {
            if args.flat {
                print_flat_output(&analysis.tree);
            } else {
                print_tree_output(&analysis.tree);
            }

            // Print summary
            println!();
            print_summary(&analysis.tree);

            // Warnings
            if analysis.has_circular {
                println!();
                eprintln!("Warning: Circular dependencies detected:");
                for path in &analysis.circular_paths {
                    eprintln!("  - {}", path.display());
                }
            }

            if analysis.has_missing {
                println!();
                eprintln!("Warning: Missing files:");
                for path in &analysis.missing_paths {
                    eprintln!("  - {}", path.display());
                }
            }
        }
    }

    Ok(())
}

fn print_tree_output(tree: &DependencyTree) {
    print!("{}", tree.format_tree());
}

fn print_flat_output(tree: &DependencyTree) {
    let flat = tree.flatten();

    if flat.is_empty() {
        println!("{}", tree.path.display());
        println!("  (no dependencies)");
        return;
    }

    println!("{}", tree.path.display());
    for dep in flat {
        let indent = "  ".repeat(dep.depth + 1);
        let suffix = if dep.is_circular {
            " (circular)"
        } else if !dep.exists {
            " (not found)"
        } else {
            ""
        };
        println!("{}{}{}", indent, dep.path.display(), suffix);
    }
}

fn print_summary(tree: &DependencyTree) {
    let unique = tree.unique_count();
    let circular_count = tree.circular_paths().len();
    let missing_count = tree.missing_paths().len();

    if unique == 0 {
        println!("No dependencies found");
    } else if unique == 1 {
        print!("Found 1 dependency");
    } else {
        print!("Found {} unique dependencies", unique);
    }

    if circular_count > 0 || missing_count > 0 {
        let mut issues = Vec::new();
        if circular_count > 0 {
            issues.push(format!("{} circular", circular_count));
        }
        if missing_count > 0 {
            issues.push(format!("{} missing", missing_count));
        }
        println!(" ({})", issues.join(", "));
    } else {
        println!();
    }
}

fn print_json_output(tree: &DependencyTree, script: &std::path::Path) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "script": script.display().to_string(),
        "dependencies": tree_to_json(tree),
        "summary": {
            "unique_count": tree.unique_count(),
            "has_circular": tree.has_circular(),
            "has_missing": tree.has_missing(),
            "circular_paths": tree.circular_paths().iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "missing_paths": tree.missing_paths().iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "circular_count": tree.circular_paths().len(),
            "missing_count": tree.missing_paths().len(),
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn tree_to_json(tree: &DependencyTree) -> serde_json::Value {
    use serde_json::json;

    let children: Vec<serde_json::Value> = tree.children.iter().map(tree_to_json).collect();

    json!({
        "path": tree.path.display().to_string(),
        "type": tree.dep_type.map(|t| t.to_string()),
        "exists": tree.exists,
        "is_circular": tree.is_circular,
        "line_number": tree.line_number,
        "children": children,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_execute_no_deps() {
        let temp = TempDir::new().unwrap();
        let script = create_test_file(temp.path(), "main.do", "display \"hello\"");

        let args = DepsArgs {
            script,
            flat: false,
            format: OutputFormat::Human,
        };

        // Should not panic
        execute(&args).unwrap();
    }

    #[test]
    fn test_execute_with_deps() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "helper.do", "display \"helper\"");
        let script = create_test_file(temp.path(), "main.do", "do \"helper.do\"");

        let args = DepsArgs {
            script,
            flat: false,
            format: OutputFormat::Human,
        };

        execute(&args).unwrap();
    }

    #[test]
    fn test_execute_flat() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "helper.do", "display \"helper\"");
        let script = create_test_file(temp.path(), "main.do", "do \"helper.do\"");

        let args = DepsArgs {
            script,
            flat: true,
            format: OutputFormat::Human,
        };

        execute(&args).unwrap();
    }
}
