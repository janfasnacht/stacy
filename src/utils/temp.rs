//! Temporary script file management
//!
//! Handles creation and automatic cleanup of temporary .do files
//! for inline code execution via `stacy run -c 'code'`.

use crate::error::{Error, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A temporary Stata script that auto-deletes on drop.
///
/// This struct owns a temporary .do file created in the specified directory.
/// The file is automatically deleted when the struct goes out of scope,
/// ensuring cleanup even on panic or early return.
///
/// The associated log file (created by Stata) is also cleaned up.
pub struct TempScript {
    path: PathBuf,
    log_path: PathBuf,
}

impl TempScript {
    /// Create a new temporary script file with the given code.
    ///
    /// # Arguments
    ///
    /// * `code` - The Stata code to write to the file
    /// * `dir` - Directory to create the file in (typically CWD)
    ///
    /// # Returns
    ///
    /// A `TempScript` that will auto-delete when dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The code is empty
    /// - The file cannot be created or written
    pub fn new(code: &str, dir: &Path) -> Result<Self> {
        // Validate code is not empty
        if code.trim().is_empty() {
            return Err(Error::Config("Inline code cannot be empty".into()));
        }

        let filename = generate_temp_filename();
        let path = dir.join(&filename);
        let log_path = path.with_extension("log");

        // Write code to file
        let mut file = fs::File::create(&path)?;
        file.write_all(code.as_bytes())?;

        // Ensure file is flushed to disk before Stata reads it
        file.flush()?;

        Ok(Self { path, log_path })
    }

    /// Get the path to the temporary script.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the path where Stata will write the log file.
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

impl Drop for TempScript {
    fn drop(&mut self) {
        // Best-effort cleanup - don't panic if files don't exist
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(&self.log_path);
    }
}

use std::sync::atomic::{AtomicU32, Ordering};

/// Global counter for unique filename generation
static FILENAME_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Generate a unique temporary filename.
///
/// Format: `_stacy_inline_{timestamp}_{pid}_{counter}.do`
///
/// - Underscore prefix: somewhat hidden from casual `ls`
/// - Timestamp: allows sorting, aids debugging
/// - PID + counter: ensures uniqueness even in rapid succession
fn generate_temp_filename() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let pid = std::process::id();
    let counter = FILENAME_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("_stacy_inline_{}_{}_{}.do", timestamp, pid, counter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_temp_script_creation() {
        let temp_dir = TempDir::new().unwrap();
        let code = "display \"hello\"";

        let script = TempScript::new(code, temp_dir.path()).unwrap();

        // File should exist
        assert!(script.path().exists());

        // File should contain the code
        let contents = fs::read_to_string(script.path()).unwrap();
        assert_eq!(contents, code);

        // Should have .do extension
        assert_eq!(script.path().extension().unwrap(), "do");

        // Log path should have .log extension
        assert_eq!(script.log_path().extension().unwrap(), "log");
    }

    #[test]
    fn test_temp_script_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let script_path;
        let log_path;

        {
            let script = TempScript::new("display 1", temp_dir.path()).unwrap();
            script_path = script.path().to_path_buf();
            log_path = script.log_path().to_path_buf();

            // Create a fake log file to test cleanup
            fs::write(&log_path, "log content").unwrap();

            assert!(script_path.exists());
            assert!(log_path.exists());
        } // script dropped here

        // Both files should be cleaned up
        assert!(!script_path.exists());
        assert!(!log_path.exists());
    }

    #[test]
    fn test_temp_script_cleanup_no_log() {
        // Test that cleanup doesn't fail if log file doesn't exist
        let temp_dir = TempDir::new().unwrap();
        let script_path;

        {
            let script = TempScript::new("display 1", temp_dir.path()).unwrap();
            script_path = script.path().to_path_buf();
            assert!(script_path.exists());
        } // script dropped here - no log file exists

        // Script file should still be cleaned up
        assert!(!script_path.exists());
    }

    #[test]
    fn test_temp_filename_format() {
        let filename = generate_temp_filename();

        assert!(filename.starts_with("_stacy_inline_"));
        assert!(filename.ends_with(".do"));
    }

    #[test]
    fn test_temp_filenames_unique() {
        let names: Vec<_> = (0..100).map(|_| generate_temp_filename()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();

        // All names should be unique
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_empty_code_rejected() {
        let temp_dir = TempDir::new().unwrap();

        let result = TempScript::new("", temp_dir.path());
        assert!(result.is_err());

        let result = TempScript::new("   ", temp_dir.path());
        assert!(result.is_err());

        let result = TempScript::new("\n\t", temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_code() {
        let temp_dir = TempDir::new().unwrap();
        let code = "display \"cafe: 5 EUR\"";

        let script = TempScript::new(code, temp_dir.path()).unwrap();
        let contents = fs::read_to_string(script.path()).unwrap();

        assert_eq!(contents, code);
    }

    #[test]
    fn test_multiline_code() {
        let temp_dir = TempDir::new().unwrap();
        let code = "sysuse auto\ndescribe\nsummarize price";

        let script = TempScript::new(code, temp_dir.path()).unwrap();
        let contents = fs::read_to_string(script.path()).unwrap();

        assert_eq!(contents, code);
    }

    #[test]
    fn test_code_with_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let code = r#"display "hello \"world\"""#;

        let script = TempScript::new(code, temp_dir.path()).unwrap();
        let contents = fs::read_to_string(script.path()).unwrap();

        assert_eq!(contents, code);
    }
}
