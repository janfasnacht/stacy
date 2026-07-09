pub mod binary;
pub mod log_reader;
pub mod progress;
pub mod run_paths;
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
    /// Local ado directories to prepend to S_ADO (resolved to absolute paths).
    local_ado_paths: Vec<PathBuf>,
    /// Kill the Stata process if it exceeds this duration.
    timeout: Option<Duration>,
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
            local_ado_paths: Vec::new(),
            timeout: None,
        })
    }

    /// Create executor with explicit binary path (for testing)
    pub fn with_binary(binary: impl Into<String>) -> Self {
        Self {
            stata_binary: binary.into(),
            verbosity: verbosity::Verbosity::PipedDefault,
            progress_interval: Duration::from_millis(100),
            allow_global: false,
            local_ado_paths: Vec::new(),
            timeout: None,
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

    /// Set local ado directories to prepend to S_ADO
    pub fn with_local_ado_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.local_ado_paths = paths;
        self
    }

    /// Set execution timeout (SIGTERM → 5s grace → SIGKILL)
    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
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
        use runner::{run_stata, RunOptions};
        use std::thread;

        // Resolve absolute paths up front so RunPaths can derive a unique log
        // location regardless of the caller's cwd. The user's script may be
        // relative (e.g. from unit tests); the working_dir from cli/run.rs is
        // already absolute, but we default to current_dir() if absent.
        let abs_script: PathBuf = if script.is_absolute() {
            script.to_path_buf()
        } else {
            std::env::current_dir()?.join(script)
        };
        let effective_working_dir: PathBuf = match working_dir {
            Some(dir) => dir.to_path_buf(),
            None => std::env::current_dir()?,
        };

        // Per-invocation wrapper + unique log path. `_paths` is bound for the
        // full function scope so the wrapper file outlives every read of the
        // log (parse_log_for_errors, get_error_context, streaming threads).
        // See src/executor/run_paths.rs and #20 for rationale.
        let _paths = run_paths::RunPaths::prepare(&abs_script, &effective_working_dir)?;

        // Build run options
        let mut options = RunOptions::new(&self.stata_binary);
        if let Some(root) = project_root {
            options = options.with_project_root(root);
        }
        if !args.is_empty() {
            options = options.with_args(args);
        }
        options = options.with_allow_global(self.allow_global);
        if !self.local_ado_paths.is_empty() {
            options = options.with_local_ado_paths(self.local_ado_paths.clone());
        }
        if let Some(dir) = working_dir {
            options = options.with_working_dir(dir);
        }
        if let Some(timeout) = self.timeout {
            options = options.with_timeout(timeout);
        }
        options = options.with_log_file(_paths.log.clone());

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
                            &self.local_ado_paths,
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
                    Ok(None) => {
                        if !self.local_ado_paths.is_empty() {
                            let empty_lockfile = crate::project::Lockfile {
                                version: "1".to_string(),
                                stacy_version: None,
                                packages: std::collections::HashMap::new(),
                            };
                            match crate::packages::global_cache::build_s_ado(
                                &empty_lockfile,
                                self.allow_global,
                                &self.local_ado_paths,
                            ) {
                                Ok(s_ado) => eprintln!("  S_ADO: {}", s_ado),
                                Err(e) => eprintln!("  S_ADO: error building path: {}", e),
                            }
                        }
                    }
                    Err(e) => eprintln!("  S_ADO: error loading lockfile: {}", e),
                }
            }
            if let Some(timeout) = self.timeout {
                eprintln!("  Timeout: {}s", timeout.as_secs());
            }
            // The runner spawns Stata against a wrapper that delegates to the
            // user's script; show both so `-vv` reflects what actually runs.
            eprintln!("  Command: stata-mp -b -q do {}", _paths.wrapper.display());
            eprintln!("  Wraps: {}", abs_script.display());
            eprintln!();
        }

        // Log file path was computed by RunPaths above; reuse it for streaming.
        let log_file = _paths.log.clone();

        // Start log streaming thread if verbose or interactive. The thread
        // terminates when `stop` is set after the Stata process exits — the
        // log alone can't signal completion (a killed Stata writes no
        // trailer, and scripts can print marker-lookalike output).
        let stream_mode = if self.verbosity.should_stream_raw() {
            // Print header to separate stacy output from Stata log
            eprintln!("─────────────────────────────────────────────────────────────");
            eprintln!("Stata log ({})", log_file.display());
            eprintln!("─────────────────────────────────────────────────────────────");
            Some(log_reader::StreamMode::Raw)
        } else if self.verbosity.should_stream_clean() {
            Some(log_reader::StreamMode::Clean)
        } else {
            None
        };

        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stream_handle = stream_mode.map(|mode| {
            let log_path = log_file.clone();
            let poll_interval = self.progress_interval;
            let stop = std::sync::Arc::clone(&stop);
            thread::spawn(move || {
                let _ = log_reader::stream_log(&log_path, poll_interval, mode, &stop);
            })
        });

        // Run Stata against the wrapper script, not the user's script.
        // Stata derives the log basename from the script path it's given —
        // the wrapper has a unique stem so concurrent runs cannot collide.
        // Don't propagate a spawn error until the streamer is released, or
        // its thread spins unjoined forever.
        let run_result = run_stata(&_paths.wrapper, options);

        // Stata is done (or never started) — release and join the streamer.
        stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(handle) = stream_handle {
            let _ = handle.join();
        }

        let run_result = run_result?;

        // Parse log file for errors (with timing).
        //
        // Signal-killed (SIGTERM from our watchdog, OOM, ctrl-C) → ProcessKilled.
        // Otherwise — clean exit, code 0 or non-zero — inspect log + stderr.
        // A non-zero exit with no log is a launch failure (license seat
        // exhausted, missing binary, init error), and that's exactly the
        // case where Stata's stderr carries the real diagnostic (#21).
        let parse_start = Instant::now();
        let errors = if run_result.signaled {
            vec![StataError::ProcessKilled {
                exit_code: run_result.exit_code,
            }]
        } else {
            parse_or_explain(&run_result)?
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

/// Parse the log; on missing/empty/incomplete logs, fold captured stderr
/// into the error message instead of the unhelpful default
/// "Log file incomplete: no 'end of do-file' marker found".
///
/// Three cases:
/// 1. Log doesn't exist or is empty → Stata never wrote anything. Almost
///    always a launch failure (license seat exhausted, missing binary,
///    init error). The real diagnostic is on stderr.
/// 2. Log exists but parser reports it incomplete → process exited before
///    writing the trailer. Likely killed mid-run by something stacy didn't
///    initiate (OOM, external SIGKILL); stderr may also help.
/// 3. Log parses cleanly → return whatever the parser returned.
fn parse_or_explain(run: &runner::RunResult) -> Result<Vec<StataError>> {
    use crate::error::parser::parse_log_for_errors;
    use crate::error::Error;

    let metadata = std::fs::metadata(&run.log_file).ok();
    let log_present = metadata.as_ref().is_some_and(|m| m.len() > 0);

    if !log_present {
        return Err(Error::Execution(format_no_log_message(run)));
    }

    match parse_log_for_errors(&run.log_file) {
        Ok(errs) => Ok(errs),
        Err(Error::Parse(msg)) if msg.starts_with("Log file incomplete") => {
            Err(Error::Execution(format_incomplete_log_message(run)))
        }
        Err(other) => Err(other),
    }
}

fn format_no_log_message(run: &runner::RunResult) -> String {
    let trimmed = run.stderr.trim();
    let mut msg = format!(
        "Stata produced no log file (exit code {}). Expected: {}",
        run.exit_code,
        run.log_file.display()
    );
    if !trimmed.is_empty() {
        msg.push_str("\nStata stderr:\n");
        msg.push_str(trimmed);
    } else {
        msg.push_str(
            "\nStata wrote nothing to stderr. The binary may have failed to launch \
             (missing executable, missing license seat, or environment misconfiguration).",
        );
    }
    msg
}

fn format_incomplete_log_message(run: &runner::RunResult) -> String {
    let trimmed = run.stderr.trim();
    let mut msg = format!(
        "Stata exited (code {}) before writing 'end of do-file' to the log. \
         The process likely terminated mid-run.",
        run.exit_code
    );
    if !trimmed.is_empty() {
        msg.push_str("\nStata stderr:\n");
        msg.push_str(trimmed);
    }
    msg
}
