use crate::cache::detect::{check_cache_with_working_dir, hash_working_dir, CacheStatus};
use crate::cache::hash::{hash_dependency_tree, hash_lockfile};
use crate::cache::{BuildCache, CacheEntry, CachedError, CachedResult};
use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{
    CacheHitOutput, CommandOutput, ParallelRunOutput, RunOutput, ScriptRunResult,
};
use crate::error::{Error, Result};
use crate::utils::temp::TempScript;
use clap::Args;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

// =============================================================================
// Semaphore for job limiting
// =============================================================================

/// Simple counting semaphore for limiting concurrent jobs
struct Semaphore {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    /// Create a new semaphore with the given number of permits
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            condvar: Condvar::new(),
        }
    }

    /// Acquire a permit, blocking until one is available
    fn acquire(&self) -> SemaphoreGuard<'_> {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.condvar.wait(permits).unwrap();
        }
        *permits -= 1;
        SemaphoreGuard { semaphore: self }
    }
}

/// RAII guard that releases the permit when dropped
struct SemaphoreGuard<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        let mut permits = self.semaphore.permits.lock().unwrap();
        *permits += 1;
        self.semaphore.condvar.notify_one();
    }
}

/// Number of log lines to show on failure (default mode)
const FAILURE_CONTEXT_LINES: usize = 5;

/// Number of log lines to show on failure when tracing is active
const TRACE_CONTEXT_LINES: usize = 30;

/// Return the last `n` lines of `text`, with a leading omission notice if truncated.
///
/// If the text has `n` or fewer lines, it is returned unchanged.
fn tail_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= n {
        return text.to_string();
    }
    let omitted = lines.len() - n;
    let mut result = format!("   ... ({} lines omitted)\n\n", omitted);
    for (i, line) in lines[lines.len() - n..].iter().enumerate() {
        result.push_str(line);
        if i < n - 1 {
            result.push('\n');
        }
    }
    result
}

/// Strip trailing error machinery from clean log output.
///
/// Removes trailing `r(N);`, `end of do-file`, `--Break--`, and blank lines
/// that are redundant with the Error: line shown separately.
fn strip_error_trailer(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut end = lines.len();
    while end > 0 {
        let trimmed = lines[end - 1].trim();
        if trimmed.is_empty()
            || trimmed == "end of do-file"
            || trimmed == "--Break--"
            || (trimmed.starts_with("r(") && trimmed.ends_with(");"))
        {
            end -= 1;
        } else {
            break;
        }
    }
    lines[..end].join("\n")
}

/// Print failure log context as an indented block under the Log: line.
///
/// Shows last N lines of clean output, stripped of error trailer.
fn print_log_context_n(clean_output: &str, n: usize) {
    let stripped = strip_error_trailer(clean_output);
    if stripped.is_empty() {
        return;
    }
    let lines: Vec<&str> = stripped.lines().collect();
    if lines.len() > n {
        let omitted = lines.len() - n;
        eprintln!("        ... ({} lines omitted)", omitted);
    }
    let start = lines.len().saturating_sub(n);
    for line in &lines[start..] {
        eprintln!("        {}", line);
    }
}

/// Print failure log context with the default number of context lines.
fn print_log_context(clean_output: &str) {
    print_log_context_n(clean_output, FAILURE_CONTEXT_LINES);
}

/// Prepend Stata trace commands to code.
fn prepend_trace(code: &str, depth: u32) -> String {
    format!("set trace on\nset tracedepth {}\n{}", depth, code)
}

/// Source of the Stata code being executed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeSource {
    /// Code from a .do file
    File,
    /// Inline code from -c/--code flag
    Inline,
}

/// Resolve verbosity from CLI flags with TTY-awareness
fn resolve_verbosity(
    quiet: bool,
    verbose: u8,
    format: OutputFormat,
) -> crate::executor::verbosity::Verbosity {
    use crate::executor::verbosity::Verbosity;

    if quiet || format.is_machine_readable() {
        Verbosity::Quiet
    } else {
        match verbose {
            0 if std::io::stdout().is_terminal() => Verbosity::DefaultInteractive,
            0 => Verbosity::PipedDefault,
            1 => Verbosity::Verbose,
            _ => Verbosity::VeryVerbose,
        }
    }
}

#[derive(Args, Clone)]
#[command(after_help = "\
Examples:
  stacy run analysis.do                   Run a script
  stacy run a.do b.do c.do                Run multiple scripts sequentially
  stacy run --parallel *.do               Run scripts in parallel
  stacy run --parallel -j4 *.do           Parallel with max 4 concurrent jobs
  stacy run -c 'display 1+1'              Run inline code
  stacy run -c 'cmd1' -c 'cmd2'           Multiple inline commands
  stacy run - <<< 'display 1'             Read code from stdin
  stacy run -C reports/ table.do          Run in specific directory
  stacy run --cd reports/table.do         Auto cd to script's directory
  stacy run script.do --engine /path/to/stata
                                        Use specific Stata binary
  stacy run script.do -v                  Stream log output in real-time
  stacy run script.do --format json       Machine-readable output
  stacy run script.do --trace 2           Trace execution at depth 2
  stacy run script.do --trace 2 -v        Trace + stream live

Tips:
  Use -c (not -e) for inline code")]
pub struct RunArgs {
    /// Stata scripts to run (multiple allowed)
    #[arg(value_name = "SCRIPT", required_unless_present = "code")]
    pub scripts: Vec<PathBuf>,

    /// Execute inline Stata code instead of a script file (can be repeated)
    #[arg(
        short = 'c',
        long = "code",
        conflicts_with_all = ["scripts", "directory", "cd"],
        value_name = "CODE",
        action = clap::ArgAction::Append,
    )]
    pub code: Vec<String>,

    /// Run Stata in this directory instead of the current directory
    #[arg(
        short = 'C',
        long = "directory",
        conflicts_with = "cd",
        value_name = "DIR"
    )]
    pub directory: Option<PathBuf>,

    /// Change to the script's parent directory before running.
    /// The script path is resolved to absolute first.
    #[arg(long, conflicts_with = "directory")]
    pub cd: bool,

    /// Run scripts in parallel
    #[arg(long, short = 'P')]
    pub parallel: bool,

    /// Max parallel jobs (default: CPU count)
    #[arg(short = 'j', long, requires = "parallel")]
    pub jobs: Option<usize>,

    /// Suppress all output, even error context (for CI/batch)
    #[arg(short, long, conflicts_with_all = ["verbose"])]
    pub quiet: bool,

    /// Verbose output: stream log file in real-time (-v)
    /// Very verbose: show execution details + stream (-vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,

    /// Stata engine to use (overrides config and auto-detection)
    #[arg(long, value_name = "ENGINE")]
    pub engine: Option<String>,

    /// Show detailed performance profiling metrics
    #[arg(long)]
    pub profile: bool,

    /// Enable build cache (skip re-execution if script/deps unchanged)
    #[arg(long)]
    pub cache: bool,

    /// Force rebuild even if cached
    #[arg(long, requires = "cache")]
    pub force: bool,

    /// Fail if not in cache (useful for CI)
    #[arg(long, requires = "cache")]
    pub cache_only: bool,

    /// Allow globally installed packages (PLUS, PERSONAL, etc.) in addition to locked packages.
    /// By default, only locked packages and Stata's BASE are available (strict mode).
    /// Use this for convenience during development when using unlocked packages.
    #[arg(long)]
    pub allow_global: bool,

    /// Enable Stata execution tracing at given depth (set trace on, set tracedepth N)
    #[arg(long, value_name = "DEPTH", conflicts_with_all = ["quiet", "parallel"])]
    pub trace: Option<u32>,
}

/// Check if a path is the stdin marker "-"
fn is_stdin_marker(path: &Path) -> bool {
    path.as_os_str() == "-"
}

/// Read code from stdin
fn read_stdin() -> Result<String> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Err(Error::Config(
            "stdin is a terminal - pipe code or use -c flag instead\n\
             Example: echo 'display 1' | stacy run -"
                .into(),
        ));
    }
    let mut buffer = String::new();
    stdin.lock().read_to_string(&mut buffer)?;
    if buffer.trim().is_empty() {
        return Err(Error::Config("stdin is empty".into()));
    }
    Ok(buffer)
}

/// Main entry point - dispatches to appropriate execution mode
pub fn execute(args: &RunArgs) -> Result<()> {
    use std::process;

    // Check for stdin marker
    if args.scripts.len() == 1 && is_stdin_marker(&args.scripts[0]) {
        if !args.code.is_empty() {
            return Err(Error::Config("cannot use stdin (-) with -c flag".into()));
        }
        let code = read_stdin()?;
        let mut modified_args = args.clone();
        modified_args.code = vec![code];
        modified_args.scripts.clear();
        return execute_inline(&modified_args);
    }

    // Handle inline code mode
    if !args.code.is_empty() {
        return execute_inline(args);
    }

    // Dispatch based on number of scripts and parallel flag
    match (args.scripts.len(), args.parallel) {
        (0, _) => {
            // This shouldn't happen due to required_unless_present
            unreachable!("clap should require either script or code")
        }
        (1, _) => execute_single(&args.scripts[0], args),
        (_, true) => execute_parallel(args),
        (_, false) => execute_sequential(args),
    }?;

    // Note: execute_* functions call process::exit() internally
    process::exit(0);
}

/// Resolve the effective working directory from --cd or -C flags.
/// Also resolves the script path to absolute before changing directory.
fn resolve_working_dir(script: &Path, args: &RunArgs) -> Result<(PathBuf, Option<PathBuf>)> {
    // Resolve script to absolute path first (before any directory change)
    let abs_script = if script.is_absolute() {
        script.to_path_buf()
    } else {
        std::env::current_dir()?.join(script)
    };

    if args.cd {
        // --cd: use the script's parent directory
        let parent = abs_script
            .parent()
            .ok_or_else(|| {
                crate::error::Error::Config(format!(
                    "Cannot determine parent directory of: {}",
                    script.display()
                ))
            })?
            .to_path_buf();
        Ok((abs_script, Some(parent)))
    } else if let Some(ref dir) = args.directory {
        // -C <dir>: use the specified directory
        let abs_dir = if dir.is_absolute() {
            dir.clone()
        } else {
            std::env::current_dir()?.join(dir)
        };
        // Validate that the directory exists
        if !abs_dir.is_dir() {
            return Err(crate::error::Error::Config(format!(
                "Directory not found: {}",
                dir.display()
            )));
        }
        Ok((abs_script, Some(abs_dir)))
    } else {
        Ok((abs_script, None))
    }
}

/// Warn if semicolons detected in inline code (Stata uses newlines)
fn warn_if_semicolons(code_snippets: &[String]) {
    // Skip if #delimit is used (intentional semicolon mode)
    let uses_delimit = code_snippets
        .iter()
        .any(|c| c.to_lowercase().contains("#delimit"));
    if uses_delimit {
        return;
    }

    if code_snippets.iter().any(|c| c.contains(';')) {
        eprintln!("\x1b[33mwarning\x1b[0m: semicolons detected in inline code");
        eprintln!();
        eprintln!("  Stata uses newlines (not semicolons) to separate commands.");
        eprintln!();
        eprintln!("  Alternatives:");
        eprintln!("    stacy run -c 'cmd1' -c 'cmd2'       # multiple -c flags");
        eprintln!("    stacy run -c $'cmd1\\ncmd2'          # bash newline syntax");
        eprintln!("    stacy run - <<< $'cmd1\\ncmd2'       # heredoc");
        eprintln!("    stacy run -c '#delimit ;' -c 'cmd1; cmd2;'  # Stata delimit mode");
        eprintln!();
    }
}

/// Execute inline code (-c flag)
fn execute_inline(args: &RunArgs) -> Result<()> {
    use crate::executor::StataExecutor;
    use crate::metrics::Metrics;
    use std::process;

    let format = args.format;

    // Warn about semicolons before joining
    warn_if_semicolons(&args.code);

    // Join multiple -c arguments with newlines
    let mut code = args.code.join("\n");

    // Prepend trace commands if --trace is active
    if let Some(depth) = args.trace {
        code = prepend_trace(&code, depth);
    }

    // Initialize metrics if profiling enabled
    let mut metrics = if args.profile {
        let mut m = Metrics::new();
        m.start();
        Some(m)
    } else {
        None
    };

    let verbosity = resolve_verbosity(args.quiet, args.verbose, format);

    // Create temp file for inline code
    let cwd = std::env::current_dir()?;
    let temp_script = TempScript::new(&code, &cwd)?;
    let script_path = temp_script.path().to_path_buf();

    // Create executor
    if let Some(ref mut m) = metrics {
        m.start_phase("setup");
    }

    let project = crate::project::Project::find()?;
    let engine_ref = args.engine.as_deref();
    let executor =
        StataExecutor::try_new(engine_ref, verbosity)?.with_allow_global(args.allow_global);
    let project_root = project.as_ref().map(|p| p.root.as_path());

    if let Some(ref mut m) = metrics {
        m.end_phase("setup");
        m.start_phase("execution");
    }

    // Show "Running..." indicator for interactive modes
    if verbosity.should_show_running_indicator() {
        eprintln!("Running <inline code>...");
    }

    // Run Stata
    let mut result = executor.run(&script_path, project_root)?;

    if let Some(ref mut m) = metrics {
        m.end_phase("execution");
        m.record_phase("parse", result.parse_duration);
        m.end();
        result.metrics = Some(m.clone());
    }

    // Build output
    let output = RunOutput {
        success: result.success,
        exit_code: result.exit_code,
        duration_secs: result.duration.as_secs_f64(),
        error_count: result.errors.len(),
        source: "inline".to_string(),
        script: script_path.clone(),
        log_file: result.log_file.clone(),
    };

    // Handle output based on format
    match format {
        OutputFormat::Json => {
            print_json_output(&result, &script_path, CodeSource::Inline, args.profile)?;
        }
        OutputFormat::Stata => {
            println!("{}", output.to_stata());
        }
        OutputFormat::Human => {
            let tracing = args.trace.is_some();

            if verbosity.should_stream_raw() {
                eprintln!();
                eprintln!("─────────────────────────────────────────────────────────────");
            }

            if !result.success {
                // ALWAYS show FAIL + error details on failure (even in quiet mode)
                eprintln!(
                    "\x1b[31mFAIL\x1b[0m  <inline code>  ({:.2}s)",
                    result.duration.as_secs_f64()
                );
                if let Some(error) = result.errors.first() {
                    print_error_details(error);
                }
                // Log context only when not quiet and log wasn't already streamed
                if !verbosity.is_quiet()
                    && !verbosity.should_stream_raw()
                    && !verbosity.should_stream_clean()
                {
                    let context_lines = if tracing {
                        TRACE_CONTEXT_LINES
                    } else {
                        FAILURE_CONTEXT_LINES
                    };
                    if let Ok(raw) = crate::executor::log_reader::read_full_log(&result.log_file) {
                        let clean = crate::executor::log_reader::strip_boilerplate(&raw);
                        if !clean.is_empty() {
                            eprintln!();
                            eprintln!("   Log:");
                            print_log_context_n(&clean, context_lines);
                        }
                    }
                }
            } else if !verbosity.is_quiet() {
                eprintln!(
                    "\x1b[32mPASS\x1b[0m  <inline code>  ({:.2}s)",
                    result.duration.as_secs_f64()
                );
                // Show clean output post-hoc only when not already streamed and not tracing
                if !tracing && verbosity.should_show_clean_output_post_hoc() {
                    if let Ok(raw) = crate::executor::log_reader::read_full_log(&result.log_file) {
                        let clean = crate::executor::log_reader::strip_boilerplate(&raw);
                        if !clean.is_empty() {
                            println!();
                            println!("{}", clean);
                        }
                    }
                }
            }

            if args.profile {
                if let Some(ref metrics) = result.metrics {
                    eprintln!();
                    eprint!("{}", metrics.format_display());
                }
            }
        }
    }

    drop(temp_script);
    process::exit(result.exit_code);
}

/// Execute a single script (original behavior)
fn execute_single(script_path: &Path, args: &RunArgs) -> Result<()> {
    use crate::executor::StataExecutor;
    use crate::metrics::Metrics;
    use std::process;

    let format = args.format;

    // Resolve working directory from --cd or -C flags
    let (resolved_script, working_dir) = resolve_working_dir(script_path, args)?;
    let effective_script = if working_dir.is_some() {
        &resolved_script
    } else {
        script_path
    };

    // Initialize metrics if profiling enabled
    let mut metrics = if args.profile {
        let mut m = Metrics::new();
        m.start();
        Some(m)
    } else {
        None
    };

    let verbosity = resolve_verbosity(args.quiet, args.verbose, format);

    // Verify script exists (check original path, before any directory change)
    if !resolved_script.exists() {
        if format.is_machine_readable() {
            let output = RunOutput {
                success: false,
                exit_code: 3,
                duration_secs: 0.0,
                error_count: 1,
                source: "file".to_string(),
                script: script_path.to_path_buf(),
                log_file: PathBuf::new(),
            };
            match format {
                OutputFormat::Json => println!("{}", output.to_json()),
                OutputFormat::Stata => println!("{}", output.to_stata()),
                OutputFormat::Human => {}
            }
        } else {
            eprintln!("Error: Script not found: {}", script_path.display());
        }
        process::exit(3);
    }

    // Find project for cache operations
    let project = crate::project::Project::find()?;
    let project_root = project.as_ref().map(|p| p.root.as_path());
    let tracing = args.trace.is_some();

    // Warn and skip cache when tracing (trace modifies script content)
    if tracing && args.cache && !args.quiet && format == OutputFormat::Human {
        eprintln!(
            "\x1b[33mwarning\x1b[0m: --cache ignored with --trace (trace modifies script content)"
        );
    }

    // Check cache if enabled (skip when tracing)
    if args.cache && !tracing {
        if let Some(root) = project_root {
            let cache = BuildCache::load(root)?;
            let cache_status = check_cache_with_working_dir(
                &cache,
                effective_script,
                Some(root),
                working_dir.as_deref(),
                args.force,
            )?;

            match cache_status {
                CacheStatus::Hit(entry) => {
                    // Cache hit - return cached result
                    let output = CacheHitOutput {
                        success: entry.result.success,
                        exit_code: entry.result.exit_code,
                        duration_secs: entry.result.duration_secs,
                        error_count: entry.result.errors.len(),
                        source: "cache".to_string(),
                        script: script_path.to_path_buf(),
                        cached_at: entry.cached_at,
                    };

                    match format {
                        OutputFormat::Json => println!("{}", output.to_json()),
                        OutputFormat::Stata => println!("{}", output.to_stata()),
                        OutputFormat::Human => {
                            if !args.quiet {
                                if entry.result.success {
                                    eprintln!(
                                        "\x1b[32mPASS\x1b[0m  {}  ({:.2}s cached)",
                                        script_path.display(),
                                        entry.result.duration_secs
                                    );
                                } else {
                                    eprintln!(
                                        "\x1b[31mFAIL\x1b[0m  {}  ({:.2}s cached)",
                                        script_path.display(),
                                        entry.result.duration_secs
                                    );
                                }
                            }
                        }
                    }

                    process::exit(entry.result.exit_code);
                }
                CacheStatus::Miss(reason) => {
                    // Cache miss - if cache_only mode, fail
                    if args.cache_only {
                        if !args.quiet && format == OutputFormat::Human {
                            eprintln!("Error: Cache miss ({}): {}", reason, script_path.display());
                        }
                        process::exit(5);
                    }

                    // Otherwise, continue with execution
                    if !args.quiet && format == OutputFormat::Human && args.verbose > 0 {
                        eprintln!("Cache miss ({}): rebuilding...", reason);
                    }
                }
            }
        }
    }

    // Create executor
    if let Some(ref mut m) = metrics {
        m.start_phase("setup");
    }

    let engine_ref = args.engine.as_deref();
    let executor =
        StataExecutor::try_new(engine_ref, verbosity)?.with_allow_global(args.allow_global);

    if let Some(ref mut m) = metrics {
        m.end_phase("setup");
        m.start_phase("execution");
    }

    // Show "Running..." indicator for interactive modes
    if verbosity.should_show_running_indicator() {
        let name = script_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| script_path.display().to_string());
        eprintln!("Running {}...", name);
    }

    // Run Stata (with trace injection if active)
    let _trace_temp_script; // keep TempScript alive until after execution
    let mut result = if let Some(depth) = args.trace {
        // Read the original script, prepend trace commands, run via TempScript
        let original_code = std::fs::read_to_string(effective_script).map_err(|e| {
            crate::error::Error::Config(format!(
                "Cannot read script for tracing: {}: {}",
                effective_script.display(),
                e
            ))
        })?;
        let traced_code = prepend_trace(&original_code, depth);
        let temp_dir = working_dir
            .as_deref()
            .unwrap_or_else(|| effective_script.parent().unwrap_or(Path::new(".")));
        let temp_script = TempScript::new(&traced_code, temp_dir)?;
        let temp_path = temp_script.path().to_path_buf();
        _trace_temp_script = Some(temp_script);
        if let Some(ref dir) = working_dir {
            executor.run_in_dir(&temp_path, project_root, dir)?
        } else {
            executor.run(&temp_path, project_root)?
        }
    } else if let Some(ref dir) = working_dir {
        _trace_temp_script = None;
        executor.run_in_dir(effective_script, project_root, dir)?
    } else {
        _trace_temp_script = None;
        executor.run(effective_script, project_root)?
    };

    if let Some(ref mut m) = metrics {
        m.end_phase("execution");
        m.record_phase("parse", result.parse_duration);
        m.end();
        result.metrics = Some(m.clone());
    }

    // Update cache if enabled and we have a project root (skip when tracing)
    if args.cache && !tracing {
        if let Some(root) = project_root {
            if let Err(e) = update_cache(root, effective_script, &result, working_dir.as_deref()) {
                // Log warning but don't fail execution
                if !args.quiet && format == OutputFormat::Human {
                    eprintln!("Warning: Failed to update cache: {}", e);
                }
            }
        }
    }

    // Build output
    let output = RunOutput {
        success: result.success,
        exit_code: result.exit_code,
        duration_secs: result.duration.as_secs_f64(),
        error_count: result.errors.len(),
        source: "file".to_string(),
        script: script_path.to_path_buf(),
        log_file: result.log_file.clone(),
    };

    // Handle output based on format
    match format {
        OutputFormat::Json => {
            print_json_output(&result, script_path, CodeSource::File, args.profile)?;
        }
        OutputFormat::Stata => {
            println!("{}", output.to_stata());
        }
        OutputFormat::Human => {
            if verbosity.should_stream_raw() {
                eprintln!();
                eprintln!("─────────────────────────────────────────────────────────────");
            }

            if !result.success {
                // ALWAYS show FAIL + error details on failure (even in quiet mode)
                eprintln!(
                    "\x1b[31mFAIL\x1b[0m  {}  ({:.2}s)",
                    script_path.display(),
                    result.duration.as_secs_f64()
                );
                if let Some(error) = result.errors.first() {
                    print_error_details(error);
                }
                // Log context only when not quiet and log wasn't already streamed
                if !verbosity.is_quiet()
                    && !verbosity.should_stream_raw()
                    && !verbosity.should_stream_clean()
                {
                    let context_lines = if tracing {
                        TRACE_CONTEXT_LINES
                    } else {
                        FAILURE_CONTEXT_LINES
                    };
                    eprintln!("\n   Log: {}", result.log_file.display());
                    if let Ok(raw) = crate::executor::log_reader::read_full_log(&result.log_file) {
                        let clean = crate::executor::log_reader::strip_boilerplate(&raw);
                        if !clean.is_empty() {
                            print_log_context_n(&clean, context_lines);
                        }
                    }
                }
            } else if !verbosity.is_quiet() {
                eprintln!(
                    "\x1b[32mPASS\x1b[0m  {}  ({:.2}s)",
                    script_path.display(),
                    result.duration.as_secs_f64()
                );
                // Show clean output post-hoc only when not already streamed and not tracing
                if !tracing && verbosity.should_show_clean_output_post_hoc() {
                    if let Ok(raw) = crate::executor::log_reader::read_full_log(&result.log_file) {
                        let clean = crate::executor::log_reader::strip_boilerplate(&raw);
                        if !clean.is_empty() {
                            println!();
                            println!("{}", clean);
                        }
                    }
                }
            }

            if args.profile {
                if let Some(ref metrics) = result.metrics {
                    eprintln!();
                    eprint!("{}", metrics.format_display());
                }
            }
        }
    }

    process::exit(result.exit_code);
}

/// Update the build cache after execution
fn update_cache(
    project_root: &Path,
    script_path: &Path,
    result: &crate::executor::ExecutionResult,
    working_dir: Option<&Path>,
) -> Result<()> {
    let mut cache = BuildCache::load(project_root)?;

    // Compute hashes
    let hashes = hash_dependency_tree(script_path)?;
    let lockfile_hash = hash_lockfile(project_root)?;
    let working_dir_hash = hash_working_dir(working_dir);

    // Convert errors to cached format
    let cached_errors: Vec<CachedError> = result
        .errors
        .iter()
        .map(|e| match e {
            crate::error::StataError::StataCode {
                r_code,
                message,
                line_number,
                ..
            } => CachedError {
                error_type: "StataCode".to_string(),
                r_code: Some(*r_code),
                message: message.clone(),
                line_number: *line_number,
            },
            crate::error::StataError::ProcessKilled { exit_code } => CachedError {
                error_type: "ProcessKilled".to_string(),
                r_code: None,
                message: format!("Process killed with exit code {}", exit_code),
                line_number: None,
            },
        })
        .collect();

    // Create cache entry
    let entry = CacheEntry::with_working_dir(
        hashes.script_hash,
        hashes.dependency_hashes,
        lockfile_hash,
        working_dir_hash,
        CachedResult {
            exit_code: result.exit_code,
            success: result.success,
            duration_secs: result.duration.as_secs_f64(),
            errors: cached_errors,
        },
    );

    cache.insert(script_path, entry);
    cache.save(project_root)?;

    Ok(())
}

/// Execute multiple scripts sequentially (fail-fast)
fn execute_sequential(args: &RunArgs) -> Result<()> {
    use crate::executor::StataExecutor;
    use std::process;

    let format = args.format;
    let scripts = &args.scripts;

    let verbosity = resolve_verbosity(args.quiet, args.verbose, format);

    // Resolve working directories and check all scripts exist first
    let mut resolved_scripts: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    for script in scripts {
        let (abs_script, work_dir) = resolve_working_dir(script, args)?;
        if !abs_script.exists() {
            if !args.quiet && format == OutputFormat::Human {
                eprintln!("Error: Script not found: {}", script.display());
            }
            process::exit(3);
        }
        resolved_scripts.push((abs_script, work_dir));
    }

    // Create executor
    let project = crate::project::Project::find()?;
    let engine_ref = args.engine.as_deref();
    let executor =
        StataExecutor::try_new(engine_ref, verbosity)?.with_allow_global(args.allow_global);
    let project_root = project.as_ref().map(|p| p.root.as_path());

    let start = Instant::now();
    let mut results: Vec<ScriptRunResult> = Vec::new();

    if !verbosity.is_quiet() && format == OutputFormat::Human {
        eprintln!("Running {} scripts sequentially...\n", scripts.len());
    }

    // Execute scripts sequentially, fail-fast on error
    for (i, script) in scripts.iter().enumerate() {
        let (ref abs_script, ref work_dir) = resolved_scripts[i];

        // When tracing, read file, prepend trace commands, run via TempScript
        let _trace_temp_script;
        let result = if let Some(depth) = args.trace {
            let original_code = std::fs::read_to_string(abs_script).map_err(|e| {
                crate::error::Error::Config(format!(
                    "Cannot read script for tracing: {}: {}",
                    abs_script.display(),
                    e
                ))
            })?;
            let traced_code = prepend_trace(&original_code, depth);
            let temp_dir = work_dir
                .as_deref()
                .unwrap_or_else(|| abs_script.parent().unwrap_or(Path::new(".")));
            let temp_script = TempScript::new(&traced_code, temp_dir)?;
            let temp_path = temp_script.path().to_path_buf();
            _trace_temp_script = Some(temp_script);
            if let Some(ref dir) = work_dir {
                executor.run_in_dir(&temp_path, project_root, dir)?
            } else {
                executor.run(&temp_path, project_root)?
            }
        } else {
            _trace_temp_script = None;
            if let Some(ref dir) = work_dir {
                executor.run_in_dir(abs_script, project_root, dir)?
            } else {
                executor.run(script, project_root)?
            }
        };

        let script_result = ScriptRunResult {
            script: script.clone(),
            success: result.success,
            exit_code: result.exit_code,
            duration_secs: result.duration.as_secs_f64(),
            log_file: result.log_file.clone(),
            error_message: if !result.success {
                result.errors.first().map(format_stata_error)
            } else {
                None
            },
        };

        // Print progress in human mode
        if !verbosity.is_quiet() && format == OutputFormat::Human {
            print_script_result(&script_result, i + 1, scripts.len());
        }

        let failed = !script_result.success;
        results.push(script_result);

        // Fail-fast: stop on first error
        if failed {
            break;
        }
    }

    let total_duration = start.elapsed();
    let passed = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();

    // Determine exit code: highest severity
    let exit_code = results.iter().map(|r| r.exit_code).max().unwrap_or(0);

    // Build output
    let output = ParallelRunOutput {
        success: failed == 0,
        exit_code,
        duration_secs: total_duration.as_secs_f64(),
        parallel: false,
        jobs: 1,
        passed,
        failed,
        total: results.len(),
        scripts: results,
    };

    // Handle output based on format
    match format {
        OutputFormat::Json => {
            println!("{}", output.to_json());
        }
        OutputFormat::Stata => {
            println!("{}", output.to_stata());
        }
        OutputFormat::Human => {
            if !verbosity.is_quiet() {
                print_summary(&output);
            } else if failed > 0 {
                eprintln!("{} of {} scripts failed", failed, output.total);
            }
        }
    }

    process::exit(exit_code);
}

/// Execute multiple scripts in parallel
fn execute_parallel(args: &RunArgs) -> Result<()> {
    use crate::executor::{verbosity::Verbosity, StataExecutor};
    use std::process;
    use std::sync::mpsc;

    let format = args.format;
    let scripts = &args.scripts;
    let total_scripts = scripts.len();

    // Verbose mode disabled with parallel (log interleaving would be unreadable)
    if args.verbose > 0 && !args.quiet && format == OutputFormat::Human {
        eprintln!("Warning: --verbose is ignored with --parallel (logs would interleave)");
    }

    // Always use quiet verbosity for parallel execution
    let verbosity = Verbosity::Quiet;

    // Resolve working directories and check all scripts exist first
    let mut resolved_scripts: Vec<(PathBuf, PathBuf, Option<PathBuf>)> = Vec::new();
    for script in scripts {
        let (abs_script, work_dir) = resolve_working_dir(script, args)?;
        if !abs_script.exists() {
            if !args.quiet && format == OutputFormat::Human {
                eprintln!("Error: Script not found: {}", script.display());
            }
            process::exit(3);
        }
        resolved_scripts.push((script.clone(), abs_script, work_dir));
    }

    // Determine job count
    let max_jobs = args.jobs.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    // Create executor
    let project = crate::project::Project::find()?;
    let engine_ref = args.engine.as_deref();
    let executor =
        StataExecutor::try_new(engine_ref, verbosity)?.with_allow_global(args.allow_global);
    let project_root = project.as_ref().map(|p| p.root.as_path());

    if !args.quiet && format == OutputFormat::Human {
        eprintln!(
            "Running {} scripts in parallel ({} jobs)...\n",
            total_scripts, max_jobs
        );
    }

    let start = Instant::now();

    // Use mpsc channel to stream results as they complete
    let (tx, rx) = mpsc::channel::<ScriptRunResult>();
    let semaphore = Arc::new(Semaphore::new(max_jobs));

    // Spawn all threads
    std::thread::scope(|s| {
        for (script, abs_script, work_dir) in &resolved_scripts {
            let tx = tx.clone();
            let semaphore = Arc::clone(&semaphore);
            let executor = &executor;

            s.spawn(move || {
                let _permit = semaphore.acquire();
                let result = if let Some(ref dir) = work_dir {
                    executor.run_in_dir(abs_script, project_root, dir)
                } else {
                    executor.run(script, project_root)
                };

                let script_result = match result {
                    Ok(result) => ScriptRunResult {
                        script: script.clone(),
                        success: result.success,
                        exit_code: result.exit_code,
                        duration_secs: result.duration.as_secs_f64(),
                        log_file: result.log_file.clone(),
                        error_message: if !result.success {
                            result.errors.first().map(format_stata_error)
                        } else {
                            None
                        },
                    },
                    Err(e) => ScriptRunResult {
                        script: script.clone(),
                        success: false,
                        exit_code: 5, // Internal error
                        duration_secs: 0.0,
                        log_file: PathBuf::new(),
                        error_message: Some(e.to_string()),
                    },
                };

                // Send result immediately when done (ignore send errors if receiver dropped)
                let _ = tx.send(script_result);
            });
        }

        // Drop the original sender so the receiver knows when all threads are done
        drop(tx);

        // Collect results as they arrive, printing in human mode
        let mut script_results = Vec::with_capacity(total_scripts);
        let mut completed = 0;

        for result in rx {
            completed += 1;

            // Print progress in human mode (streaming output)
            if !args.quiet && format == OutputFormat::Human {
                print_script_result(&result, completed, total_scripts);
            }

            script_results.push(result);
        }

        // Calculate final stats
        let total_duration = start.elapsed();
        let passed = script_results.iter().filter(|r| r.success).count();
        let failed = script_results.iter().filter(|r| !r.success).count();

        // Determine exit code: highest severity
        let exit_code = script_results
            .iter()
            .map(|r| r.exit_code)
            .max()
            .unwrap_or(0);

        // Build output
        let output = ParallelRunOutput {
            success: failed == 0,
            exit_code,
            duration_secs: total_duration.as_secs_f64(),
            parallel: true,
            jobs: max_jobs,
            passed,
            failed,
            total: script_results.len(),
            scripts: script_results,
        };

        // Handle output based on format
        match format {
            OutputFormat::Json => {
                println!("{}", output.to_json());
            }
            OutputFormat::Stata => {
                println!("{}", output.to_stata());
            }
            OutputFormat::Human => {
                if !args.quiet {
                    print_summary(&output);
                } else if failed > 0 {
                    eprintln!("{} of {} scripts failed", failed, output.total);
                }
            }
        }

        process::exit(exit_code);
    });

    // This is unreachable due to process::exit above, but needed for type checker
    #[allow(unreachable_code)]
    Ok(())
}

/// Format a StataError into a human-readable string
fn format_stata_error(err: &crate::error::StataError) -> String {
    use crate::error::StataError;

    match err {
        StataError::StataCode {
            r_code,
            message,
            line_number,
            ..
        } => {
            let line_info = line_number
                .map(|l| format!(" at line {}", l))
                .unwrap_or_default();
            format!("r({}){} - {}", r_code, line_info, message)
        }
        StataError::ProcessKilled { exit_code } => {
            format!("Process killed (exit code {})", exit_code)
        }
    }
}

/// Print a single script result with progress counter (unified format)
fn print_script_result(result: &ScriptRunResult, index: usize, total: usize) {
    use crate::cli::format::format_duration_secs;

    let name = result
        .script
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| result.script.display().to_string());

    let progress = format!("[{}/{}]", index, total);
    let duration = format_duration_secs(result.duration_secs);

    if result.success {
        eprintln!(
            "{:<7} \x1b[32mPASS\x1b[0m  {:<40} {}",
            progress, name, duration
        );
    } else {
        eprintln!(
            "{:<7} \x1b[31mFAIL\x1b[0m  {:<40} {}",
            progress, name, duration
        );
        if let Some(ref msg) = result.error_message {
            eprintln!("              {}", msg);
        }
    }
}

/// Print summary for multi-script runs
fn print_summary(output: &ParallelRunOutput) {
    eprintln!();
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mode = if output.parallel {
        "parallel"
    } else {
        "sequential"
    };

    if output.failed == 0 {
        eprintln!("Scripts: \x1b[32m{} passed\x1b[0m", output.passed);
    } else {
        eprintln!(
            "Scripts: \x1b[32m{} passed\x1b[0m, \x1b[31m{} failed\x1b[0m",
            output.passed, output.failed
        );
    }
    eprintln!("Time:    {:.2}s ({})", output.duration_secs, mode);
}

use super::format::print_error_details;

/// Print machine-readable JSON output (includes full error details)
fn print_json_output(
    result: &crate::executor::ExecutionResult,
    script: &Path,
    source: CodeSource,
    include_metrics: bool,
) -> Result<()> {
    use serde_json::json;

    let mut output = json!({
        "source": match source {
            CodeSource::File => "file",
            CodeSource::Inline => "inline",
        },
        "script": script.display().to_string(),
        "success": result.success,
        "exit_code": result.exit_code,
        "duration_secs": result.duration.as_secs_f64(),
        "log_file": result.log_file.display().to_string(),
        "error_count": result.errors.len(),
        "errors": result.errors.iter().map(|e| {
            json!({
                "type": format!("{:?}", e),
                "r_code": match e {
                    crate::error::StataError::StataCode { r_code, .. } => Some(r_code),
                    _ => None,
                },
            })
        }).collect::<Vec<_>>(),
    });

    // Add metrics if profiling enabled
    if include_metrics {
        if let Some(ref metrics) = result.metrics {
            output["metrics"] = metrics.to_json_value();
        }
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    // =========================================================================
    // Semaphore tests
    // =========================================================================

    #[test]
    fn test_semaphore_basic_acquire_release() {
        let sem = Semaphore::new(2);

        // Acquire first permit
        let _guard1 = sem.acquire();
        assert_eq!(*sem.permits.lock().unwrap(), 1);

        // Acquire second permit
        let _guard2 = sem.acquire();
        assert_eq!(*sem.permits.lock().unwrap(), 0);

        // Drop first guard, should release permit
        drop(_guard1);
        assert_eq!(*sem.permits.lock().unwrap(), 1);

        // Drop second guard
        drop(_guard2);
        assert_eq!(*sem.permits.lock().unwrap(), 2);
    }

    #[test]
    fn test_semaphore_limits_concurrency() {
        let sem = Arc::new(Semaphore::new(2));
        let active_count = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));

        thread::scope(|s| {
            for _ in 0..10 {
                let sem = Arc::clone(&sem);
                let active_count = Arc::clone(&active_count);
                let max_seen = Arc::clone(&max_seen);

                s.spawn(move || {
                    let _permit = sem.acquire();

                    // Increment active count
                    let current = active_count.fetch_add(1, Ordering::SeqCst) + 1;

                    // Update max seen
                    max_seen.fetch_max(current, Ordering::SeqCst);

                    // Simulate work
                    thread::sleep(Duration::from_millis(10));

                    // Decrement active count
                    active_count.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });

        // Max concurrent should never exceed 2
        assert!(max_seen.load(Ordering::SeqCst) <= 2);
    }

    #[test]
    fn test_semaphore_single_permit() {
        let sem = Semaphore::new(1);

        let _guard = sem.acquire();
        assert_eq!(*sem.permits.lock().unwrap(), 0);

        drop(_guard);
        assert_eq!(*sem.permits.lock().unwrap(), 1);
    }

    // =========================================================================
    // format_stata_error tests
    // =========================================================================

    #[test]
    fn test_format_stata_error_code() {
        use crate::error::{ErrorType, StataError};

        let err = StataError::StataCode {
            error_type: ErrorType::SyntaxError,
            r_code: 198,
            message: "invalid syntax".to_string(),
            line_number: Some(42),
        };

        let formatted = format_stata_error(&err);
        assert!(formatted.contains("r(198)"));
        assert!(formatted.contains("at line 42"));
        assert!(formatted.contains("invalid syntax"));
    }

    #[test]
    fn test_format_stata_error_code_no_line() {
        use crate::error::{ErrorType, StataError};

        let err = StataError::StataCode {
            error_type: ErrorType::StataError,
            r_code: 111,
            message: "observation out of range".to_string(),
            line_number: None,
        };

        let formatted = format_stata_error(&err);
        assert!(formatted.contains("r(111)"));
        assert!(!formatted.contains("at line"));
        assert!(formatted.contains("observation out of range"));
    }

    #[test]
    fn test_format_stata_error_process_killed() {
        use crate::error::StataError;

        let err = StataError::ProcessKilled { exit_code: 137 };

        let formatted = format_stata_error(&err);
        assert!(formatted.contains("Process killed"));
        assert!(formatted.contains("137"));
    }

    // =========================================================================
    // ScriptRunResult tests
    // =========================================================================

    #[test]
    fn test_script_run_result_success() {
        let result = ScriptRunResult {
            script: PathBuf::from("test.do"),
            success: true,
            exit_code: 0,
            duration_secs: 1.5,
            log_file: PathBuf::from("test.log"),
            error_message: None,
        };

        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_script_run_result_failure() {
        let result = ScriptRunResult {
            script: PathBuf::from("failing.do"),
            success: false,
            exit_code: 2,
            duration_secs: 0.5,
            log_file: PathBuf::from("failing.log"),
            error_message: Some("r(198) - syntax error".to_string()),
        };

        assert!(!result.success);
        assert_eq!(result.exit_code, 2);
        assert!(result.error_message.is_some());
    }

    // =========================================================================
    // tail_lines tests
    // =========================================================================

    #[test]
    fn test_tail_lines_short() {
        let text = "line 1\nline 2\nline 3";
        let result = tail_lines(text, 5);
        assert_eq!(result, text);
    }

    #[test]
    fn test_tail_lines_exact() {
        let text = "line 1\nline 2\nline 3";
        let result = tail_lines(text, 3);
        assert_eq!(result, text);
    }

    #[test]
    fn test_tail_lines_truncated() {
        let text = "line 1\nline 2\nline 3\nline 4\nline 5";
        let result = tail_lines(text, 2);
        assert!(result.contains("3 lines omitted"), "got: {}", result);
        assert!(result.contains("line 4"));
        assert!(result.contains("line 5"));
        assert!(!result.contains("line 1"));
        assert!(!result.contains("line 2"));
        assert!(!result.contains("line 3\n"));
    }

    #[test]
    fn test_tail_lines_single() {
        let text = "a\nb\nc\nd\ne";
        let result = tail_lines(text, 1);
        assert!(result.contains("4 lines omitted"));
        assert!(result.contains("e"));
        assert!(!result.contains("\na\n"));
    }

    // =========================================================================
    // prepend_trace tests
    // =========================================================================

    #[test]
    fn test_prepend_trace() {
        let code = "display 42";
        let result = prepend_trace(code, 2);
        assert_eq!(result, "set trace on\nset tracedepth 2\ndisplay 42");
    }

    #[test]
    fn test_prepend_trace_depth_1() {
        let code = "sysuse auto, clear\nsummarize price";
        let result = prepend_trace(code, 1);
        assert!(result.starts_with("set trace on\nset tracedepth 1\n"));
        assert!(result.ends_with("sysuse auto, clear\nsummarize price"));
    }
}
