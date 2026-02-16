//! Task execution engine
//!
//! Handles sequential and parallel execution of tasks defined in the task graph.

use crate::error::{Error, Result};
use crate::executor::StataExecutor;
use crate::project::config::TaskDef;
use crate::task::TaskGraph;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Result of running a single script
#[derive(Debug, Clone)]
pub struct ScriptResult {
    /// Name of the task (for composite tasks) or script basename
    pub name: String,
    /// Path to the script that was executed
    pub script: std::path::PathBuf,
    /// Whether the script succeeded
    pub success: bool,
    /// Exit code from execution
    pub exit_code: i32,
    /// How long the script took
    pub duration: Duration,
    /// Path to the log file
    pub log_file: std::path::PathBuf,
}

/// Result of running a task (which may include multiple scripts)
#[derive(Debug)]
pub struct TaskResult {
    /// Name of the task
    pub name: String,
    /// Whether all scripts succeeded
    pub success: bool,
    /// Exit code (0 if all succeeded, first failure code otherwise)
    pub exit_code: i32,
    /// Total duration for all scripts
    pub duration: Duration,
    /// Results for individual scripts
    pub script_results: Vec<ScriptResult>,
}

impl TaskResult {
    /// Create a successful empty result
    pub fn empty(name: &str) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            exit_code: 0,
            duration: Duration::ZERO,
            script_results: vec![],
        }
    }

    /// Add a script result
    pub fn add_result(&mut self, result: ScriptResult) {
        if !result.success && self.success {
            self.success = false;
            self.exit_code = result.exit_code;
        }
        self.duration += result.duration;
        self.script_results.push(result);
    }

    /// Merge another TaskResult into this one
    pub fn merge(&mut self, other: TaskResult) {
        for result in other.script_results {
            self.add_result(result);
        }
    }

    /// Get the number of successful scripts
    pub fn success_count(&self) -> usize {
        self.script_results.iter().filter(|r| r.success).count()
    }

    /// Get the number of failed scripts
    pub fn failed_count(&self) -> usize {
        self.script_results.iter().filter(|r| !r.success).count()
    }
}

/// Task execution context
pub struct TaskExecutor<'a> {
    /// The task graph
    graph: &'a TaskGraph,
    /// Stata executor
    stata: &'a StataExecutor,
    /// Project root directory
    project_root: &'a Path,
    /// Arguments to pass to scripts (name -> value)
    args: HashMap<String, String>,
}

impl<'a> TaskExecutor<'a> {
    /// Create a new task executor
    pub fn new(graph: &'a TaskGraph, stata: &'a StataExecutor, project_root: &'a Path) -> Self {
        Self {
            graph,
            stata,
            project_root,
            args: HashMap::new(),
        }
    }

    /// Set arguments to pass to scripts
    pub fn with_args(mut self, args: HashMap<String, String>) -> Self {
        self.args = args;
        self
    }

    /// Execute a task by name
    pub fn execute(&self, task_name: &str) -> Result<TaskResult> {
        let task = self.graph.get_task(task_name).ok_or_else(|| {
            let similar = self.graph.find_similar(task_name);
            let msg = if similar.is_empty() {
                format!("Unknown task '{}'", task_name)
            } else {
                format!(
                    "Unknown task '{}'\n\nDid you mean '{}'?",
                    task_name,
                    similar.join("', '")
                )
            };
            Error::Config(msg)
        })?;

        self.execute_task(task_name, task)
    }

    /// Execute a task definition
    fn execute_task(&self, name: &str, task: &TaskDef) -> Result<TaskResult> {
        match task {
            TaskDef::Simple(script) => self.execute_script(name, script),
            TaskDef::Sequential(tasks) => self.execute_sequential(name, tasks),
            TaskDef::Complex(complex) => {
                if let Some(ref parallel) = complex.parallel {
                    self.execute_parallel(name, parallel)
                } else if let Some(ref script) = complex.script {
                    self.execute_script(name, script)
                } else {
                    // Empty task
                    Ok(TaskResult::empty(name))
                }
            }
        }
    }

    /// Execute a single script
    fn execute_script(&self, name: &str, script: &Path) -> Result<TaskResult> {
        let start = Instant::now();

        // Resolve script path relative to project root
        let script_path = if script.is_absolute() {
            script.to_path_buf()
        } else {
            self.project_root.join(script)
        };

        // Check that the script exists
        if !script_path.exists() {
            return Err(Error::Config(format!(
                "Task '{}': Script not found: {}",
                name,
                script_path.display()
            )));
        }

        // Run the script with Stata executor
        let result = self
            .stata
            .run_with_args(&script_path, Some(self.project_root), &self.args)?;

        let duration = start.elapsed();

        let script_result = ScriptResult {
            name: name.to_string(),
            script: script_path,
            success: result.success,
            exit_code: result.exit_code,
            duration,
            log_file: result.log_file,
        };

        let mut task_result = TaskResult::empty(name);
        task_result.add_result(script_result);
        Ok(task_result)
    }

    /// Execute tasks sequentially
    fn execute_sequential(&self, name: &str, tasks: &[String]) -> Result<TaskResult> {
        let mut result = TaskResult::empty(name);

        for task_name in tasks {
            let task = self.graph.get_task(task_name).ok_or_else(|| {
                Error::Config(format!(
                    "Task '{}' references unknown task '{}'",
                    name, task_name
                ))
            })?;

            let task_result = self.execute_task(task_name, task)?;

            // Merge results
            let failed = !task_result.success;
            result.merge(task_result);

            // Stop on first failure
            if failed {
                break;
            }
        }

        Ok(result)
    }

    /// Execute tasks in parallel using scoped threads
    fn execute_parallel(&self, name: &str, tasks: &[String]) -> Result<TaskResult> {
        if tasks.is_empty() {
            return Ok(TaskResult::empty(name));
        }

        // Collect task definitions first to avoid holding references across threads
        let task_defs: Vec<_> = tasks
            .iter()
            .map(|task_name| {
                self.graph
                    .get_task(task_name)
                    .map(|t| (task_name.clone(), t.clone()))
                    .ok_or_else(|| {
                        Error::Config(format!(
                            "Task '{}' references unknown task '{}'",
                            name, task_name
                        ))
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        // Use scoped threads for parallel execution
        let results = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        std::thread::scope(|s| {
            for (task_name, task_def) in task_defs {
                let results = Arc::clone(&results);
                let errors = Arc::clone(&errors);

                s.spawn(move || match self.execute_task(&task_name, &task_def) {
                    Ok(result) => {
                        results.lock().unwrap().push(result);
                    }
                    Err(e) => {
                        errors.lock().unwrap().push(e);
                    }
                });
            }
        });

        // Check for errors
        let errors = Arc::try_unwrap(errors).unwrap().into_inner().unwrap();
        if !errors.is_empty() {
            return Err(errors.into_iter().next().unwrap());
        }

        // Merge results
        let task_results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
        let mut final_result = TaskResult::empty(name);
        for task_result in task_results {
            final_result.merge(task_result);
        }

        Ok(final_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_result_empty() {
        let result = TaskResult::empty("test");
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.script_results.len(), 0);
        assert_eq!(result.success_count(), 0);
        assert_eq!(result.failed_count(), 0);
    }

    #[test]
    fn test_task_result_add_success() {
        let mut result = TaskResult::empty("test");
        result.add_result(ScriptResult {
            name: "script1".to_string(),
            script: std::path::PathBuf::from("test.do"),
            success: true,
            exit_code: 0,
            duration: Duration::from_secs(1),
            log_file: std::path::PathBuf::from("test.log"),
        });

        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.success_count(), 1);
        assert_eq!(result.failed_count(), 0);
    }

    #[test]
    fn test_task_result_add_failure() {
        let mut result = TaskResult::empty("test");
        result.add_result(ScriptResult {
            name: "script1".to_string(),
            script: std::path::PathBuf::from("test.do"),
            success: true,
            exit_code: 0,
            duration: Duration::from_secs(1),
            log_file: std::path::PathBuf::from("test.log"),
        });
        result.add_result(ScriptResult {
            name: "script2".to_string(),
            script: std::path::PathBuf::from("test2.do"),
            success: false,
            exit_code: 1,
            duration: Duration::from_secs(2),
            log_file: std::path::PathBuf::from("test2.log"),
        });

        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
        assert_eq!(result.success_count(), 1);
        assert_eq!(result.failed_count(), 1);
        assert_eq!(result.duration, Duration::from_secs(3));
    }

    #[test]
    fn test_task_result_merge() {
        let mut result1 = TaskResult::empty("test1");
        result1.add_result(ScriptResult {
            name: "script1".to_string(),
            script: std::path::PathBuf::from("test1.do"),
            success: true,
            exit_code: 0,
            duration: Duration::from_secs(1),
            log_file: std::path::PathBuf::from("test1.log"),
        });

        let mut result2 = TaskResult::empty("test2");
        result2.add_result(ScriptResult {
            name: "script2".to_string(),
            script: std::path::PathBuf::from("test2.do"),
            success: true,
            exit_code: 0,
            duration: Duration::from_secs(2),
            log_file: std::path::PathBuf::from("test2.log"),
        });

        result1.merge(result2);
        assert_eq!(result1.script_results.len(), 2);
        assert_eq!(result1.duration, Duration::from_secs(3));
    }
}
