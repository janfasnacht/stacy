//! Per-invocation wrapper script + log path generation.
//!
//! Stata's `-b` mode writes the log to `<script_stem>.log` in the process cwd
//! and offers no flag to override it. When two stacy processes run scripts
//! that share a basename from a shared cwd (e.g. `module_a/build.do` and
//! `module_b/build.do` under Make `-j`), both target `./build.log` and corrupt
//! it.
//!
//! Fix: hand Stata a one-line wrapper `do "<abs_user_script>"` whose own
//! basename is unique per invocation. Stata's cwd stays as the user intended,
//! so relative paths in the user's script keep working. The log lands at
//! `<working_dir>/<unique_stem>.log`.

use crate::error::{Error, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

/// Counter to break same-process same-nanosecond ties when multiple runs
/// start in rapid succession (e.g. `--parallel`).
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Files prepared for a single Stata invocation.
///
/// Drop cleans up the wrapper file via the owned `TempDir`. The log file
/// (which lives in the user's working directory, not the tempdir) persists
/// for post-run inspection.
pub struct RunPaths {
    /// Path to the wrapper `.do` file passed to Stata (lives in the tempdir).
    pub wrapper: PathBuf,
    /// Path where Stata will write the log file (lives in `working_dir`).
    pub log: PathBuf,
    /// RAII handle that deletes the wrapper file when dropped.
    _wrapper_dir: TempDir,
}

impl RunPaths {
    /// Build a wrapper that delegates to the user's script and compute the
    /// resulting log path.
    ///
    /// `user_script` must be absolute and exist. `working_dir` must be
    /// absolute (its existence is the caller's responsibility — Stata's
    /// spawn would fail anyway).
    pub fn prepare(user_script: &Path, working_dir: &Path) -> Result<Self> {
        debug_assert!(
            user_script.is_absolute(),
            "RunPaths::prepare: user_script must be absolute, got {}",
            user_script.display()
        );
        debug_assert!(
            working_dir.is_absolute(),
            "RunPaths::prepare: working_dir must be absolute, got {}",
            working_dir.display()
        );

        let original_stem = user_script
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                Error::Config(format!(
                    "Cannot extract a usable stem from script path: {}",
                    user_script.display()
                ))
            })?;

        let unique_stem = generate_unique_stem(original_stem);

        let wrapper_dir = TempDir::with_prefix("stacy-run-")?;
        let wrapper = wrapper_dir.path().join(format!("{}.do", unique_stem));
        let log = working_dir.join(format!("{}.log", unique_stem));

        // Stata compound double-quotes (`"..."') tolerate spaces and embedded
        // single/double quotes inside the absolute path.
        let body = format!("do `\"{}\"'\n", user_script.display());

        let mut f = File::create(&wrapper)?;
        f.write_all(body.as_bytes())?;
        f.flush()?;

        Ok(RunPaths {
            wrapper,
            log,
            _wrapper_dir: wrapper_dir,
        })
    }
}

/// Build a unique stem for the wrapper/log filenames.
///
/// Format: `<sanitized_original>_<pid>_<nanos>_<counter>`. The original stem
/// is sanitized to `[A-Za-z0-9_-]` so that filename-unsafe characters in the
/// user's script name don't leak into the wrapper path.
fn generate_unique_stem(original: &str) -> String {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

    let safe: String = original
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    format!("{}_{}_{}_{}", safe, pid, nanos, counter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_unique_stem_includes_original() {
        let stem = generate_unique_stem("analysis");
        assert!(stem.starts_with("analysis_"), "got: {}", stem);
    }

    #[test]
    fn test_unique_stem_sanitizes_special_chars() {
        let stem = generate_unique_stem("café analysis");
        // 'é' and ' ' both replaced with '_'; ASCII letters preserved.
        assert!(stem.starts_with("caf__analysis_"), "got: {}", stem);
    }

    #[test]
    fn test_unique_stems_are_unique() {
        let stems: Vec<_> = (0..200).map(|_| generate_unique_stem("x")).collect();
        let unique: std::collections::HashSet<_> = stems.iter().collect();
        assert_eq!(stems.len(), unique.len(), "stems should be pairwise unique");
    }

    #[test]
    fn test_prepare_creates_wrapper_with_do_line() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("build.do");
        fs::write(&script, "display 1\n").unwrap();

        let paths = RunPaths::prepare(&script, temp.path()).unwrap();

        // Wrapper file exists and contains a single `do` line referencing the abs script.
        let body = fs::read_to_string(&paths.wrapper).unwrap();
        assert!(
            body.contains(&format!("do `\"{}\"'", script.display())),
            "wrapper body unexpected: {:?}",
            body
        );

        // Wrapper basename derives from script's stem.
        let wrapper_stem = paths.wrapper.file_stem().unwrap().to_str().unwrap();
        assert!(
            wrapper_stem.starts_with("build_"),
            "wrapper stem should start with original stem: {}",
            wrapper_stem
        );

        // Log path lives in working_dir with matching stem and .log extension.
        assert_eq!(paths.log.parent(), Some(temp.path()));
        assert_eq!(paths.log.extension().and_then(|s| s.to_str()), Some("log"));
        let log_stem = paths.log.file_stem().unwrap().to_str().unwrap();
        assert_eq!(log_stem, wrapper_stem);
    }

    #[test]
    fn test_prepare_log_path_in_working_dir_not_tempdir() {
        let working = TempDir::new().unwrap();
        let script_dir = TempDir::new().unwrap();
        let script = script_dir.path().join("ana.do");
        fs::write(&script, "display 1\n").unwrap();

        let paths = RunPaths::prepare(&script, working.path()).unwrap();

        assert!(
            paths.log.starts_with(working.path()),
            "log {} must be inside working_dir {}",
            paths.log.display(),
            working.path().display()
        );
        assert!(
            !paths.log.starts_with(script_dir.path()),
            "log must not be inside the script's source directory"
        );
    }

    #[test]
    fn test_wrapper_dir_cleaned_on_drop() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("build.do");
        fs::write(&script, "display 1\n").unwrap();

        let wrapper_path = {
            let paths = RunPaths::prepare(&script, temp.path()).unwrap();
            assert!(paths.wrapper.exists());
            paths.wrapper.clone()
        };

        assert!(
            !wrapper_path.exists(),
            "wrapper should be cleaned up when RunPaths drops"
        );
    }

    #[test]
    fn test_prepare_handles_path_with_spaces() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("my project");
        fs::create_dir(&dir).unwrap();
        let script = dir.join("build.do");
        fs::write(&script, "display 1\n").unwrap();

        let paths = RunPaths::prepare(&script, temp.path()).unwrap();
        let body = fs::read_to_string(&paths.wrapper).unwrap();
        // Compound quoting must wrap the path so spaces don't split it.
        assert!(body.contains("`\""), "compound-quote opener missing");
        assert!(body.contains("\"'"), "compound-quote closer missing");
        assert!(body.contains(&script.display().to_string()));
    }
}
