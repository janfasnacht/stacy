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
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Cap on captured stderr bytes. Stata's startup-failure messages (license
/// seat exhausted, init errors) are tiny; this just stops a misbehaving binary
/// from ballooning memory.
const STDERR_CAPTURE_LIMIT: usize = 8 * 1024;

/// Result of running a Stata script
#[derive(Debug)]
pub struct RunResult {
    /// The exit code from Stata process
    pub exit_code: i32,
    /// Path to the generated log file
    pub log_file: PathBuf,
    /// How long the script took to run
    pub duration: Duration,
    /// Whether the process exited cleanly with code 0.
    pub completed: bool,
    /// True when the process was killed by a signal (SIGTERM/SIGINT/SIGKILL),
    /// e.g. by stacy's watchdog on timeout. Distinct from `!completed`, which
    /// also covers a clean non-zero exit (a launch failure where Stata never
    /// produced a log).
    pub signaled: bool,
    /// Captured stderr from the Stata process, lossy-decoded and capped at
    /// `STDERR_CAPTURE_LIMIT` bytes. Empty on a normal Stata run; carries the
    /// real diagnostic when Stata fails to start (license seat exhausted,
    /// missing binary, init error).
    pub stderr: String,
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
    /// Local ado directories to prepend to S_ADO (resolved to absolute paths).
    pub local_ado_paths: Vec<PathBuf>,
    /// Precomputed path where Stata will write the log file. When set, the
    /// runner uses this directly instead of deriving it from the script's stem.
    /// Callers that pass a wrapper script (see `executor::run_paths`) must set
    /// this so the log path reflects the wrapper's basename, not the user's
    /// script.
    pub log_file: Option<PathBuf>,
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
            local_ado_paths: Vec::new(),
            log_file: None,
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

    pub fn with_local_ado_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.local_ado_paths = paths;
        self
    }

    pub fn with_log_file(mut self, path: PathBuf) -> Self {
        self.log_file = Some(path);
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
/// use stacy::executor::runner::{run_stata, RunOptions};
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

    // Stata writes results to the log file, so stdout is uninteresting.
    // stderr is normally empty, but carries real diagnostics on startup
    // failures (license seat exhausted, init errors) — capture it so the
    // user isn't left with a generic "Log file incomplete" message.
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    // Set working directory if specified
    if let Some(dir) = options.working_dir {
        cmd.current_dir(dir);
    }

    // Set S_ADO from lockfile packages in global cache + local ado paths.
    // By default (strict mode), only locked packages + BASE are available.
    // With allow_global, also includes SITE, PERSONAL, PLUS, OLDPLACE.
    //
    // Missing lockfile = OK (non-stacy project or no packages yet).
    // Corrupt/unreadable lockfile = hard error (isolation was intended).
    if let Some(project_root) = options.project_root {
        let lockfile_opt = load_lockfile(project_root)?;
        let has_local_paths = !options.local_ado_paths.is_empty();

        if let Some(lockfile) = &lockfile_opt {
            let s_ado = global_cache::build_s_ado(
                lockfile,
                options.allow_global,
                &options.local_ado_paths,
            )?;
            cmd.env("S_ADO", s_ado);
        } else if has_local_paths {
            // No lockfile but local paths configured — still set S_ADO
            let empty_lockfile = crate::project::Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages: std::collections::HashMap::new(),
            };
            let s_ado = global_cache::build_s_ado(
                &empty_lockfile,
                options.allow_global,
                &options.local_ado_paths,
            )?;
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

    // Drain stderr on a background thread so the kernel pipe buffer can't
    // deadlock the child if it writes more than ~64 KiB. The reader caps the
    // captured bytes at STDERR_CAPTURE_LIMIT.
    let stderr_handle = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || -> Vec<u8> {
            let mut buf = Vec::with_capacity(512);
            let mut chunk = [0u8; 1024];
            loop {
                match pipe.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if buf.len() < STDERR_CAPTURE_LIMIT {
                            let take = (STDERR_CAPTURE_LIMIT - buf.len()).min(n);
                            buf.extend_from_slice(&chunk[..take]);
                        }
                        // Keep draining past the cap so the child doesn't block.
                    }
                    Err(_) => break,
                }
            }
            buf
        })
    });

    // Wait for completion (with optional timeout)
    let exit_status = if let Some(timeout) = options.timeout {
        wait_with_timeout(&mut child, timeout)?
    } else {
        child.wait()?
    };

    let duration = start.elapsed();

    // Collect captured stderr after the child has exited.
    let stderr = stderr_handle
        .and_then(|h| h.join().ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();

    // Determine log file path. Callers that want collision-safe parallel runs
    // pass a precomputed path via `with_log_file` (see `executor::run_paths`).
    // The fallback below preserves the legacy "{stem}.log in cwd" behavior for
    // callers that don't (notably the unit tests in this module).
    let log_file = options.log_file.clone().unwrap_or_else(|| {
        let log_basename = script
            .file_stem()
            .map(|s| PathBuf::from(s).with_extension("log"))
            .unwrap_or_else(|| script.with_extension("log"));
        match options.working_dir {
            Some(dir) => dir.join(&log_basename),
            None => log_basename,
        }
    });

    // Extract exit code
    let exit_code = exit_code_from_status(&exit_status);

    // Check if process completed normally
    let completed = exit_status.success() || exit_code == 0;

    // Was the process killed by a signal? On Unix this is the only way to
    // distinguish stacy's own watchdog (or an external SIGKILL) from a
    // binary that simply exited non-zero (a launch failure with no log).
    let signaled = signaled_from_status(&exit_status);

    Ok(RunResult {
        exit_code,
        log_file,
        duration,
        completed,
        signaled,
        stderr,
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

/// True iff the process was terminated by a signal (Unix). Always false on
/// non-Unix platforms (Windows has no equivalent concept).
fn signaled_from_status(status: &ExitStatus) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal().is_some()
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        false
    }
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

    #[test]
    fn test_run_options_with_local_ado_paths() {
        let paths = vec![
            PathBuf::from("/project/ado"),
            PathBuf::from("/project/lib/custom"),
        ];
        let options = RunOptions::new("stata-mp").with_local_ado_paths(paths.clone());

        assert_eq!(options.local_ado_paths, paths);
    }

    #[cfg(unix)]
    #[test]
    fn test_stderr_is_captured_and_capped() {
        // Stand-in for Stata: a shell script that floods stderr (well above
        // the 8 KiB cap) so we can verify both capture and bounds in one go.
        // Use write_executable (open with mode 0o755 + fsync) to avoid the
        // ETXTBSY race that Linux can briefly hit after write+chmod+exec.
        let temp = tempfile::TempDir::new().unwrap();
        let script = temp.path().join("flood.sh");
        // 1024 lines of 63 'A's + newline = ~64 KiB.
        let body = "#!/bin/sh\ni=0\nwhile [ $i -lt 1024 ]; do \
                    echo AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA >&2; \
                    i=$((i+1)); \
                    done\nexit 1\n";
        write_executable(&script, body);

        let options =
            RunOptions::new(script.to_str().unwrap()).with_log_file(temp.path().join("dummy.log"));
        let result = run_stata(Path::new("anything.do"), options).expect("spawn");

        assert!(!result.stderr.is_empty(), "stderr should be captured");
        assert!(
            result.stderr.len() <= super::STDERR_CAPTURE_LIMIT,
            "stderr should be capped at {} bytes, got {}",
            super::STDERR_CAPTURE_LIMIT,
            result.stderr.len()
        );
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, body: &str) {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o755)
            .open(path)
            .unwrap();
        f.write_all(body.as_bytes()).unwrap();
        f.sync_all().unwrap();
        // f drops here, releasing the fd before any exec.
    }

    #[test]
    fn test_local_paths_set_s_ado_without_lockfile() {
        let temp = tempfile::TempDir::new().unwrap();
        // No stacy.lock — but local ado paths are configured

        let options = RunOptions::new("/nonexistent/stata-binary")
            .with_project_root(temp.path())
            .with_local_ado_paths(vec![PathBuf::from("/project/ado")]);

        // Spawn will fail (no binary), but the error should be about spawning,
        // not about lockfile — proving local paths don't require a lockfile
        let result = run_stata(Path::new("test.do"), options);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            !err_msg.contains("stacy.lock") && !err_msg.contains("lockfile"),
            "Local paths without lockfile should not produce a lockfile error: {}",
            err_msg
        );
    }
}
