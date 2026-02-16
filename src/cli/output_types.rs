//! Output types for CLI commands
//!
//! These types ensure consistent output across all commands in JSON and Stata formats.
//! Each command should construct its output struct and use the trait methods for serialization.

// The vec![] macro suggestion makes the code less readable here since we're building
// structured output with many fields. Sequential pushes are clearer.
#![allow(clippy::vec_init_then_push)]

use crate::cli::output_format::{
    format_stata_local, format_stata_scalar_bool, format_stata_scalar_float,
    format_stata_scalar_int, format_stata_scalar_usize,
};
use serde::Serialize;
use std::path::PathBuf;

/// Trait for command outputs that can be serialized to JSON or Stata format
pub trait CommandOutput: Serialize {
    /// Get the command name
    fn command_name(&self) -> &'static str;

    /// Serialize to pretty-printed JSON string
    fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Serialize to Stata-native commands that can be directly executed
    fn to_stata(&self) -> String;
}

// =============================================================================
// RunOutput
// =============================================================================

/// Output for `stacy run` command
#[derive(Debug, Serialize)]
pub struct RunOutput {
    /// Execution time in seconds
    pub duration_secs: f64,
    /// Number of errors detected
    pub error_count: usize,
    /// Exit code (0=success)
    pub exit_code: i32,
    /// Whether script succeeded (1=yes, 0=no)
    pub success: bool,
    /// Path to log file
    pub log_file: PathBuf,
    /// Path to script
    pub script: PathBuf,
    /// 'file' or 'inline'
    pub source: String,
}

impl CommandOutput for RunOutput {
    fn command_name(&self) -> &'static str {
        "run"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy run output".to_string());
        lines.push(format_stata_scalar_bool("success", self.success));
        lines.push(format_stata_scalar_int("exit_code", self.exit_code as i64));
        lines.push(format_stata_scalar_float(
            "duration_secs",
            self.duration_secs,
        ));
        lines.push(format_stata_scalar_usize("error_count", self.error_count));
        lines.push(format_stata_local("source", &self.source));
        lines.push(format_stata_local(
            "script",
            &self.script.display().to_string(),
        ));
        lines.push(format_stata_local(
            "log_file",
            &self.log_file.display().to_string(),
        ));
        lines.join("\n")
    }
}

// =============================================================================
// ParallelRunOutput
// =============================================================================

/// Output for `stacy run --parallel` command
#[derive(Debug, Serialize)]
pub struct ParallelRunOutput {
    /// Whether all scripts succeeded
    pub success: bool,
    /// Highest exit code (for exit code strategy)
    pub exit_code: i32,
    /// Total wall-clock time in seconds
    pub duration_secs: f64,
    /// Whether parallel execution was used
    pub parallel: bool,
    /// Number of parallel jobs used
    pub jobs: usize,
    /// Number of scripts passed
    pub passed: usize,
    /// Number of scripts failed
    pub failed: usize,
    /// Total number of scripts
    pub total: usize,
    /// Individual script results
    pub scripts: Vec<ScriptRunResult>,
}

impl CommandOutput for ParallelRunOutput {
    fn command_name(&self) -> &'static str {
        "run-parallel"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy run --parallel output".to_string());
        lines.push(format_stata_scalar_bool("success", self.success));
        lines.push(format_stata_scalar_int("exit_code", self.exit_code as i64));
        lines.push(format_stata_scalar_float(
            "duration_secs",
            self.duration_secs,
        ));
        lines.push(format_stata_scalar_bool("parallel", self.parallel));
        lines.push(format_stata_scalar_usize("jobs", self.jobs));
        lines.push(format_stata_scalar_usize("passed", self.passed));
        lines.push(format_stata_scalar_usize("failed", self.failed));
        lines.push(format_stata_scalar_usize("total", self.total));
        lines.join("\n")
    }
}

// =============================================================================
// BenchOutput
// =============================================================================

/// Output for `stacy bench` command
#[derive(Debug, Serialize)]
pub struct BenchOutput {
    /// Path to the benchmarked script
    pub script: PathBuf,
    /// Number of measured runs
    pub measured_runs: usize,
    /// Number of warmup runs
    pub warmup_runs: usize,
    /// Mean execution time in seconds
    pub mean_secs: f64,
    /// Median execution time in seconds
    pub median_secs: f64,
    /// Minimum execution time in seconds
    pub min_secs: f64,
    /// Maximum execution time in seconds
    pub max_secs: f64,
    /// Standard deviation in seconds
    pub stddev_secs: f64,
    /// Whether all runs succeeded
    pub success: bool,
}

impl CommandOutput for BenchOutput {
    fn command_name(&self) -> &'static str {
        "bench"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy bench output".to_string());
        lines.push(format_stata_local(
            "script",
            &self.script.display().to_string(),
        ));
        lines.push(format_stata_scalar_usize(
            "measured_runs",
            self.measured_runs,
        ));
        lines.push(format_stata_scalar_usize("warmup_runs", self.warmup_runs));
        lines.push(format_stata_scalar_float("mean_secs", self.mean_secs));
        lines.push(format_stata_scalar_float("median_secs", self.median_secs));
        lines.push(format_stata_scalar_float("min_secs", self.min_secs));
        lines.push(format_stata_scalar_float("max_secs", self.max_secs));
        lines.push(format_stata_scalar_float("stddev_secs", self.stddev_secs));
        lines.push(format_stata_scalar_bool("success", self.success));
        lines.join("\n")
    }
}

// =============================================================================
// CacheCleanOutput
// =============================================================================

/// Output for `stacy cache clean` command
#[derive(Debug, Serialize)]
pub struct CacheCleanOutput {
    /// Number of entries removed
    pub entries_removed: usize,
    /// Number of entries remaining
    pub entries_remaining: usize,
    /// 'success' or 'error'
    pub status: String,
}

impl CommandOutput for CacheCleanOutput {
    fn command_name(&self) -> &'static str {
        "cache-clean"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy cache clean output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_scalar_usize(
            "entries_removed",
            self.entries_removed,
        ));
        lines.push(format_stata_scalar_usize(
            "entries_remaining",
            self.entries_remaining,
        ));
        lines.join("\n")
    }
}

// =============================================================================
// CacheInfoOutput
// =============================================================================

/// Output for `stacy cache info` command
#[derive(Debug, Serialize)]
pub struct CacheInfoOutput {
    /// Number of cached entries
    pub entry_count: usize,
    /// Approximate size in bytes
    pub size_bytes: usize,
    /// Path to cache file
    pub cache_path: PathBuf,
    /// Whether cache file exists
    pub cache_exists: bool,
    /// Age of oldest entry in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_age_secs: Option<u64>,
    /// Age of newest entry in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_age_secs: Option<u64>,
}

impl CommandOutput for CacheInfoOutput {
    fn command_name(&self) -> &'static str {
        "cache-info"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy cache info output".to_string());
        lines.push(format_stata_scalar_usize("entry_count", self.entry_count));
        lines.push(format_stata_scalar_usize("size_bytes", self.size_bytes));
        lines.push(format_stata_local(
            "cache_path",
            &self.cache_path.display().to_string(),
        ));
        lines.push(format_stata_scalar_bool("cache_exists", self.cache_exists));
        if let Some(oldest) = self.oldest_age_secs {
            lines.push(format_stata_scalar_usize(
                "oldest_age_secs",
                oldest as usize,
            ));
        }
        if let Some(newest) = self.newest_age_secs {
            lines.push(format_stata_scalar_usize(
                "newest_age_secs",
                newest as usize,
            ));
        }
        lines.join("\n")
    }
}

// =============================================================================
// CacheHitOutput
// =============================================================================

/// Output for `stacy run --cache` when cache hit occurs
#[derive(Debug, Serialize)]
pub struct CacheHitOutput {
    /// Whether the cached execution was successful
    pub success: bool,
    /// Cached exit code (0=success)
    pub exit_code: i32,
    /// Original execution time in seconds
    pub duration_secs: f64,
    /// Number of cached errors
    pub error_count: usize,
    /// Always 'cache' for cache hits
    pub source: String,
    /// Path to script
    pub script: PathBuf,
    /// When this entry was cached
    #[serde(with = "system_time_serde")]
    pub cached_at: std::time::SystemTime,
}

impl CommandOutput for CacheHitOutput {
    fn command_name(&self) -> &'static str {
        "run-cache"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy run --cache output (cache hit)".to_string());
        lines.push(format_stata_scalar_bool("success", self.success));
        lines.push(format_stata_scalar_int("exit_code", self.exit_code as i64));
        lines.push(format_stata_scalar_float(
            "duration_secs",
            self.duration_secs,
        ));
        lines.push(format_stata_scalar_usize("error_count", self.error_count));
        lines.push(format_stata_local("source", &self.source));
        lines.push(format_stata_local(
            "script",
            &self.script.display().to_string(),
        ));
        lines.push(format_stata_scalar_bool("cache_hit", true));
        lines.join("\n")
    }
}

/// Serde module for SystemTime (serialize as Unix timestamp)
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        serializer.serialize_u64(duration.as_secs())
    }

    #[allow(dead_code)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

/// Individual script result within a parallel run
#[derive(Debug, Serialize, Clone)]
pub struct ScriptRunResult {
    /// Path to the script
    pub script: PathBuf,
    /// Whether the script succeeded
    pub success: bool,
    /// Exit code
    pub exit_code: i32,
    /// Execution time in seconds
    pub duration_secs: f64,
    /// Path to log file
    pub log_file: PathBuf,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

// =============================================================================
// DoctorOutput
// =============================================================================

/// Output for `stacy doctor` command
#[derive(Debug, Serialize)]
pub struct DoctorOutput {
    /// Total number of checks
    pub check_count: usize,
    /// Number of failed checks
    pub failed: i32,
    /// Number of checks passed
    pub passed: i32,
    /// System is ready to use (1=yes, 0=no)
    pub ready: bool,
    /// Number of warnings
    pub warnings: i32,
}

impl CommandOutput for DoctorOutput {
    fn command_name(&self) -> &'static str {
        "doctor"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy doctor output".to_string());
        lines.push(format_stata_scalar_bool("ready", self.ready));
        lines.push(format_stata_scalar_int("passed", self.passed as i64));
        lines.push(format_stata_scalar_int("warnings", self.warnings as i64));
        lines.push(format_stata_scalar_int("failed", self.failed as i64));
        lines.push(format_stata_scalar_usize("check_count", self.check_count));
        lines.join("\n")
    }
}

// =============================================================================
// EnvOutput
// =============================================================================

/// Output for `stacy env` command
#[derive(Debug, Serialize)]
pub struct EnvOutput {
    /// Number of adopath entries
    pub adopath_count: usize,
    /// stacy.toml exists (1=yes, 0=no)
    pub has_config: bool,
    /// Progress shown (1=yes, 0=no)
    pub show_progress: bool,
    /// Global package cache directory
    pub cache_dir: PathBuf,
    /// Project log directory
    pub log_dir: PathBuf,
    /// Project root directory (may be empty if not in project)
    pub project_root: Option<PathBuf>,
    /// Path to Stata binary (may be empty if not found)
    pub stata_binary: Option<PathBuf>,
    /// How binary was detected
    pub stata_source: String,
}

impl CommandOutput for EnvOutput {
    fn command_name(&self) -> &'static str {
        "env"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy env output".to_string());
        lines.push(format_stata_scalar_bool("has_config", self.has_config));
        lines.push(format_stata_scalar_bool(
            "show_progress",
            self.show_progress,
        ));
        lines.push(format_stata_scalar_usize(
            "adopath_count",
            self.adopath_count,
        ));
        lines.push(format_stata_local(
            "cache_dir",
            &self.cache_dir.display().to_string(),
        ));
        lines.push(format_stata_local(
            "log_dir",
            &self.log_dir.display().to_string(),
        ));
        lines.push(format_stata_local("stata_source", &self.stata_source));

        // Optional fields
        if let Some(ref root) = self.project_root {
            lines.push(format_stata_local(
                "project_root",
                &root.display().to_string(),
            ));
        } else {
            lines.push("global stacy_project_root".to_string());
        }

        if let Some(ref binary) = self.stata_binary {
            lines.push(format_stata_local(
                "stata_binary",
                &binary.display().to_string(),
            ));
        } else {
            lines.push("global stacy_stata_binary".to_string());
        }

        lines.join("\n")
    }
}

// =============================================================================
// ExplainOutput
// =============================================================================

/// Output for `stacy explain` command
#[derive(Debug, Serialize)]
pub struct ExplainOutput {
    /// Error code number
    pub code: u32,
    /// Short error name
    pub name: String,
    /// Error category
    pub category: String,
    /// Full error description
    pub description: String,
    /// Link to Stata documentation
    pub url: String,
}

impl CommandOutput for ExplainOutput {
    fn command_name(&self) -> &'static str {
        "explain"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy explain output".to_string());
        lines.push(format_stata_scalar_int("code", self.code as i64));
        lines.push(format_stata_local("name", &self.name));
        lines.push(format_stata_local("category", &self.category));
        // Truncate description for Stata local (max ~200 chars for readability)
        let desc = if self.description.len() > 200 {
            format!("{}...", &self.description[..197])
        } else {
            self.description.clone()
        };
        lines.push(format_stata_local("description", &desc.replace('\n', " ")));
        lines.push(format_stata_local("url", &self.url));
        lines.join("\n")
    }
}

// =============================================================================
// InitOutput
// =============================================================================

/// Output for `stacy init` command
#[derive(Debug, Serialize)]
pub struct InitOutput {
    /// Number of files/directories created
    pub created_count: usize,
    /// Number of packages specified
    pub package_count: usize,
    /// Path where project was created
    pub path: PathBuf,
    /// 'success' or 'error'
    pub status: String,
}

impl CommandOutput for InitOutput {
    fn command_name(&self) -> &'static str {
        "init"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy init output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_local("path", &self.path.display().to_string()));
        lines.push(format_stata_scalar_usize(
            "created_count",
            self.created_count,
        ));
        lines.push(format_stata_scalar_usize(
            "package_count",
            self.package_count,
        ));
        lines.join("\n")
    }
}

// =============================================================================
// InstallOutput
// =============================================================================

/// Output for `stacy install` command
#[derive(Debug, Serialize)]
pub struct InstallOutput {
    /// Number already installed
    pub already_installed: i32,
    /// Number of newly installed packages
    pub installed: i32,
    /// Same as total
    pub package_count: usize,
    /// Number skipped (errors)
    pub skipped: i32,
    /// Total packages processed
    pub total: i32,
    /// 'success' or 'error'
    pub status: String,
}

impl CommandOutput for InstallOutput {
    fn command_name(&self) -> &'static str {
        "install"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy install output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_scalar_int("installed", self.installed as i64));
        lines.push(format_stata_scalar_int(
            "already_installed",
            self.already_installed as i64,
        ));
        lines.push(format_stata_scalar_int("skipped", self.skipped as i64));
        lines.push(format_stata_scalar_int("total", self.total as i64));
        lines.push(format_stata_scalar_usize(
            "package_count",
            self.package_count,
        ));
        lines.join("\n")
    }
}

// =============================================================================
// AddOutput
// =============================================================================

/// Output for `stacy add` command
#[derive(Debug, Serialize)]
pub struct AddOutput {
    /// Number of packages added
    pub added: i32,
    /// Dependency group: "production", "dev", or "test"
    pub group: String,
    /// Number of packages that failed
    pub failed: i32,
    /// Number of packages skipped (already present)
    pub skipped: i32,
    /// 'success', 'partial', or 'error'
    pub status: String,
    /// Total packages processed
    pub total: i32,
}

impl CommandOutput for AddOutput {
    fn command_name(&self) -> &'static str {
        "add"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy add output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_scalar_int("added", self.added as i64));
        lines.push(format_stata_scalar_int("skipped", self.skipped as i64));
        lines.push(format_stata_scalar_int("failed", self.failed as i64));
        lines.push(format_stata_scalar_int("total", self.total as i64));
        lines.push(format_stata_local("group", &self.group));
        lines.join("\n")
    }
}

// =============================================================================
// RemoveOutput
// =============================================================================

/// Output for `stacy remove` command
#[derive(Debug, Serialize)]
pub struct RemoveOutput {
    /// Number of packages removed
    pub removed: i32,
    /// Number of packages not found
    pub not_found: i32,
    /// 'success' or 'error'
    pub status: String,
    /// Total packages processed
    pub total: i32,
}

impl CommandOutput for RemoveOutput {
    fn command_name(&self) -> &'static str {
        "remove"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy remove output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_scalar_int("removed", self.removed as i64));
        lines.push(format_stata_scalar_int("not_found", self.not_found as i64));
        lines.push(format_stata_scalar_int("total", self.total as i64));
        lines.join("\n")
    }
}

// =============================================================================
// UpdateOutput
// =============================================================================

/// Output for `stacy update` command
#[derive(Debug, Serialize)]
pub struct UpdateOutput {
    /// Whether this was a dry run
    pub dry_run: bool,
    /// Number of packages that failed to update
    pub failed: i32,
    /// 'success', 'partial', or 'error'
    pub status: String,
    /// Total packages checked
    pub total: i32,
    /// Number of packages with updates available
    pub updates_available: i32,
    /// Number of packages updated
    pub updated: i32,
}

impl CommandOutput for UpdateOutput {
    fn command_name(&self) -> &'static str {
        "update"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy update output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_scalar_int("updated", self.updated as i64));
        lines.push(format_stata_scalar_int(
            "updates_available",
            self.updates_available as i64,
        ));
        lines.push(format_stata_scalar_int("failed", self.failed as i64));
        lines.push(format_stata_scalar_int("total", self.total as i64));
        lines.push(format_stata_scalar_bool("dry_run", self.dry_run));
        lines.join("\n")
    }
}

// =============================================================================
// DepsOutput
// =============================================================================

/// Output for `stacy deps` command
#[derive(Debug, Serialize)]
pub struct DepsOutput {
    /// Number of circular dependency paths
    pub circular_count: usize,
    /// Circular deps found (1=yes, 0=no)
    pub has_circular: bool,
    /// Missing files found (1=yes, 0=no)
    pub has_missing: bool,
    /// Number of missing files
    pub missing_count: usize,
    /// Number of unique dependencies
    pub unique_count: i32,
    /// Path to analyzed script
    pub script: PathBuf,
}

impl CommandOutput for DepsOutput {
    fn command_name(&self) -> &'static str {
        "deps"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy deps output".to_string());
        lines.push(format_stata_local(
            "script",
            &self.script.display().to_string(),
        ));
        lines.push(format_stata_scalar_int(
            "unique_count",
            self.unique_count as i64,
        ));
        lines.push(format_stata_scalar_bool("has_circular", self.has_circular));
        lines.push(format_stata_scalar_bool("has_missing", self.has_missing));
        lines.push(format_stata_scalar_usize(
            "circular_count",
            self.circular_count,
        ));
        lines.push(format_stata_scalar_usize(
            "missing_count",
            self.missing_count,
        ));
        lines.join("\n")
    }
}

// =============================================================================
// TaskOutput
// =============================================================================

/// Output for `stacy task` command
#[derive(Debug, Serialize)]
pub struct TaskOutput {
    /// Name of the task
    pub task_name: String,
    /// Whether all scripts succeeded
    pub success: bool,
    /// Exit code (0=success)
    pub exit_code: i32,
    /// Total execution time in seconds
    pub duration_secs: f64,
    /// Total number of scripts executed
    pub script_count: usize,
    /// Number of successful scripts
    pub success_count: usize,
    /// Number of failed scripts
    pub failed_count: usize,
    /// Results for individual scripts
    pub scripts: Vec<ScriptResultOutput>,
}

impl CommandOutput for TaskOutput {
    fn command_name(&self) -> &'static str {
        "task"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy task output".to_string());
        lines.push(format_stata_local("task_name", &self.task_name));
        lines.push(format_stata_scalar_bool("success", self.success));
        lines.push(format_stata_scalar_int("exit_code", self.exit_code as i64));
        lines.push(format_stata_scalar_float(
            "duration_secs",
            self.duration_secs,
        ));
        lines.push(format_stata_scalar_usize("script_count", self.script_count));
        lines.push(format_stata_scalar_usize(
            "success_count",
            self.success_count,
        ));
        lines.push(format_stata_scalar_usize("failed_count", self.failed_count));
        lines.join("\n")
    }
}

/// Individual script result within a task
#[derive(Debug, Serialize)]
pub struct ScriptResultOutput {
    /// Name of the script/task
    pub name: String,
    /// Path to the script
    pub script: PathBuf,
    /// Whether the script succeeded
    pub success: bool,
    /// Exit code
    pub exit_code: i32,
    /// Execution time in seconds
    pub duration_secs: f64,
}

// =============================================================================
// TaskListOutput
// =============================================================================

/// Output for `stacy task --list` command
#[derive(Debug, Serialize)]
pub struct TaskListOutput {
    /// Number of tasks defined
    pub task_count: usize,
    /// List of task info
    pub tasks: Vec<TaskInfo>,
}

/// Information about a single task
#[derive(Debug, Serialize)]
pub struct TaskInfo {
    /// Task name
    pub name: String,
    /// Task description
    pub description: String,
}

impl CommandOutput for TaskListOutput {
    fn command_name(&self) -> &'static str {
        "task-list"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy task --list output".to_string());
        lines.push(format_stata_scalar_usize("task_count", self.task_count));
        // Create a comma-separated list of task names
        let task_names: Vec<_> = self.tasks.iter().map(|t| t.name.as_str()).collect();
        lines.push(format_stata_local("task_names", &task_names.join(",")));
        lines.join("\n")
    }
}

// =============================================================================
// ListOutput
// =============================================================================

/// Output for `stacy list` command
#[derive(Debug, Serialize)]
pub struct ListOutput {
    /// 'success' or 'error'
    pub status: String,
    /// Number of packages
    pub package_count: usize,
    /// List of package info
    pub packages: Vec<ListPackageInfo>,
}

/// Information about a package in the list
#[derive(Debug, Serialize)]
pub struct ListPackageInfo {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package source (ssc, github:user/repo, etc.)
    pub source: String,
    /// Dependency group (production, dev, test)
    pub group: String,
}

impl CommandOutput for ListOutput {
    fn command_name(&self) -> &'static str {
        "list"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy list output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_scalar_usize(
            "package_count",
            self.package_count,
        ));
        // Create comma-separated lists
        let names: Vec<_> = self.packages.iter().map(|p| p.name.as_str()).collect();
        let versions: Vec<_> = self.packages.iter().map(|p| p.version.as_str()).collect();
        let sources: Vec<_> = self.packages.iter().map(|p| p.source.as_str()).collect();
        let groups: Vec<_> = self.packages.iter().map(|p| p.group.as_str()).collect();
        lines.push(format_stata_local("package_names", &names.join(",")));
        lines.push(format_stata_local("package_versions", &versions.join(",")));
        lines.push(format_stata_local("package_sources", &sources.join(",")));
        lines.push(format_stata_local("package_groups", &groups.join(",")));
        lines.join("\n")
    }
}

// =============================================================================
// OutdatedOutput
// =============================================================================

/// Output for `stacy outdated` command
#[derive(Debug, Serialize)]
pub struct OutdatedOutput {
    /// 'success' or 'error'
    pub status: String,
    /// Number of outdated packages
    pub outdated_count: usize,
    /// Total packages checked
    pub total_count: usize,
    /// List of outdated packages
    pub packages: Vec<OutdatedPackageInfo>,
}

/// Information about an outdated package
#[derive(Debug, Serialize)]
pub struct OutdatedPackageInfo {
    /// Package name
    pub name: String,
    /// Current installed version
    pub current: String,
    /// Latest available version
    pub latest: String,
    /// Package source
    pub source: String,
}

impl CommandOutput for OutdatedOutput {
    fn command_name(&self) -> &'static str {
        "outdated"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy outdated output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_scalar_usize(
            "outdated_count",
            self.outdated_count,
        ));
        lines.push(format_stata_scalar_usize("total_count", self.total_count));
        // Create comma-separated lists
        let names: Vec<_> = self.packages.iter().map(|p| p.name.as_str()).collect();
        let currents: Vec<_> = self.packages.iter().map(|p| p.current.as_str()).collect();
        let latests: Vec<_> = self.packages.iter().map(|p| p.latest.as_str()).collect();
        lines.push(format_stata_local("outdated_names", &names.join(",")));
        lines.push(format_stata_local("outdated_currents", &currents.join(",")));
        lines.push(format_stata_local("outdated_latests", &latests.join(",")));
        lines.join("\n")
    }
}

// =============================================================================
// LockOutput
// =============================================================================

/// Output for `stacy lock` command
#[derive(Debug, Serialize)]
pub struct LockOutput {
    /// 'success', 'updated', or 'error'
    pub status: String,
    /// Number of packages in lockfile
    pub package_count: usize,
    /// Whether lockfile was updated
    pub updated: bool,
    /// Whether lockfile is in sync with config (for --check)
    pub in_sync: bool,
}

impl CommandOutput for LockOutput {
    fn command_name(&self) -> &'static str {
        "lock"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy lock output".to_string());
        lines.push(format_stata_local("status", &self.status));
        lines.push(format_stata_scalar_usize(
            "package_count",
            self.package_count,
        ));
        lines.push(format_stata_scalar_bool("updated", self.updated));
        lines.push(format_stata_scalar_bool("in_sync", self.in_sync));
        lines.join("\n")
    }
}

// =============================================================================
// TestOutput
// =============================================================================

/// Output for `stacy test` command
#[derive(Debug, Serialize)]
pub struct TestOutput {
    /// Total number of tests
    pub test_count: usize,
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// Number of skipped tests
    pub skipped: usize,
    /// Total execution time in seconds
    pub duration_secs: f64,
    /// Whether all tests passed
    pub success: bool,
    /// Individual test results
    pub tests: Vec<TestResultOutput>,
}

impl CommandOutput for TestOutput {
    fn command_name(&self) -> &'static str {
        "test"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy test output".to_string());
        lines.push(format_stata_scalar_bool("success", self.success));
        lines.push(format_stata_scalar_usize("test_count", self.test_count));
        lines.push(format_stata_scalar_usize("passed", self.passed));
        lines.push(format_stata_scalar_usize("failed", self.failed));
        lines.push(format_stata_scalar_usize("skipped", self.skipped));
        lines.push(format_stata_scalar_float(
            "duration_secs",
            self.duration_secs,
        ));
        lines.join("\n")
    }
}

/// Individual test result
#[derive(Debug, Serialize)]
pub struct TestResultOutput {
    /// Test name
    pub name: String,
    /// Path to the test file
    pub path: PathBuf,
    /// Status: "passed", "failed", or "skipped"
    pub status: String,
    /// Execution time in seconds
    pub duration_secs: f64,
    /// Exit code
    pub exit_code: i32,
    /// Error message if test failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

// =============================================================================
// TestListOutput
// =============================================================================

/// Output for `stacy test --list` command
#[derive(Debug, Serialize)]
pub struct TestListOutput {
    /// Number of tests found
    pub test_count: usize,
    /// List of test info
    pub tests: Vec<TestInfo>,
}

/// Information about a single test
#[derive(Debug, Serialize)]
pub struct TestInfo {
    /// Test name
    pub name: String,
    /// Path to the test file
    pub path: PathBuf,
}

impl CommandOutput for TestListOutput {
    fn command_name(&self) -> &'static str {
        "test-list"
    }

    fn to_stata(&self) -> String {
        let mut lines = Vec::new();
        lines.push("* stacy test --list output".to_string());
        lines.push(format_stata_scalar_usize("test_count", self.test_count));
        // Create a comma-separated list of test names
        let test_names: Vec<_> = self.tests.iter().map(|t| t.name.as_str()).collect();
        lines.push(format_stata_local("test_names", &test_names.join(",")));
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // RunOutput tests
    // =========================================================================

    #[test]
    fn test_run_output_to_stata() {
        let output = RunOutput {
            success: true,
            exit_code: 0,
            duration_secs: 1.5,
            error_count: 0,
            source: "file".to_string(),
            script: PathBuf::from("/path/to/script.do"),
            log_file: PathBuf::from("/path/to/script.log"),
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_success = 1"));
        assert!(stata.contains("scalar stacy_exit_code = 0"));
        assert!(stata.contains("scalar stacy_duration_secs = 1.500000"));
        assert!(stata.contains("global stacy_source \"file\""));
        assert!(stata.contains("global stacy_script \"/path/to/script.do\""));
    }

    #[test]
    fn test_run_output_to_json() {
        let output = RunOutput {
            success: true,
            exit_code: 0,
            duration_secs: 1.5,
            error_count: 0,
            source: "file".to_string(),
            script: PathBuf::from("/path/to/script.do"),
            log_file: PathBuf::from("/path/to/script.log"),
        };

        let json = output.to_json();
        assert!(json.contains("\"success\": true"));
        assert!(json.contains("\"exit_code\": 0"));
    }

    #[test]
    fn test_doctor_output_to_stata() {
        let output = DoctorOutput {
            ready: true,
            passed: 5,
            warnings: 1,
            failed: 0,
            check_count: 6,
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_ready = 1"));
        assert!(stata.contains("scalar stacy_passed = 5"));
        assert!(stata.contains("scalar stacy_warnings = 1"));
        assert!(stata.contains("scalar stacy_failed = 0"));
        assert!(stata.contains("scalar stacy_check_count = 6"));
    }

    #[test]
    fn test_env_output_to_stata_with_optionals() {
        let output = EnvOutput {
            has_config: true,
            show_progress: true,
            adopath_count: 4,
            cache_dir: PathBuf::from("/home/user/.cache/stacy/packages"),
            log_dir: PathBuf::from("logs"),
            project_root: Some(PathBuf::from("/project")),
            stata_binary: Some(PathBuf::from("/usr/local/bin/stata")),
            stata_source: "auto-detected".to_string(),
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_has_config = 1"));
        assert!(stata.contains("global stacy_cache_dir"));
        assert!(stata.contains("global stacy_project_root \"/project\""));
        assert!(stata.contains("global stacy_stata_binary \"/usr/local/bin/stata\""));
    }

    #[test]
    fn test_env_output_to_stata_without_optionals() {
        let output = EnvOutput {
            has_config: false,
            show_progress: true,
            adopath_count: 4,
            cache_dir: PathBuf::from("/home/user/.cache/stacy/packages"),
            log_dir: PathBuf::from("logs"),
            project_root: None,
            stata_binary: None,
            stata_source: "not found".to_string(),
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_has_config = 0"));
        assert!(stata.contains("global stacy_cache_dir"));
        assert!(stata.contains("global stacy_project_root"));
        assert!(stata.contains("global stacy_stata_binary"));
    }

    #[test]
    fn test_init_output_to_stata() {
        let output = InitOutput {
            status: "success".to_string(),
            path: PathBuf::from("/project"),
            created_count: 2,
            package_count: 0,
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_created_count = 2"));
    }

    #[test]
    fn test_install_output_to_stata() {
        let output = InstallOutput {
            status: "success".to_string(),
            installed: 3,
            already_installed: 2,
            skipped: 1,
            total: 6,
            package_count: 6,
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_installed = 3"));
        assert!(stata.contains("scalar stacy_already_installed = 2"));
        assert!(stata.contains("scalar stacy_skipped = 1"));
    }

    #[test]
    fn test_deps_output_to_stata() {
        let output = DepsOutput {
            script: PathBuf::from("/path/to/main.do"),
            unique_count: 5,
            has_circular: false,
            has_missing: true,
            circular_count: 0,
            missing_count: 2,
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_script \"/path/to/main.do\""));
        assert!(stata.contains("scalar stacy_unique_count = 5"));
        assert!(stata.contains("scalar stacy_has_circular = 0"));
        assert!(stata.contains("scalar stacy_has_missing = 1"));
        assert!(stata.contains("scalar stacy_missing_count = 2"));
    }

    #[test]
    fn test_run_output_with_path_containing_spaces() {
        let output = RunOutput {
            success: true,
            exit_code: 0,
            duration_secs: 1.0,
            error_count: 0,
            source: "file".to_string(),
            script: PathBuf::from("/path/with spaces/script.do"),
            log_file: PathBuf::from("/path/with spaces/script.log"),
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_script \"/path/with spaces/script.do\""));
    }

    // =========================================================================
    // ParallelRunOutput tests
    // =========================================================================

    #[test]
    fn test_parallel_run_output_to_stata() {
        let output = ParallelRunOutput {
            success: true,
            exit_code: 0,
            duration_secs: 2.5,
            parallel: true,
            jobs: 4,
            passed: 3,
            failed: 0,
            total: 3,
            scripts: vec![],
        };

        let stata = output.to_stata();
        assert!(stata.contains("* stacy run --parallel output"));
        assert!(stata.contains("scalar stacy_success = 1"));
        assert!(stata.contains("scalar stacy_exit_code = 0"));
        assert!(stata.contains("scalar stacy_parallel = 1"));
        assert!(stata.contains("scalar stacy_jobs = 4"));
        assert!(stata.contains("scalar stacy_passed = 3"));
        assert!(stata.contains("scalar stacy_failed = 0"));
        assert!(stata.contains("scalar stacy_total = 3"));
    }

    #[test]
    fn test_parallel_run_output_to_stata_with_failures() {
        let output = ParallelRunOutput {
            success: false,
            exit_code: 2,
            duration_secs: 1.8,
            parallel: true,
            jobs: 2,
            passed: 1,
            failed: 2,
            total: 3,
            scripts: vec![],
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_success = 0"));
        assert!(stata.contains("scalar stacy_exit_code = 2"));
        assert!(stata.contains("scalar stacy_passed = 1"));
        assert!(stata.contains("scalar stacy_failed = 2"));
    }

    #[test]
    fn test_parallel_run_output_sequential_mode() {
        let output = ParallelRunOutput {
            success: true,
            exit_code: 0,
            duration_secs: 3.0,
            parallel: false,
            jobs: 1,
            passed: 2,
            failed: 0,
            total: 2,
            scripts: vec![],
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_parallel = 0"));
        assert!(stata.contains("scalar stacy_jobs = 1"));
    }

    #[test]
    fn test_parallel_run_output_to_json() {
        let output = ParallelRunOutput {
            success: true,
            exit_code: 0,
            duration_secs: 1.5,
            parallel: true,
            jobs: 4,
            passed: 2,
            failed: 0,
            total: 2,
            scripts: vec![
                ScriptRunResult {
                    script: PathBuf::from("first.do"),
                    success: true,
                    exit_code: 0,
                    duration_secs: 0.5,
                    log_file: PathBuf::from("first.log"),
                    error_message: None,
                },
                ScriptRunResult {
                    script: PathBuf::from("second.do"),
                    success: true,
                    exit_code: 0,
                    duration_secs: 0.7,
                    log_file: PathBuf::from("second.log"),
                    error_message: None,
                },
            ],
        };

        let json = output.to_json();
        assert!(json.contains("\"success\": true"));
        assert!(json.contains("\"parallel\": true"));
        assert!(json.contains("\"jobs\": 4"));
        assert!(json.contains("\"passed\": 2"));
        assert!(json.contains("\"failed\": 0"));
        assert!(json.contains("\"scripts\""));
        assert!(json.contains("first.do"));
        assert!(json.contains("second.do"));
    }

    #[test]
    fn test_parallel_run_output_json_with_error() {
        let output = ParallelRunOutput {
            success: false,
            exit_code: 2,
            duration_secs: 1.0,
            parallel: true,
            jobs: 2,
            passed: 0,
            failed: 1,
            total: 1,
            scripts: vec![ScriptRunResult {
                script: PathBuf::from("failing.do"),
                success: false,
                exit_code: 2,
                duration_secs: 0.3,
                log_file: PathBuf::from("failing.log"),
                error_message: Some("r(198) - syntax error".to_string()),
            }],
        };

        let json = output.to_json();
        assert!(json.contains("\"success\": false"));
        assert!(json.contains("\"exit_code\": 2"));
        assert!(json.contains("\"error_message\": \"r(198) - syntax error\""));
    }

    // =========================================================================
    // ScriptRunResult tests
    // =========================================================================

    #[test]
    fn test_script_run_result_json_skips_none_error() {
        let result = ScriptRunResult {
            script: PathBuf::from("test.do"),
            success: true,
            exit_code: 0,
            duration_secs: 0.5,
            log_file: PathBuf::from("test.log"),
            error_message: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        // error_message should be skipped when None
        assert!(!json.contains("error_message"));
    }

    #[test]
    fn test_script_run_result_json_includes_error() {
        let result = ScriptRunResult {
            script: PathBuf::from("failing.do"),
            success: false,
            exit_code: 1,
            duration_secs: 0.2,
            log_file: PathBuf::from("failing.log"),
            error_message: Some("error occurred".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"error_message\":\"error occurred\""));
    }

    // =========================================================================
    // BenchOutput tests
    // =========================================================================

    #[test]
    fn test_bench_output_to_stata() {
        let output = BenchOutput {
            script: PathBuf::from("/path/to/bench.do"),
            measured_runs: 10,
            warmup_runs: 3,
            mean_secs: 1.234,
            median_secs: 1.200,
            min_secs: 0.900,
            max_secs: 1.800,
            stddev_secs: 0.150,
            success: true,
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_script \"/path/to/bench.do\""));
        assert!(stata.contains("scalar stacy_measured_runs = 10"));
        assert!(stata.contains("scalar stacy_warmup_runs = 3"));
        assert!(stata.contains("scalar stacy_mean_secs = 1.234000"));
        assert!(stata.contains("scalar stacy_median_secs = 1.200000"));
        assert!(stata.contains("scalar stacy_min_secs = 0.900000"));
        assert!(stata.contains("scalar stacy_max_secs = 1.800000"));
        assert!(stata.contains("scalar stacy_stddev_secs = 0.150000"));
        assert!(stata.contains("scalar stacy_success = 1"));
    }

    // =========================================================================
    // CacheCleanOutput tests
    // =========================================================================

    #[test]
    fn test_cache_clean_output_to_stata() {
        let output = CacheCleanOutput {
            entries_removed: 5,
            entries_remaining: 10,
            status: "success".to_string(),
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_entries_removed = 5"));
        assert!(stata.contains("scalar stacy_entries_remaining = 10"));
    }

    // =========================================================================
    // CacheInfoOutput tests
    // =========================================================================

    #[test]
    fn test_cache_info_output_to_stata() {
        let output = CacheInfoOutput {
            entry_count: 42,
            size_bytes: 1048576,
            cache_path: PathBuf::from("/home/user/.cache/stacy"),
            cache_exists: true,
            oldest_age_secs: Some(86400),
            newest_age_secs: Some(3600),
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_entry_count = 42"));
        assert!(stata.contains("scalar stacy_size_bytes = 1048576"));
        assert!(stata.contains("global stacy_cache_path \"/home/user/.cache/stacy\""));
        assert!(stata.contains("scalar stacy_cache_exists = 1"));
        assert!(stata.contains("scalar stacy_oldest_age_secs = 86400"));
        assert!(stata.contains("scalar stacy_newest_age_secs = 3600"));
    }

    #[test]
    fn test_cache_info_output_to_stata_without_optionals() {
        let output = CacheInfoOutput {
            entry_count: 0,
            size_bytes: 0,
            cache_path: PathBuf::from("/tmp/cache"),
            cache_exists: false,
            oldest_age_secs: None,
            newest_age_secs: None,
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_cache_exists = 0"));
        assert!(!stata.contains("oldest_age_secs"));
        assert!(!stata.contains("newest_age_secs"));
    }

    // =========================================================================
    // CacheHitOutput tests
    // =========================================================================

    #[test]
    fn test_cache_hit_output_to_stata() {
        let output = CacheHitOutput {
            success: true,
            exit_code: 0,
            duration_secs: 0.5,
            error_count: 0,
            source: "cache".to_string(),
            script: PathBuf::from("/path/to/cached.do"),
            cached_at: std::time::UNIX_EPOCH,
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_success = 1"));
        assert!(stata.contains("scalar stacy_exit_code = 0"));
        assert!(stata.contains("scalar stacy_duration_secs = 0.500000"));
        assert!(stata.contains("scalar stacy_error_count = 0"));
        assert!(stata.contains("global stacy_source \"cache\""));
        assert!(stata.contains("global stacy_script \"/path/to/cached.do\""));
        assert!(stata.contains("scalar stacy_cache_hit = 1"));
    }

    // =========================================================================
    // ExplainOutput tests
    // =========================================================================

    #[test]
    fn test_explain_output_to_stata() {
        let output = ExplainOutput {
            code: 198,
            name: "syntax error".to_string(),
            category: "Syntax".to_string(),
            description: "The command is not recognized.".to_string(),
            url: "https://www.stata.com/help.cgi?r(198)".to_string(),
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_code = 198"));
        assert!(stata.contains("global stacy_name \"syntax error\""));
        assert!(stata.contains("global stacy_category \"Syntax\""));
        assert!(stata.contains("global stacy_description \"The command is not recognized.\""));
        assert!(stata.contains("global stacy_url \"https://www.stata.com/help.cgi?r(198)\""));
    }

    // =========================================================================
    // AddOutput tests
    // =========================================================================

    #[test]
    fn test_add_output_to_stata() {
        let output = AddOutput {
            added: 2,
            group: "production".to_string(),
            failed: 0,
            skipped: 1,
            status: "success".to_string(),
            total: 3,
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_added = 2"));
        assert!(stata.contains("scalar stacy_skipped = 1"));
        assert!(stata.contains("scalar stacy_failed = 0"));
        assert!(stata.contains("scalar stacy_total = 3"));
        assert!(stata.contains("global stacy_group \"production\""));
    }

    // =========================================================================
    // RemoveOutput tests
    // =========================================================================

    #[test]
    fn test_remove_output_to_stata() {
        let output = RemoveOutput {
            removed: 2,
            not_found: 1,
            status: "success".to_string(),
            total: 3,
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_removed = 2"));
        assert!(stata.contains("scalar stacy_not_found = 1"));
        assert!(stata.contains("scalar stacy_total = 3"));
    }

    // =========================================================================
    // UpdateOutput tests
    // =========================================================================

    #[test]
    fn test_update_output_to_stata() {
        let output = UpdateOutput {
            dry_run: true,
            failed: 0,
            status: "success".to_string(),
            total: 5,
            updates_available: 2,
            updated: 0,
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_updated = 0"));
        assert!(stata.contains("scalar stacy_updates_available = 2"));
        assert!(stata.contains("scalar stacy_failed = 0"));
        assert!(stata.contains("scalar stacy_total = 5"));
        assert!(stata.contains("scalar stacy_dry_run = 1"));
    }

    // =========================================================================
    // TaskOutput tests
    // =========================================================================

    #[test]
    fn test_task_output_to_stata() {
        let output = TaskOutput {
            task_name: "build".to_string(),
            success: true,
            exit_code: 0,
            duration_secs: 2.5,
            script_count: 3,
            success_count: 3,
            failed_count: 0,
            scripts: vec![],
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_task_name \"build\""));
        assert!(stata.contains("scalar stacy_success = 1"));
        assert!(stata.contains("scalar stacy_exit_code = 0"));
        assert!(stata.contains("scalar stacy_duration_secs = 2.500000"));
        assert!(stata.contains("scalar stacy_script_count = 3"));
        assert!(stata.contains("scalar stacy_success_count = 3"));
        assert!(stata.contains("scalar stacy_failed_count = 0"));
    }

    // =========================================================================
    // TaskListOutput tests
    // =========================================================================

    #[test]
    fn test_task_list_output_to_stata() {
        let output = TaskListOutput {
            task_count: 2,
            tasks: vec![
                TaskInfo {
                    name: "build".to_string(),
                    description: "Build the project".to_string(),
                },
                TaskInfo {
                    name: "test".to_string(),
                    description: "Run tests".to_string(),
                },
            ],
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_task_count = 2"));
        assert!(stata.contains("global stacy_task_names \"build,test\""));
    }

    // =========================================================================
    // ListOutput tests
    // =========================================================================

    #[test]
    fn test_list_output_to_stata() {
        let output = ListOutput {
            status: "success".to_string(),
            package_count: 2,
            packages: vec![
                ListPackageInfo {
                    name: "estout".to_string(),
                    version: "3.31".to_string(),
                    source: "ssc".to_string(),
                    group: "production".to_string(),
                },
                ListPackageInfo {
                    name: "reghdfe".to_string(),
                    version: "6.0".to_string(),
                    source: "github:sergiocorreia/reghdfe".to_string(),
                    group: "production".to_string(),
                },
            ],
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_package_count = 2"));
        assert!(stata.contains("global stacy_package_names \"estout,reghdfe\""));
        assert!(stata.contains("global stacy_package_versions \"3.31,6.0\""));
        assert!(stata.contains("global stacy_package_sources \"ssc,github:sergiocorreia/reghdfe\""));
        assert!(stata.contains("global stacy_package_groups \"production,production\""));
    }

    // =========================================================================
    // OutdatedOutput tests
    // =========================================================================

    #[test]
    fn test_outdated_output_to_stata() {
        let output = OutdatedOutput {
            status: "success".to_string(),
            outdated_count: 1,
            total_count: 3,
            packages: vec![OutdatedPackageInfo {
                name: "estout".to_string(),
                current: "3.30".to_string(),
                latest: "3.31".to_string(),
                source: "ssc".to_string(),
            }],
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_outdated_count = 1"));
        assert!(stata.contains("scalar stacy_total_count = 3"));
        assert!(stata.contains("global stacy_outdated_names \"estout\""));
        assert!(stata.contains("global stacy_outdated_currents \"3.30\""));
        assert!(stata.contains("global stacy_outdated_latests \"3.31\""));
    }

    // =========================================================================
    // LockOutput tests
    // =========================================================================

    #[test]
    fn test_lock_output_to_stata() {
        let output = LockOutput {
            status: "success".to_string(),
            package_count: 5,
            updated: false,
            in_sync: true,
        };

        let stata = output.to_stata();
        assert!(stata.contains("global stacy_status \"success\""));
        assert!(stata.contains("scalar stacy_package_count = 5"));
        assert!(stata.contains("scalar stacy_updated = 0"));
        assert!(stata.contains("scalar stacy_in_sync = 1"));
    }

    // =========================================================================
    // TestOutput tests
    // =========================================================================

    #[test]
    fn test_test_output_to_stata() {
        let output = TestOutput {
            test_count: 10,
            passed: 8,
            failed: 1,
            skipped: 1,
            duration_secs: 5.5,
            success: false,
            tests: vec![],
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_success = 0"));
        assert!(stata.contains("scalar stacy_test_count = 10"));
        assert!(stata.contains("scalar stacy_passed = 8"));
        assert!(stata.contains("scalar stacy_failed = 1"));
        assert!(stata.contains("scalar stacy_skipped = 1"));
        assert!(stata.contains("scalar stacy_duration_secs = 5.500000"));
    }

    // =========================================================================
    // TestListOutput tests
    // =========================================================================

    #[test]
    fn test_test_list_output_to_stata() {
        let output = TestListOutput {
            test_count: 2,
            tests: vec![
                TestInfo {
                    name: "test_basic".to_string(),
                    path: PathBuf::from("tests/test_basic.do"),
                },
                TestInfo {
                    name: "test_advanced".to_string(),
                    path: PathBuf::from("tests/test_advanced.do"),
                },
            ],
        };

        let stata = output.to_stata();
        assert!(stata.contains("scalar stacy_test_count = 2"));
        assert!(stata.contains("global stacy_test_names \"test_basic,test_advanced\""));
    }

    // =========================================================================
    // Cross-cutting Stata syntax invariant tests
    // =========================================================================

    /// Helper: construct every output type and return their to_stata() results.
    fn all_output_stata_strings() -> Vec<(&'static str, String)> {
        vec![
            (
                "RunOutput",
                RunOutput {
                    success: true,
                    exit_code: 0,
                    duration_secs: 1.0,
                    error_count: 0,
                    source: "file".to_string(),
                    script: PathBuf::from("test.do"),
                    log_file: PathBuf::from("test.log"),
                }
                .to_stata(),
            ),
            (
                "ParallelRunOutput",
                ParallelRunOutput {
                    success: true,
                    exit_code: 0,
                    duration_secs: 1.0,
                    parallel: true,
                    jobs: 2,
                    passed: 2,
                    failed: 0,
                    total: 2,
                    scripts: vec![],
                }
                .to_stata(),
            ),
            (
                "BenchOutput",
                BenchOutput {
                    script: PathBuf::from("bench.do"),
                    measured_runs: 5,
                    warmup_runs: 2,
                    mean_secs: 1.0,
                    median_secs: 1.0,
                    min_secs: 0.5,
                    max_secs: 1.5,
                    stddev_secs: 0.1,
                    success: true,
                }
                .to_stata(),
            ),
            (
                "CacheCleanOutput",
                CacheCleanOutput {
                    entries_removed: 3,
                    entries_remaining: 7,
                    status: "success".to_string(),
                }
                .to_stata(),
            ),
            (
                "CacheInfoOutput",
                CacheInfoOutput {
                    entry_count: 10,
                    size_bytes: 1024,
                    cache_path: PathBuf::from("/tmp/cache"),
                    cache_exists: true,
                    oldest_age_secs: Some(100),
                    newest_age_secs: Some(10),
                }
                .to_stata(),
            ),
            (
                "CacheHitOutput",
                CacheHitOutput {
                    success: true,
                    exit_code: 0,
                    duration_secs: 0.1,
                    error_count: 0,
                    source: "cache".to_string(),
                    script: PathBuf::from("test.do"),
                    cached_at: std::time::UNIX_EPOCH,
                }
                .to_stata(),
            ),
            (
                "DoctorOutput",
                DoctorOutput {
                    ready: true,
                    passed: 5,
                    warnings: 0,
                    failed: 0,
                    check_count: 5,
                }
                .to_stata(),
            ),
            (
                "EnvOutput",
                EnvOutput {
                    has_config: true,
                    show_progress: false,
                    adopath_count: 3,
                    cache_dir: PathBuf::from("/tmp/cache"),
                    log_dir: PathBuf::from("logs"),
                    project_root: Some(PathBuf::from("/project")),
                    stata_binary: Some(PathBuf::from("/usr/bin/stata")),
                    stata_source: "config".to_string(),
                }
                .to_stata(),
            ),
            (
                "ExplainOutput",
                ExplainOutput {
                    code: 100,
                    name: "not allowed".to_string(),
                    category: "General".to_string(),
                    description: "Explanation text".to_string(),
                    url: "https://stata.com/r100".to_string(),
                }
                .to_stata(),
            ),
            (
                "InitOutput",
                InitOutput {
                    status: "success".to_string(),
                    path: PathBuf::from("/project"),
                    created_count: 2,
                    package_count: 0,
                }
                .to_stata(),
            ),
            (
                "InstallOutput",
                InstallOutput {
                    status: "success".to_string(),
                    installed: 3,
                    already_installed: 1,
                    skipped: 0,
                    total: 4,
                    package_count: 4,
                }
                .to_stata(),
            ),
            (
                "AddOutput",
                AddOutput {
                    added: 1,
                    group: "production".to_string(),
                    failed: 0,
                    skipped: 0,
                    status: "success".to_string(),
                    total: 1,
                }
                .to_stata(),
            ),
            (
                "RemoveOutput",
                RemoveOutput {
                    removed: 1,
                    not_found: 0,
                    status: "success".to_string(),
                    total: 1,
                }
                .to_stata(),
            ),
            (
                "UpdateOutput",
                UpdateOutput {
                    dry_run: false,
                    failed: 0,
                    status: "success".to_string(),
                    total: 2,
                    updates_available: 1,
                    updated: 1,
                }
                .to_stata(),
            ),
            (
                "DepsOutput",
                DepsOutput {
                    script: PathBuf::from("main.do"),
                    unique_count: 3,
                    has_circular: false,
                    has_missing: false,
                    circular_count: 0,
                    missing_count: 0,
                }
                .to_stata(),
            ),
            (
                "TaskOutput",
                TaskOutput {
                    task_name: "build".to_string(),
                    success: true,
                    exit_code: 0,
                    duration_secs: 1.0,
                    script_count: 1,
                    success_count: 1,
                    failed_count: 0,
                    scripts: vec![],
                }
                .to_stata(),
            ),
            (
                "TaskListOutput",
                TaskListOutput {
                    task_count: 1,
                    tasks: vec![TaskInfo {
                        name: "build".to_string(),
                        description: "Build".to_string(),
                    }],
                }
                .to_stata(),
            ),
            (
                "ListOutput",
                ListOutput {
                    status: "success".to_string(),
                    package_count: 0,
                    packages: vec![],
                }
                .to_stata(),
            ),
            (
                "OutdatedOutput",
                OutdatedOutput {
                    status: "success".to_string(),
                    outdated_count: 0,
                    total_count: 0,
                    packages: vec![],
                }
                .to_stata(),
            ),
            (
                "LockOutput",
                LockOutput {
                    status: "success".to_string(),
                    package_count: 0,
                    updated: false,
                    in_sync: true,
                }
                .to_stata(),
            ),
            (
                "TestOutput",
                TestOutput {
                    test_count: 0,
                    passed: 0,
                    failed: 0,
                    skipped: 0,
                    duration_secs: 0.0,
                    success: true,
                    tests: vec![],
                }
                .to_stata(),
            ),
            (
                "TestListOutput",
                TestListOutput {
                    test_count: 0,
                    tests: vec![],
                }
                .to_stata(),
            ),
        ]
    }

    #[test]
    fn test_all_outputs_use_stacy_prefix() {
        for (name, stata) in all_output_stata_strings() {
            for line in stata.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('*') {
                    continue;
                }
                assert!(
                    line.starts_with("scalar stacy_") || line.starts_with("global stacy_"),
                    "{}: line should start with 'scalar stacy_' or 'global stacy_', got: {}",
                    name,
                    line
                );
            }
        }
    }

    #[test]
    fn test_all_outputs_globals_use_double_quotes() {
        for (name, stata) in all_output_stata_strings() {
            for line in stata.lines() {
                if line.starts_with("global stacy_") {
                    // Must NOT contain compound quote pattern: `"..."'
                    assert!(
                        !line.contains("`\""),
                        "{}: global line uses compound quotes (backtick-double-quote), \
                         should use plain double quotes: {}",
                        name,
                        line
                    );
                }
            }
        }
    }

    #[test]
    fn test_all_outputs_scalars_have_numeric_values() {
        for (name, stata) in all_output_stata_strings() {
            for line in stata.lines() {
                if line.starts_with("scalar stacy_") {
                    if let Some(eq_pos) = line.find('=') {
                        let value = line[eq_pos + 1..].trim();
                        assert!(
                            value.parse::<f64>().is_ok(),
                            "{}: scalar value should be numeric, got '{}' in: {}",
                            name,
                            value,
                            line
                        );
                    } else {
                        panic!("{}: scalar line missing '=' assignment: {}", name, line);
                    }
                }
            }
        }
    }
}
