//! Stata error code extraction
//!
//! Extracts all valid error codes from the user's Stata installation by running
//! `capture noisily error N` for codes 1-9999. Parses the log output and builds
//! an `ErrorDatabase` for caching.

use super::categories::category_for_code;
use super::error_db::{ErrorCodeCache, ErrorCodeEntry, ErrorDatabase};
use crate::error::Result;
use tempfile::TempDir;

/// Extract error codes from Stata by running a do-file that probes all codes 1-9999.
///
/// Returns an `ErrorDatabase` populated with all codes that Stata recognizes.
/// Takes approximately 0.3 seconds on a modern machine.
pub fn extract_error_codes(stata_binary: &str) -> Result<ErrorDatabase> {
    let tmp_dir = TempDir::new()?;
    let script_path = tmp_dir.path().join("stacy_extract_errors.do");

    // Write the extraction do-file
    let do_file_content = r#"display "STACY_EXTRACTION_START"
display "STATA_VERSION:" c(stata_version)
display "SYSDIR:" c(sysdir_stata)
forvalues i = 1/9999 {
    capture noisily error `i'
    if _rc != 0 {
        display "STACY_CODE:`i'"
    }
}
display "STACY_EXTRACTION_END"
"#;
    std::fs::write(&script_path, do_file_content)?;

    // Run Stata in batch mode
    let mut cmd = std::process::Command::new(stata_binary);
    cmd.args(["-b", "-q", "do"]);
    cmd.arg(&script_path);
    cmd.current_dir(tmp_dir.path());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    let mut child = cmd.spawn().map_err(|e| {
        crate::error::Error::Execution(format!("Failed to run Stata for error extraction: {}", e))
    })?;

    // Wait with 60-second timeout to avoid blocking forever
    let timeout = std::time::Duration::from_secs(60);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break, // Process exited (may be non-zero, that's OK)
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(crate::error::Error::Execution(
                        "Stata extraction timed out after 60 seconds".to_string(),
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                return Err(crate::error::Error::Execution(format!(
                    "Stata extraction process failed: {}",
                    e
                )));
            }
        }
    }

    // Read the log file
    let log_path = tmp_dir.path().join("stacy_extract_errors.log");
    if !log_path.exists() {
        return Err(crate::error::Error::Execution(
            "Stata did not produce a log file during error extraction".to_string(),
        ));
    }

    let log_content = std::fs::read_to_string(&log_path)?;
    parse_extraction_log(&log_content)
}

/// Parse a Stata extraction log and build an ErrorDatabase.
///
/// The log contains markers like:
/// ```text
/// STACY_EXTRACTION_START
/// STATA_VERSION:19.5
/// SYSDIR:/usr/local/stata19
/// ...error message text...
/// r(199);
/// STACY_CODE:199
/// ...
/// STACY_EXTRACTION_END
/// ```
pub fn parse_extraction_log(log_content: &str) -> Result<ErrorDatabase> {
    let lines: Vec<&str> = log_content.lines().collect();

    // Find extraction boundaries
    let start_idx = lines
        .iter()
        .position(|l| l.contains("STACY_EXTRACTION_START"));
    let end_idx = lines
        .iter()
        .position(|l| l.contains("STACY_EXTRACTION_END"));

    let (start, end) = match (start_idx, end_idx) {
        (Some(s), Some(e)) if e > s => (s, e),
        _ => {
            return Err(crate::error::Error::Parse(
                "Could not find extraction markers in Stata log".to_string(),
            ));
        }
    };

    let region = &lines[start + 1..end];

    // Extract metadata
    let mut stata_version = None;
    let mut sysdir = None;
    let mut entries = Vec::new();

    // Collect message lines between STACY_CODE markers
    let mut message_lines: Vec<&str> = Vec::new();

    for line in region {
        if let Some(ver) = line.strip_prefix("STATA_VERSION:") {
            stata_version = Some(ver.trim().to_string());
            continue;
        }
        if let Some(dir) = line.strip_prefix("SYSDIR:") {
            sysdir = Some(dir.trim().to_string());
            continue;
        }

        if let Some(code_str) = line.strip_prefix("STACY_CODE:") {
            if let Ok(code) = code_str.trim().parse::<u32>() {
                // Build the message from accumulated lines, excluding:
                // - the r(N); line
                // - the STACY_CODE:N line itself
                // - empty lines at start/end
                let message = build_message(&message_lines, code);
                let category = category_for_code(code).to_string();

                entries.push(ErrorCodeEntry {
                    code,
                    message,
                    category,
                });
            }
            message_lines.clear();
            continue;
        }

        message_lines.push(line);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            // Format as ISO 8601 date (YYYY-MM-DD)
            let secs = d.as_secs();
            let days = secs / 86400;
            // Simple date calculation from epoch days
            let (year, month, day) = epoch_days_to_date(days);
            format!("{:04}-{:02}-{:02}", year, month, day)
        })
        .unwrap_or_else(|_| "unknown".to_string());

    let mut db = ErrorDatabase::empty();
    db.stata_version = stata_version;
    db.sysdir = sysdir;
    db.extracted_at = now;
    db.errors = entries;
    db.build_index();

    Ok(db)
}

/// Build a clean error message from the lines between two STACY_CODE markers.
///
/// Filters out the `r(N);` return code line and trims whitespace.
fn build_message(lines: &[&str], code: u32) -> String {
    let r_pattern = format!("r({});", code);

    let message: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && trimmed != r_pattern
        })
        .collect();

    message.join("\n").trim().to_string()
}

/// Convert epoch days to (year, month, day)
fn epoch_days_to_date(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's date library (public domain)
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Ensure the error database is populated.
///
/// If no cache exists and a Stata binary is provided, runs extraction,
/// saves the cache, and returns the database. If cache exists, loads it.
pub fn ensure_error_database(stata_binary: &str) -> Result<()> {
    if ErrorCodeCache::exists() {
        return Ok(());
    }

    let db = extract_error_codes(stata_binary)?;
    ErrorCodeCache::save(&db)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_log() -> String {
        r#". display "STACY_EXTRACTION_START"
STACY_EXTRACTION_START

. display "STATA_VERSION:" c(stata_version)
STATA_VERSION:19.5

. display "SYSDIR:" c(sysdir_stata)
SYSDIR:/usr/local/stata19

. forvalues i = 1/9999 {
You pressed Break.  This is not considered an error.
r(1);

STACY_CODE:1
connection timed out
r(2);

STACY_CODE:2
no dataset in use
r(3);

STACY_CODE:3
unrecognized command:  foobar
r(199);

STACY_CODE:199
file mydata.dta not found
r(601);

STACY_CODE:601
. display "STACY_EXTRACTION_END"
STACY_EXTRACTION_END
"#
        .to_string()
    }

    #[test]
    fn test_parse_extraction_log() {
        let log = sample_log();
        let db = parse_extraction_log(&log).unwrap();

        assert_eq!(db.stata_version, Some("19.5".to_string()));
        assert_eq!(db.sysdir, Some("/usr/local/stata19".to_string()));
        assert_eq!(db.errors.len(), 5);

        // Check specific entries
        let e1 = db.lookup(1).unwrap();
        assert!(e1.message.contains("Break"));
        assert_eq!(e1.category, "General");

        let e199 = db.lookup(199).unwrap();
        assert!(e199.message.contains("unrecognized command"));
        assert_eq!(e199.category, "Syntax/Command");

        let e601 = db.lookup(601).unwrap();
        assert!(e601.message.contains("not found"));
        assert_eq!(e601.category, "File I/O");
    }

    #[test]
    fn test_parse_extraction_log_no_markers() {
        let log = "some random content\nwithout markers\n";
        let result = parse_extraction_log(log);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_message_filters_r_code() {
        let lines = vec!["unrecognized command:  foobar", "r(199);"];
        let msg = build_message(&lines, 199);
        assert_eq!(msg, "unrecognized command:  foobar");
    }

    #[test]
    fn test_build_message_empty() {
        let lines: Vec<&str> = vec![];
        let msg = build_message(&lines, 42);
        assert_eq!(msg, "");
    }

    #[test]
    fn test_build_message_multiline() {
        let lines = vec!["first line of message", "second line of message", "r(100);"];
        let msg = build_message(&lines, 100);
        assert_eq!(msg, "first line of message\nsecond line of message");
    }

    #[test]
    fn test_epoch_days_to_date() {
        // 2026-02-05 = day 20489 from epoch (1970-01-01)
        let (y, m, d) = epoch_days_to_date(20489);
        assert_eq!(y, 2026);
        assert_eq!(m, 2);
        assert_eq!(d, 5);
    }

    #[test]
    fn test_epoch_days_to_date_leap_year() {
        // 2024-02-29 (leap day)
        let (y, m, d) = epoch_days_to_date(19782);
        assert_eq!((y, m, d), (2024, 2, 29));
    }

    #[test]
    fn test_epoch_days_to_date_year_boundary() {
        // 2025-01-01
        let (y, m, d) = epoch_days_to_date(20089);
        assert_eq!((y, m, d), (2025, 1, 1));
    }

    /// Full end-to-end extraction test — requires Stata installed
    #[test]
    #[ignore]
    fn test_full_extraction() {
        let binary = std::env::var("STATA_BINARY").unwrap_or_else(|_| "stata-mp".to_string());
        let db = extract_error_codes(&binary).unwrap();
        assert!(
            db.errors.len() > 100,
            "Expected >100 error codes, got {}",
            db.errors.len()
        );
        assert!(db.stata_version.is_some());
    }
}
