//! Retention policy for the per-invocation Stata log.
//!
//! Stata's batch mode writes `<script_stem>.log` into the process working
//! directory and offers no flag to redirect it (see `run_paths`). So the log is
//! always born next to the run, and stacy decides afterwards what happens to it:
//!
//! - `--log FILE` — the log is a durable artifact: move it to FILE, keep it
//!   whether the run passed or failed.
//! - otherwise the log is internal: removed when the run succeeded, kept when it
//!   failed.
//!
//! The rule does not depend on the output format. `--format json` and
//! `--format stata` used to keep the log of a successful run so that the
//! `log_file` they report would resolve; since the in-Stata wrappers always run
//! `--format stata`, that left one log per successful `stacy_run` behind for
//! good. They now report no log when there is none, and callers that want the
//! raw log of a passing run ask for it with `--log`.
//!
//! Kept logs land in `[run] log_dir` from `stacy.toml` when the run happened
//! inside a project — without that they piled up in the working directory (#98).

use crate::project::Project;
use std::path::{Path, PathBuf};

/// What to do with a log file once the run is over.
#[derive(Debug, Clone, Default)]
pub struct LogPolicy {
    /// Directory kept logs are moved into. `None` leaves them in place.
    keep_dir: Option<PathBuf>,
    /// Explicit destination from `--log`. Wins over everything else.
    dest: Option<PathBuf>,
}

impl LogPolicy {
    /// Internal log: removed on success, kept on failure, in the working dir.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve `[run] log_dir` against the project root. Outside a project the
    /// log stays where Stata wrote it.
    pub fn for_project(project: Option<&Project>) -> Self {
        Self {
            keep_dir: project.and_then(log_dir_for),
            ..Self::default()
        }
    }

    /// Write the log to this exact path (`--log FILE`), pass or fail.
    pub fn with_dest(mut self, dest: Option<PathBuf>) -> Self {
        self.dest = dest;
        self
    }

    /// Directory kept logs are moved into, if any.
    pub fn keep_dir(&self) -> Option<&Path> {
        self.keep_dir.as_deref()
    }

    /// Apply the policy to `log` and return the path it now lives at, or `None`
    /// when the log was removed and there is nothing left to point at.
    ///
    /// Call this only after everything that reads the log (streaming, error
    /// context, printed excerpts) is done.
    pub fn finalize(&self, log: &Path, success: bool) -> Option<PathBuf> {
        if let Some(dest) = &self.dest {
            return Some(move_log(log, dest));
        }

        if success {
            let _ = std::fs::remove_file(log);
            return None;
        }

        match (&self.keep_dir, log.file_name()) {
            (Some(dir), Some(name)) => {
                if let Err(e) = std::fs::create_dir_all(dir) {
                    eprintln!(
                        "Warning: could not create log directory {}: {}",
                        dir.display(),
                        e
                    );
                    return Some(log.to_path_buf());
                }
                Some(move_log(log, &dir.join(name)))
            }
            _ => Some(log.to_path_buf()),
        }
    }
}

/// Absolute `[run] log_dir` for a project, if the log file has somewhere to go.
fn log_dir_for(project: &Project) -> Option<PathBuf> {
    let config = project.config.as_ref()?;
    let dir = &config.run.log_dir;
    if dir.as_os_str().is_empty() {
        return None;
    }
    Some(project.root.join(dir))
}

/// Move `log` to `dest`, falling back to copy+remove across filesystems.
/// On failure the log is left where it is and its original path is returned.
fn move_log(log: &Path, dest: &Path) -> PathBuf {
    if log == dest {
        return dest.to_path_buf();
    }
    let moved = std::fs::rename(log, dest).or_else(|_| {
        std::fs::copy(log, dest).map(|_| {
            let _ = std::fs::remove_file(log);
        })
    });
    match moved {
        Ok(()) => dest.to_path_buf(),
        Err(e) => {
            eprintln!("Warning: could not write log to {}: {}", dest.display(), e);
            log.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_log(dir: &Path) -> PathBuf {
        let log = dir.join("analysis_1_2_0.log");
        fs::write(&log, "log body\n").unwrap();
        log
    }

    #[test]
    fn test_success_removes_internal_log() {
        let temp = TempDir::new().unwrap();
        let log = write_log(temp.path());

        let final_path = LogPolicy::new().finalize(&log, true);

        assert_eq!(final_path, None, "a removed log has no path to report");
        assert!(!log.exists(), "successful run should not keep its log");
    }

    #[test]
    fn test_failure_keeps_log_in_place_without_log_dir() {
        let temp = TempDir::new().unwrap();
        let log = write_log(temp.path());

        let final_path = LogPolicy::new().finalize(&log, false);

        assert_eq!(final_path, Some(log.clone()));
        assert!(log.exists(), "failed run must keep its log");
    }

    #[test]
    fn test_failure_moves_log_into_log_dir() {
        let temp = TempDir::new().unwrap();
        let log = write_log(temp.path());
        let log_dir = temp.path().join("logs");

        let policy = LogPolicy {
            keep_dir: Some(log_dir.clone()),
            ..LogPolicy::new()
        };
        let final_path = policy.finalize(&log, false).expect("failure keeps the log");

        assert_eq!(final_path, log_dir.join("analysis_1_2_0.log"));
        assert!(final_path.exists(), "kept log should be in log_dir");
        assert!(!log.exists(), "kept log should not stay in the working dir");
        assert_eq!(fs::read_to_string(&final_path).unwrap(), "log body\n");
    }

    #[test]
    fn test_success_removes_log_even_with_log_dir() {
        let temp = TempDir::new().unwrap();
        let log = write_log(temp.path());
        let log_dir = temp.path().join("logs");

        let policy = LogPolicy {
            keep_dir: Some(log_dir.clone()),
            ..LogPolicy::new()
        };
        assert_eq!(policy.finalize(&log, true), None);

        assert!(!log.exists());
        assert!(
            !log_dir.join("analysis_1_2_0.log").exists(),
            "a successful run should not populate log_dir"
        );
    }

    #[test]
    fn test_dest_wins_over_log_dir() {
        let temp = TempDir::new().unwrap();
        let log = write_log(temp.path());
        let log_dir = temp.path().join("logs");
        let dest = temp.path().join("run.log");

        let policy = LogPolicy {
            keep_dir: Some(log_dir.clone()),
            ..LogPolicy::new()
        }
        .with_dest(Some(dest.clone()));
        let final_path = policy.finalize(&log, true);

        assert_eq!(final_path, Some(dest.clone()));
        assert!(dest.exists(), "--log destination must be written");
        assert!(!log.exists());
        assert!(!log_dir.exists(), "--log must not populate log_dir");
    }

    #[test]
    fn test_dest_kept_on_failure() {
        let temp = TempDir::new().unwrap();
        let log = write_log(temp.path());
        let dest = temp.path().join("run.log");

        let policy = LogPolicy::new().with_dest(Some(dest.clone()));
        let final_path = policy.finalize(&log, false);

        assert_eq!(final_path, Some(dest.clone()));
        assert_eq!(fs::read_to_string(&dest).unwrap(), "log body\n");
    }
}
