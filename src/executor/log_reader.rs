//! Log file reading utilities
//!
//! Handles reading Stata log files, which can be:
//! - Large (MB or GB for long-running scripts)
//! - Still being written (need to read final state)
//! - Missing (if Stata crashed before writing)

use crate::error::Result;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

/// Read the last N lines of a log file
///
/// This is more efficient than reading the entire file for large logs.
/// Error detection only needs the last 20-50 lines.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use stata_cli::executor::log_reader::read_last_lines;
///
/// let log = Path::new("script.log");
/// let lines = read_last_lines(log, 20)?;
///
/// for line in lines {
///     println!("{}", line);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn read_last_lines(log_file: &Path, n: usize) -> Result<Vec<String>> {
    let file = File::open(log_file)?;
    let mut reader = BufReader::new(file);

    // For small files, just read all lines
    let file_size = reader.seek(SeekFrom::End(0))?;
    if file_size < 10_000 {
        // File < 10KB, read everything
        reader.seek(SeekFrom::Start(0))?;
        let lines: Vec<String> = reader.lines().collect::<std::io::Result<_>>()?;
        return Ok(lines.into_iter().rev().take(n).rev().collect());
    }

    // For large files, read backwards from end
    // Strategy: Read last 5KB (typical for last 20-50 lines)
    let read_size = 5_000.min(file_size);
    reader.seek(SeekFrom::End(-(read_size as i64)))?;

    let lines: Vec<String> = reader.lines().collect::<std::io::Result<_>>()?;

    // Take last n lines
    Ok(lines.into_iter().rev().take(n).rev().collect())
}

/// Read the entire log file
///
/// Use sparingly - prefer read_last_lines() for error detection.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use stata_cli::executor::log_reader::read_full_log;
///
/// let log = Path::new("script.log");
/// let content = read_full_log(log)?;
///
/// println!("{}", content);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn read_full_log(log_file: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(log_file)?)
}

/// Check if log file indicates successful completion
///
/// Success = "end of do-file" with no r() error code after it
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use stata_cli::executor::log_reader::is_successful_completion;
///
/// let log = Path::new("script.log");
/// if is_successful_completion(log)? {
///     println!("Script succeeded!");
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn is_successful_completion(log_file: &Path) -> Result<bool> {
    let lines = read_last_lines(log_file, 20)?;

    // Look for "end of do-file" marker
    let has_end_marker = lines.iter().any(|line| line.contains("end of do-file"));

    if !has_end_marker {
        // Incomplete log (probably killed/interrupted)
        return Ok(false);
    }

    // Check if there's an error after the end marker
    let end_marker_idx = lines
        .iter()
        .rposition(|line| line.contains("end of do-file"))
        .unwrap();

    let lines_after_end = &lines[end_marker_idx + 1..];

    // Look for r() error codes
    let has_error = lines_after_end
        .iter()
        .any(|line| line.contains("r(") && line.contains(");"));

    Ok(!has_error)
}

/// Get error context from log file (last 20 lines, formatted)
///
/// Used for default verbosity mode - shows context when error occurs.
///
/// Returns formatted string with actual line numbers from log file.
pub fn get_error_context(log_file: &Path) -> Result<String> {
    // Read entire file to get accurate line numbers
    let content = std::fs::read_to_string(log_file)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len();

    // Get last 20 lines
    let start_idx = total_lines.saturating_sub(20);
    let last_lines = &all_lines[start_idx..];

    let mut output = String::new();
    output.push('\n');
    output.push_str("─────────────────────────────────────────────────────────────\n");
    output.push_str("Last 20 lines of log:\n");
    output.push_str("─────────────────────────────────────────────────────────────\n");

    // Show actual line numbers from file
    for (i, line) in last_lines.iter().enumerate() {
        let line_num = start_idx + i + 1; // +1 for 1-indexed

        // Highlight lines with r() codes
        if line.contains("r(") && line.contains(");") {
            output.push_str(&format!("{:3} → {}\n", line_num, line));
        } else {
            output.push_str(&format!("{:3} │ {}\n", line_num, line));
        }
    }

    output.push_str("─────────────────────────────────────────────────────────────\n");

    Ok(output)
}

/// Stream log file to stdout in real-time
///
/// Used for `-v` and `-vv` verbosity modes.
///
/// Tails the log file while Stata is running, printing new lines as they appear.
///
/// # Arguments
///
/// * `log_file` - Path to log file to stream
/// * `poll_interval` - How often to check for new lines (milliseconds)
///
/// # Returns
///
/// Returns when log file shows "end of do-file" marker or file is deleted
pub fn stream_log_file(log_file: &Path, poll_interval: std::time::Duration) -> Result<()> {
    use std::io::{BufRead, BufReader, Seek, SeekFrom};
    use std::thread::sleep;

    // Wait for log file to be created
    while !log_file.exists() {
        sleep(poll_interval);
    }

    let mut file = File::open(log_file)?;
    let mut reader = BufReader::new(file);
    let mut position = 0u64;

    loop {
        // Read new lines
        reader.seek(SeekFrom::Start(position))?;

        let mut buffer = String::new();
        let bytes_read = reader.read_line(&mut buffer)?;

        if bytes_read > 0 {
            print!("{}", buffer);
            position += bytes_read as u64;

            // Check if we've reached end of do-file
            if buffer.trim() == "end of do-file" {
                // Read a few more lines to get r() code if present
                for _ in 0..3 {
                    sleep(poll_interval);
                    buffer.clear();
                    let bytes = reader.read_line(&mut buffer)?;
                    if bytes > 0 {
                        print!("{}", buffer);
                        let _ = position + bytes as u64; // Track position but don't update since we break
                    }
                }
                break;
            }
        } else {
            // No new data, check if file still exists
            if !log_file.exists() {
                break;
            }

            // Wait before polling again
            sleep(poll_interval);

            // Reopen file in case it was truncated/recreated
            file = File::open(log_file)?;
            reader = BufReader::new(file);
        }
    }

    Ok(())
}

/// Stream clean (boilerplate-stripped) log file to stdout in real-time
///
/// Used for TTY default mode (DefaultInteractive). Filters out command echoes
/// and boilerplate, showing only substantive Stata output as it appears.
pub fn stream_log_file_clean(log_file: &Path, poll_interval: std::time::Duration) -> Result<()> {
    use std::io::{BufRead, BufReader, Seek, SeekFrom};
    use std::thread::sleep;

    // Wait for log file to be created
    while !log_file.exists() {
        sleep(poll_interval);
    }

    let mut file = File::open(log_file)?;
    let mut reader = BufReader::new(file);
    let mut position = 0u64;
    let mut seen_content = false;
    let mut prev_blank = false;

    loop {
        // Read new lines
        reader.seek(SeekFrom::Start(position))?;

        let mut buffer = String::new();
        let bytes_read = reader.read_line(&mut buffer)?;

        if bytes_read > 0 {
            position += bytes_read as u64;
            let trimmed = buffer.trim();

            // Stop at end of do-file
            if trimmed == "end of do-file" {
                break;
            }

            // Skip command echo lines
            if is_command_echo(trimmed) {
                continue;
            }

            let is_blank = trimmed.is_empty();

            // Skip leading blanks
            if is_blank && !seen_content {
                continue;
            }

            // Collapse consecutive blank lines
            if is_blank && prev_blank {
                continue;
            }

            if !is_blank {
                seen_content = true;
            }
            prev_blank = is_blank;

            // Output to stdout (this is data, not status)
            print!("{}", buffer);
        } else {
            // No new data, check if file still exists
            if !log_file.exists() {
                break;
            }

            // Wait before polling again
            sleep(poll_interval);

            // Reopen file in case it was truncated/recreated
            file = File::open(log_file)?;
            reader = BufReader::new(file);
        }
    }

    Ok(())
}

/// Check if a trimmed line is a Stata command echo
///
/// Matches:
/// - `. command` — standard command echo
/// - `.` — bare continuation dot
/// - `  2. command` — numbered continuation inside loops/programs
/// - `> continuation` — long command wrapping or `#delimit ;` continuation
pub fn is_command_echo(trimmed: &str) -> bool {
    // Standard: `. ` prefix or bare `.`
    if trimmed.starts_with(". ") || trimmed == "." {
        return true;
    }

    // Continuation lines: `> ` prefix from long commands or #delimit ; mode
    if trimmed.starts_with("> ") {
        return true;
    }

    // Numbered continuation: `2. `, `10. `, etc.
    // Pattern: optional digits followed by `. ` or just digits followed by `.`
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    // Must start with a digit
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return false;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Must be followed by `. ` or be `N.` at end of line
    if i < bytes.len() && bytes[i] == b'.' {
        // `N.` (end) or `N. ` (continuation)
        if i + 1 == bytes.len() || bytes[i + 1] == b' ' {
            return true;
        }
    }

    false
}

/// Strip Stata boilerplate from log output, returning only substantive content
///
/// Removes:
/// - Lines starting with `. ` (command echo — Stata repeating what it was told)
/// - Lines that are exactly `.` (continuation of command echo)
/// - Numbered continuation lines inside loops/programs (e.g., `  2. display x`)
/// - Leading blank lines
/// - `end of do-file` marker and everything after it (including `r(CODE);`)
/// - Collapses consecutive blank lines to a single blank line
/// - Trims trailing whitespace
pub fn strip_boilerplate(log_content: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();

    for line in log_content.lines() {
        // Stop at end-of-do-file marker
        if line.trim() == "end of do-file" {
            break;
        }

        // Skip command echo lines
        let trimmed = line.trim();
        if is_command_echo(trimmed) {
            continue;
        }

        lines.push(line);
    }

    // Remove leading blank lines
    while lines.first().is_some_and(|l| l.trim().is_empty()) {
        lines.remove(0);
    }

    // Remove trailing blank lines
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }

    // Collapse consecutive blank lines
    let mut result = String::new();
    let mut prev_blank = false;
    for line in &lines {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line.trim_end());
        prev_blank = is_blank;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_last_lines_small_file() -> Result<()> {
        let mut temp = NamedTempFile::new()?;
        writeln!(temp, "line 1")?;
        writeln!(temp, "line 2")?;
        writeln!(temp, "line 3")?;
        writeln!(temp, "line 4")?;
        writeln!(temp, "line 5")?;
        temp.flush()?;

        let lines = read_last_lines(temp.path(), 3)?;
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line 3");
        assert_eq!(lines[1], "line 4");
        assert_eq!(lines[2], "line 5");

        Ok(())
    }

    // =========================================================================
    // strip_boilerplate tests
    // =========================================================================

    #[test]
    fn test_strip_boilerplate_typical_success() {
        let log = "\n\n\
. display 1+1\n\
2\n\
\n\
. display \"hello\"\n\
hello\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "2\n\nhello");
    }

    #[test]
    fn test_strip_boilerplate_error_log() {
        let log = "\n\
. invalid_command\n\
unrecognized command:  invalid_command\n\
r(199);\n\
\n\
end of do-file\n\
\n\
r(199);\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "unrecognized command:  invalid_command\nr(199);");
    }

    #[test]
    fn test_strip_boilerplate_empty_output() {
        let log = "\n\n\
. quietly display 1+1\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_boilerplate_collapse_blank_lines() {
        let log = "\n\
. display 1\n\
1\n\
\n\
\n\
\n\
. display 2\n\
2\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "1\n\n2");
    }

    #[test]
    fn test_strip_boilerplate_no_end_marker() {
        // If log is truncated (no end-of-do-file), include everything
        let log = ". display 1\n1\n\n. display 2\n2\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "1\n\n2");
    }

    #[test]
    fn test_strip_boilerplate_bare_dot_continuation() {
        let log = "\n\
. foreach x of numlist 1/3 {\n\
.   display `x'\n\
. }\n\
1\n\
2\n\
3\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "1\n2\n3");
    }

    #[test]
    fn test_strip_boilerplate_numbered_continuation() {
        // Stata shows numbered lines inside loops in interactive-style logs
        let log = "\n\
. foreach x of numlist 1/3 {\n\
  2.   display `x'\n\
  3. }\n\
1\n\
2\n\
3\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "1\n2\n3");
    }

    #[test]
    fn test_strip_boilerplate_program_define() {
        let log = "\n\
. program define myprog\n\
  2.   display \"hello from myprog\"\n\
  3. end\n\
\n\
. myprog\n\
hello from myprog\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "hello from myprog");
    }

    #[test]
    fn test_strip_boilerplate_high_numbered_lines() {
        // Double-digit line numbers inside a long block
        let log = "\n\
. program define longprog\n\
  2.   local a = 1\n\
  3.   local b = 2\n\
  10.   display `a' + `b'\n\
  11. end\n\
\n\
. longprog\n\
3\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "3");
    }

    #[test]
    fn test_is_command_echo_standard() {
        assert!(is_command_echo(". display 1"));
        assert!(is_command_echo("."));
        assert!(is_command_echo(". foreach x of numlist 1/3 {"));
    }

    #[test]
    fn test_is_command_echo_numbered() {
        assert!(is_command_echo("2. display `x'"));
        assert!(is_command_echo("10. end"));
        assert!(is_command_echo("3."));
    }

    #[test]
    fn test_is_command_echo_continuation() {
        assert!(is_command_echo("> /\") + 1, .), \"\", .)"));
        assert!(is_command_echo("> )' _n\""));
        assert!(is_command_echo("> local x = 1"));
    }

    #[test]
    fn test_is_command_echo_not_echo() {
        assert!(!is_command_echo("hello world"));
        assert!(!is_command_echo("2"));
        assert!(!is_command_echo("r(199);"));
        assert!(!is_command_echo(""));
        assert!(!is_command_echo("123"));
        // A line like "3.14" is not command echo (no space after dot)
        assert!(!is_command_echo("3.14"));
    }

    #[test]
    fn test_is_successful_completion() -> Result<()> {
        // Test successful completion
        let mut temp = NamedTempFile::new()?;
        writeln!(temp, ". display \"hello\"")?;
        writeln!(temp, "hello")?;
        writeln!(temp, "")?;
        writeln!(temp, "end of do-file")?;
        temp.flush()?;

        assert!(is_successful_completion(temp.path())?);

        // Test failed completion
        let mut temp_fail = NamedTempFile::new()?;
        writeln!(temp_fail, ". invalid command")?;
        writeln!(temp_fail, "unrecognized command")?;
        writeln!(temp_fail, "")?;
        writeln!(temp_fail, "end of do-file")?;
        writeln!(temp_fail, "")?;
        writeln!(temp_fail, "r(199);")?;
        temp_fail.flush()?;

        assert!(!is_successful_completion(temp_fail.path())?);

        Ok(())
    }
}
