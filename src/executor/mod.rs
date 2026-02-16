pub mod binary;
pub mod log_reader;
pub mod progress;
pub mod runner;
pub mod verbosity;
pub mod wrapper;

use crate::error::{Result, StataError};
use crate::metrics::Metrics;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct ExecutionResult {
    pub exit_code: i32,
    pub log_file: PathBuf,
    pub errors: Vec<StataError>,
    pub duration: Duration,
    pub success: bool,
    pub parse_duration: Duration,
    pub metrics: Option<Metrics>,
}

pub struct StataExecutor {
    stata_binary: String,
    verbosity: verbosity::Verbosity,
    progress_interval: Duration,
    /// Allow global packages (SITE, PERSONAL, PLUS, OLDPLACE) in addition to locked packages.
    /// Default is false (strict mode) - only locked packages and BASE are available.
    allow_global: bool,
}

impl Default for StataExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl StataExecutor {
    /// Create new executor with auto-detected Stata binary and default verbosity
    ///
    /// Uses precedence chain:
    /// 1. Environment variable `$STATA_BINARY`
    /// 2. User config `~/.config/stacy/config.toml`
    /// 3. Auto-detection (platform locations + PATH search)
    ///
    /// # Panics
    ///
    /// Panics if no Stata binary found. Use `try_new()` for fallible construction.
    pub fn new() -> Self {
        Self::try_new(None, verbosity::Verbosity::PipedDefault)
            .expect("Failed to detect Stata binary")
    }

    /// Create new executor with optional CLI override
    ///
    /// # Arguments
    ///
    /// * `cli_engine` - Optional binary from CLI `--engine` flag
    /// * `verbosity` - Output verbosity level
    ///
    /// # Returns
    ///
    /// Returns error if no Stata binary found
    pub fn try_new(cli_engine: Option<&str>, verbosity: verbosity::Verbosity) -> Result<Self> {
        let stata_binary = binary::detect_stata_binary(cli_engine)?;

        Ok(Self {
            stata_binary,
            verbosity,
            progress_interval: Duration::from_millis(100),
            allow_global: false,
        })
    }

    /// Create executor with explicit binary path (for testing)
    pub fn with_binary(binary: impl Into<String>) -> Self {
        Self {
            stata_binary: binary.into(),
            verbosity: verbosity::Verbosity::PipedDefault,
            progress_interval: Duration::from_millis(100),
            allow_global: false,
        }
    }

    /// Set verbosity level
    pub fn with_verbosity(mut self, verbosity: verbosity::Verbosity) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Allow global packages (SITE, PERSONAL, PLUS, OLDPLACE) in addition to locked packages
    pub fn with_allow_global(mut self, allow: bool) -> Self {
        self.allow_global = allow;
        self
    }

    /// Run a Stata script with optional arguments
    pub fn run_with_args(
        &self,
        script: &Path,
        project_root: Option<&Path>,
        args: &std::collections::HashMap<String, String>,
    ) -> Result<ExecutionResult> {
        self.run_internal(script, project_root, args.clone(), None)
    }

    pub fn run(&self, script: &Path, project_root: Option<&Path>) -> Result<ExecutionResult> {
        self.run_internal(script, project_root, std::collections::HashMap::new(), None)
    }

    /// Run a Stata script in a specific working directory
    pub fn run_in_dir(
        &self,
        script: &Path,
        project_root: Option<&Path>,
        working_dir: &Path,
    ) -> Result<ExecutionResult> {
        self.run_internal(
            script,
            project_root,
            std::collections::HashMap::new(),
            Some(working_dir),
        )
    }

    fn run_internal(
        &self,
        script: &Path,
        project_root: Option<&Path>,
        args: std::collections::HashMap<String, String>,
        working_dir: Option<&Path>,
    ) -> Result<ExecutionResult> {
        use crate::error::parser::parse_log_for_errors;
        use runner::{run_stata, RunOptions};
        use std::thread;

        // Build run options
        let mut options = RunOptions::new(&self.stata_binary);
        if let Some(root) = project_root {
            options = options.with_project_root(root);
        }
        if !args.is_empty() {
            options = options.with_args(args);
        }
        options = options.with_allow_global(self.allow_global);
        if let Some(dir) = working_dir {
            options = options.with_working_dir(dir);
        }

        // Show execution details if VeryVerbose
        if self.verbosity.should_show_execution_details() {
            eprintln!("Execution details:");
            eprintln!("  Stata binary: {}", self.stata_binary);
            eprintln!("  Script: {}", script.display());
            if let Some(dir) = working_dir {
                eprintln!("  Working dir: {}", dir.display());
            }
            if let Some(root) = project_root {
                match crate::packages::lockfile::load_lockfile(root) {
                    Ok(Some(lockfile)) => {
                        match crate::packages::global_cache::build_s_ado(
                            &lockfile,
                            self.allow_global,
                        ) {
                            Ok(s_ado) => {
                                eprintln!("  S_ADO: {}", s_ado);
                                if !self.allow_global {
                                    eprintln!("  Mode: strict (use --allow-global to include PLUS, PERSONAL, etc.)");
                                }
                            }
                            Err(e) => eprintln!("  S_ADO: error building path: {}", e),
                        }
                    }
                    Ok(None) => {} // No lockfile — no S_ADO to display
                    Err(e) => eprintln!("  S_ADO: error loading lockfile: {}", e),
                }
            }
            eprintln!("  Command: stata-mp -b -q do {}", script.display());
            eprintln!();
        }

        // Determine log file path
        // Stata creates: {script_basename}.log in current working directory
        // e.g., "build/analysis.do" -> "analysis.log" (NOT "build/analysis.log")
        // When working_dir is set, the log lands in that directory.
        let log_basename = script
            .file_stem()
            .map(|s| PathBuf::from(s).with_extension("log"))
            .unwrap_or_else(|| script.with_extension("log"));
        let log_file = if let Some(dir) = working_dir {
            dir.join(&log_basename)
        } else {
            log_basename
        };

        // Start log streaming thread if verbose or interactive
        let stream_handle = if self.verbosity.should_stream_raw() {
            let log_path = log_file.clone();
            let poll_interval = self.progress_interval;

            // Print header to separate stacy output from Stata log
            eprintln!("─────────────────────────────────────────────────────────────");
            eprintln!("Stata log ({})", log_path.display());
            eprintln!("─────────────────────────────────────────────────────────────");

            Some(thread::spawn(move || {
                let _ = log_reader::stream_log_file(&log_path, poll_interval);
            }))
        } else if self.verbosity.should_stream_clean() {
            let log_path = log_file.clone();
            let poll_interval = self.progress_interval;

            Some(thread::spawn(move || {
                let _ = log_reader::stream_log_file_clean(&log_path, poll_interval);
            }))
        } else {
            None
        };

        // Run Stata
        let run_result = run_stata(script, options)?;

        // Wait for streaming thread to finish
        if let Some(handle) = stream_handle {
            let _ = handle.join();
        }

        // Parse log file for errors (with timing)
        let parse_start = Instant::now();
        let errors = if run_result.completed {
            parse_log_for_errors(&run_result.log_file)?
        } else {
            // Process was killed, create error for that
            vec![StataError::ProcessKilled {
                exit_code: run_result.exit_code,
            }]
        };
        let parse_duration = parse_start.elapsed();

        // Determine success and exit code
        let success = errors.is_empty();
        let exit_code = if success {
            0
        } else {
            // Map first error to exit code
            let exit_code = crate::error::mapper::error_to_exit_code(&errors[0]);

            // Show error context if Default verbosity and error occurred
            if self.verbosity.should_show_error_context() {
                if let Ok(context) = log_reader::get_error_context(&run_result.log_file) {
                    eprintln!("{}", context);
                }
            }

            exit_code
        };

        Ok(ExecutionResult {
            exit_code,
            log_file: run_result.log_file,
            errors,
            duration: run_result.duration,
            success,
            parse_duration,
            metrics: None, // Metrics collection happens in CLI layer
        })
    }
}
