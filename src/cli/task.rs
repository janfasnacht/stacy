//! CLI implementation for `stacy task` command
//!
//! Run defined tasks from stacy.toml's `[scripts]` section.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{
    CommandOutput, ScriptResultOutput, TaskInfo, TaskListOutput, TaskOutput,
};
use crate::error::{Error, Result};
use crate::executor::StataExecutor;
use crate::packages::lockfile::{load_lockfile, verify_lockfile_sync};
use crate::project::config::TaskDef;
use crate::project::Project;
use crate::task::executor::TaskExecutor;
use crate::task::{task_description, TaskGraph};
use clap::Args;
use std::collections::HashMap;
use std::process;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy task build                        Run the 'build' task
  stacy task analyze -- robust=1          Pass arguments to task scripts
  stacy task --list                       List available tasks
  stacy task build --frozen               Verify lockfile sync before running")]
pub struct TaskArgs {
    /// Task name to run
    #[arg(value_name = "TASK")]
    pub task: Option<String>,

    /// List available tasks
    #[arg(long, conflicts_with = "task")]
    pub list: bool,

    /// Fail if lockfile doesn't match stacy.toml (for CI reproducibility)
    #[arg(long)]
    pub frozen: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,

    /// Arguments to pass to scripts (after --)
    /// Format: key=value pairs
    #[arg(last = true)]
    pub args: Vec<String>,
}

pub fn execute(args: &TaskArgs) -> Result<()> {
    let format = args.format;

    // Find project
    let project = Project::find()?;
    let project = project.ok_or(Error::ProjectNotFound)?;

    // Get config, using default if none exists
    let config = project.config.clone().unwrap_or_default();

    // If --frozen, verify lockfile is in sync with manifest
    if args.frozen {
        let lockfile = load_lockfile(&project.root)?;

        match lockfile {
            Some(lockfile) => {
                let config_package_names: Vec<&str> = config
                    .packages
                    .all_packages()
                    .map(|(name, _, _)| name.as_str())
                    .collect();

                let sync_result = verify_lockfile_sync(&lockfile, &config_package_names);

                if !sync_result.in_sync {
                    let mut msg =
                        String::from("Lockfile out of sync with stacy.toml (--frozen mode)\n");

                    if !sync_result.missing_in_lock.is_empty() {
                        msg.push_str("\n  Missing from stacy.lock:\n");
                        for name in &sync_result.missing_in_lock {
                            msg.push_str(&format!("    - {}\n", name));
                        }
                    }

                    if !sync_result.extra_in_lock.is_empty() {
                        msg.push_str("\n  Extra in stacy.lock (not in stacy.toml):\n");
                        for name in &sync_result.extra_in_lock {
                            msg.push_str(&format!("    - {}\n", name));
                        }
                    }

                    msg.push_str("\n  hint: run `stacy lock` then commit stacy.lock");

                    return Err(Error::Config(msg));
                }
            }
            None => {
                // No lockfile exists - check if there are packages that should be locked
                let has_packages = config.packages.all_packages().next().is_some();
                if has_packages {
                    return Err(Error::Config(
                        "No stacy.lock found but stacy.toml has packages (--frozen mode)\n\n  hint: run `stacy lock` then commit stacy.lock".to_string()
                    ));
                }
                // No packages defined, no lockfile needed - this is fine
            }
        }
    }

    // Build task graph
    let graph = TaskGraph::from_config(&config.scripts)?;

    // Handle --list flag
    if args.list {
        return execute_list(&graph, format);
    }

    // Need a task name to run
    let task_name = args.task.as_ref().ok_or_else(|| {
        Error::Config("No task specified. Use --list to see available tasks.".to_string())
    })?;

    // Check if task exists
    if !graph.has_task(task_name) {
        let similar = graph.find_similar(task_name);
        let available: Vec<_> = graph.list_tasks().iter().map(|(n, _)| *n).collect();

        let msg = if similar.is_empty() {
            format!(
                "Unknown task '{}'\n\nAvailable tasks: {}",
                task_name,
                if available.is_empty() {
                    "none (add tasks to [scripts] section in stacy.toml)".to_string()
                } else {
                    available.join(", ")
                }
            )
        } else {
            format!(
                "Unknown task '{}'\n\nAvailable tasks: {}\nDid you mean '{}'?",
                task_name,
                available.join(", "),
                similar.join("', '")
            )
        };

        if format.is_machine_readable() {
            let output = TaskOutput {
                task_name: task_name.clone(),
                success: false,
                exit_code: 5,
                duration_secs: 0.0,
                script_count: 0,
                success_count: 0,
                failed_count: 0,
                scripts: vec![],
            };
            match format {
                OutputFormat::Json => println!("{}", output.to_json()),
                OutputFormat::Stata => println!("{}", output.to_stata()),
                OutputFormat::Human => {}
            }
        } else {
            eprintln!("Error: {}", msg);
        }
        process::exit(5); // Internal error
    }

    // Parse arguments
    let task_args = parse_task_args(&args.args)?;

    // Create Stata executor
    let executor =
        StataExecutor::try_new(None, crate::executor::verbosity::Verbosity::PipedDefault)?;

    // Create task executor
    let task_executor = TaskExecutor::new(&graph, &executor, &project.root).with_args(task_args);

    // Run the task
    let result = task_executor.execute(task_name)?;

    // Build output
    let output = TaskOutput {
        task_name: task_name.clone(),
        success: result.success,
        exit_code: result.exit_code,
        duration_secs: result.duration.as_secs_f64(),
        script_count: result.script_results.len(),
        success_count: result.success_count(),
        failed_count: result.failed_count(),
        scripts: result
            .script_results
            .iter()
            .map(|r| ScriptResultOutput {
                name: r.name.clone(),
                script: r.script.clone(),
                success: r.success,
                exit_code: r.exit_code,
                duration_secs: r.duration.as_secs_f64(),
            })
            .collect(),
    };

    // Output results
    match format {
        OutputFormat::Json => {
            println!("{}", output.to_json());
        }
        OutputFormat::Stata => {
            println!("{}", output.to_stata());
        }
        OutputFormat::Human => {
            if result.success {
                println!(
                    "\x1b[32mPASS\x1b[0m  Task '{}'  ({:.2}s)",
                    task_name,
                    result.duration.as_secs_f64()
                );
                if result.script_results.len() > 1 {
                    println!(
                        "      {} scripts executed successfully",
                        result.script_results.len()
                    );
                }
            } else {
                eprintln!(
                    "\x1b[31mFAIL\x1b[0m  Task '{}'  ({:.2}s)",
                    task_name,
                    result.duration.as_secs_f64()
                );
                eprintln!(
                    "      {}/{} scripts succeeded",
                    result.success_count(),
                    result.script_results.len()
                );

                // Show which script failed
                for script_result in &result.script_results {
                    if !script_result.success {
                        eprintln!(
                            "      FAIL  {} (exit code {})",
                            script_result.script.display(),
                            script_result.exit_code
                        );
                    }
                }
            }
        }
    }

    process::exit(result.exit_code);
}

/// Execute --list to show available tasks
fn execute_list(graph: &TaskGraph, format: OutputFormat) -> Result<()> {
    let tasks = graph.list_tasks();

    match format {
        OutputFormat::Json => {
            let output = TaskListOutput {
                task_count: tasks.len(),
                tasks: tasks
                    .iter()
                    .map(|(name, def): &(&str, &TaskDef)| TaskInfo {
                        name: name.to_string(),
                        description: task_description(def),
                    })
                    .collect(),
            };
            println!("{}", output.to_json());
        }
        OutputFormat::Stata => {
            let output = TaskListOutput {
                task_count: tasks.len(),
                tasks: tasks
                    .iter()
                    .map(|(name, def): &(&str, &TaskDef)| TaskInfo {
                        name: name.to_string(),
                        description: task_description(def),
                    })
                    .collect(),
            };
            println!("{}", output.to_stata());
        }
        OutputFormat::Human => {
            if tasks.is_empty() {
                println!("No tasks defined.");
                println!();
                println!("Add tasks to the [scripts] section of stacy.toml:");
                println!();
                println!("  [scripts]");
                println!("  clean = \"src/01_clean.do\"");
                println!("  analyze = \"src/02_analyze.do\"");
                println!("  all = [\"clean\", \"analyze\"]");
            } else {
                println!("Available tasks:");
                println!();
                for (name, def) in &tasks {
                    println!("  {:<15} {}", name, task_description(def));
                }
            }
        }
    }

    Ok(())
}

/// Parse task arguments from command line
///
/// Expected format: `key=value` pairs
fn parse_task_args(args: &[String]) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            result.insert(key.to_string(), value.to_string());
        } else {
            return Err(Error::Config(format!(
                "Invalid argument '{}'. Expected format: key=value",
                arg
            )));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_task_args_empty() {
        let args = parse_task_args(&[]).unwrap();
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_task_args_single() {
        let args = parse_task_args(&["robust=1".to_string()]).unwrap();
        assert_eq!(args.get("robust"), Some(&"1".to_string()));
    }

    #[test]
    fn test_parse_task_args_multiple() {
        let args = parse_task_args(&["robust=1".to_string(), "cluster=state".to_string()]).unwrap();
        assert_eq!(args.get("robust"), Some(&"1".to_string()));
        assert_eq!(args.get("cluster"), Some(&"state".to_string()));
    }

    #[test]
    fn test_parse_task_args_with_equals_in_value() {
        let args = parse_task_args(&["expr=x=1".to_string()]).unwrap();
        assert_eq!(args.get("expr"), Some(&"x=1".to_string()));
    }

    #[test]
    fn test_parse_task_args_invalid() {
        let result = parse_task_args(&["invalid".to_string()]);
        assert!(result.is_err());
    }
}
