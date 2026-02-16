//! Stata subprocess management
//!
//! This module handles:
//! - Spawning Stata in batch mode
//! - Setting environment variables (S_ADO from lockfile for package isolation)
//! - Waiting for completion
//! - Handling signals (SIGTERM, SIGINT)
//! - Collecting exit codes

use crate::error::Result;
use crate::packages::global_cache;
use crate::packages::lockfile::load_lockfile;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Result of running a Stata script
#[derive(Debug)]
pub struct RunResult {
    /// The exit code from Stata process
    pub exit_code: i32,
    /// Path to the generated log file
    pub log_file: PathBuf,
    /// How long the script took to run
    pub duration: Duration,
    /// Whether the process completed normally (not killed)
    pub completed: bool,
}

/// Options for running Stata
pub struct RunOptions<'a> {
    /// Path to Stata binary (e.g., "stata-mp" or "/Applications/StataNow/.../stata-mp")
    pub stata_binary: &'a str,
    /// Optional project root for S_ADO isolation
    pub project_root: Option<&'a Path>,
    /// Optional timeout (kill if exceeds)
    pub timeout: Option<Duration>,
    /// Arguments to pass as environment variables (STACY_ARG_*)
    pub args: std::collections::HashMap<String, String>,
    /// Allow global packages (SITE, PERSONAL, PLUS, OLDPLACE) in addition to locked packages.
    /// Default is false (strict mode) - only locked packages and BASE are available.
    pub allow_global: bool,
    /// Optional working directory for Stata execution.
    /// When set, Stata runs in this directory instead of the invoking directory.
    pub working_dir: Option<&'a Path>,
}

impl<'a> RunOptions<'a> {
    pub fn new(stata_binary: &'a str) -> Self {
        Self {
            stata_binary,
            project_root: None,
            timeout: None,
            args: std::collections::HashMap::new(),
            allow_global: false,
            working_dir: None,
        }
    }

    pub fn with_project_root(mut self, root: &'a Path) -> Self {
        self.project_root = Some(root);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_args(mut self, args: std::collections::HashMap<String, String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_allow_global(mut self, allow: bool) -> Self {
        self.allow_global = allow;
        self
    }

    pub fn with_working_dir(mut self, dir: &'a Path) -> Self {
        self.working_dir = Some(dir);
        self
    }
}

/// Run a Stata script in batch mode
///
/// # Implementation Notes
///
/// - Uses `-b -q` flags (batch mode, quiet startup)
/// - Sets S_ADO environment variable if project_root provided
/// - Log file: `{script_name}.log` in current directory
/// - Returns actual exit code from Stata (always 0 for Stata errors!)
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use stata_cli::executor::runner::{run_stata, RunOptions};
///
/// let script = Path::new("analysis.do");
/// let options = RunOptions::new("stata-mp");
/// let result = run_stata(script, options)?;
///
/// println!("Exit code: {}", result.exit_code);
/// println!("Log: {}", result.log_file.display());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_stata(script: &Path, options: RunOptions) -> Result<RunResult> {
    let start = Instant::now();

    // Build Stata command
    let mut cmd = Command::new(options.stata_binary);

    // Batch mode flags:
    // -b: batch mode (no GUI)
    // -q: quiet startup (suppress banner)
    cmd.args(["-b", "-q", "do"]);
    cmd.arg(script);

    // Suppress Stata's output to stdout/stderr (we read logs instead)
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    // Set working directory if specified
    if let Some(dir) = options.working_dir {
        cmd.current_dir(dir);
    }

    // Set S_ADO from lockfile packages in global cache.
    // By default (strict mode), only locked packages + BASE are available.
    // With allow_global, also includes SITE, PERSONAL, PLUS, OLDPLACE.
    //
    // Missing lockfile = OK (non-stacy project or no packages yet).
    // Corrupt/unreadable lockfile = hard error (isolation was intended).
    if let Some(project_root) = options.project_root {
        if let Some(lockfile) = load_lockfile(project_root)? {
            let s_ado = global_cache::build_s_ado(&lockfile, options.allow_global)?;
            cmd.env("S_ADO", s_ado);
        }
    }

    // Set STACY_ARG_* environment variables for arguments
    // Stata can read these via: local value : environment STACY_ARG_NAME
    for (key, value) in &options.args {
        let env_key = format!("STACY_ARG_{}", key.to_uppercase());
        cmd.env(&env_key, value);
    }

    // Spawn process
    let mut child = cmd.spawn()?;

    // Wait for completion (with optional timeout)
    let exit_status = if let Some(timeout) = options.timeout {
        wait_with_timeout(&mut child, timeout)?
    } else {
        child.wait()?
    };

    let duration = start.elapsed();

    // Determine log file path
    // Stata creates: {script_basename}.log in current working directory
    // e.g., "build/analysis.do" -> "analysis.log" (NOT "build/analysis.log")
    // When working_dir is set, the log lands in that directory.
    let log_basename = script
        .file_stem()
        .map(|s| PathBuf::from(s).with_extension("log"))
        .unwrap_or_else(|| script.with_extension("log"));
    let log_file = if let Some(dir) = options.working_dir {
        dir.join(&log_basename)
    } else {
        log_basename
    };

    // Extract exit code
    let exit_code = exit_code_from_status(&exit_status);

    // Check if process completed normally
    let completed = exit_status.success() || exit_code == 0;

    Ok(RunResult {
        exit_code,
        log_file,
        duration,
        completed,
    })
}

/// Wait for process with timeout
///
/// If timeout expires, kills the process with SIGTERM, then SIGKILL after 5s.
/// Uses channel-based cancellation so the watchdog is cleanly stopped when
/// the process exits before the timeout.
fn wait_with_timeout(child: &mut std::process::Child, timeout: Duration) -> Result<ExitStatus> {
    use std::sync::mpsc;
    use std::thread;

    #[cfg(unix)]
    let pid = child.id();

    let (tx, rx) = mpsc::channel();

    let watchdog = thread::spawn(move || {
        // Wait for timeout OR cancellation signal
        if rx.recv_timeout(timeout).is_err() {
            // Timeout expired, no cancel received — kill process
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);

                // SIGKILL escalation — wait 5s, then force kill if still alive
                thread::sleep(Duration::from_secs(5));
                if libc::kill(pid as i32, 0) == 0 {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
        }
        // If Ok(_) received, process exited normally — do nothing
    });

    let status = child.wait()?;
    let _ = tx.send(()); // Cancel watchdog (ignore error if thread already exited)
    let _ = watchdog.join(); // Wait for clean thread shutdown

    Ok(status)
}

/// Extract exit code from ExitStatus
///
/// On Unix, handles both normal exits and signals:
/// - Normal exit: return code
/// - Signal: 128 + signal number
fn exit_code_from_status(status: &ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(code) = status.code() {
            code
        } else if let Some(signal) = status.signal() {
            // Process killed by signal: return 128 + signal
            128 + signal
        } else {
            // Unknown exit status
            -1
        }
    }

    #[cfg(not(unix))]
    {
        status.code().unwrap_or(-1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_from_status() {
        // This is tested via integration tests with real Stata execution
    }

    #[test]
    fn test_run_options_builder() {
        let options = RunOptions::new("stata-mp")
            .with_project_root(Path::new("/tmp/project"))
            .with_timeout(Duration::from_secs(60));

        assert_eq!(options.stata_binary, "stata-mp");
        assert_eq!(options.project_root, Some(Path::new("/tmp/project")));
        assert_eq!(options.timeout, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_watchdog_cancellation() {
        // Verify that a fast-exiting process cancels the watchdog
        // without sending any signal
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc;
        use std::sync::Arc;
        use std::thread;

        let signal_sent = Arc::new(AtomicBool::new(false));
        let signal_sent_clone = signal_sent.clone();

        let (tx, rx) = mpsc::channel();

        let watchdog = thread::spawn(move || {
            if rx.recv_timeout(Duration::from_secs(10)).is_err() {
                signal_sent_clone.store(true, Ordering::SeqCst);
            }
        });

        // Simulate process exiting immediately
        tx.send(()).unwrap();
        watchdog.join().unwrap();

        assert!(
            !signal_sent.load(Ordering::SeqCst),
            "Watchdog should NOT fire when cancelled"
        );
    }

    #[test]
    fn test_corrupt_lockfile_returns_error() {
        let temp = tempfile::TempDir::new().unwrap();
        let lockfile_path = temp.path().join("stacy.lock");
        // Write invalid TOML as a corrupt lockfile
        std::fs::write(&lockfile_path, "this is not valid toml {{{").unwrap();

        let options = RunOptions::new("/nonexistent/stata-binary").with_project_root(temp.path());

        // Lockfile error should propagate BEFORE we even try to spawn Stata
        let result = run_stata(Path::new("test.do"), options);
        assert!(result.is_err(), "Corrupt lockfile should produce an error");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("stacy.lock") || err_msg.contains("TOML") || err_msg.contains("parse"),
            "Error should mention lockfile parsing: {}",
            err_msg
        );
    }

    #[test]
    fn test_missing_lockfile_no_error() {
        let temp = tempfile::TempDir::new().unwrap();
        // No stacy.lock file — should not error from S_ADO logic

        // Use a non-existent binary to ensure spawn fails (not lockfile logic)
        let options = RunOptions::new("/nonexistent/stata-binary").with_project_root(temp.path());

        let result = run_stata(Path::new("test.do"), options);
        // The error should be about spawning stata, not about lockfile
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            !err_msg.contains("stacy.lock") && !err_msg.contains("lockfile"),
            "Missing lockfile should not produce a lockfile error: {}",
            err_msg
        );
    }
}
