//! Rich test output formatting
//!
//! Provides pytest/jest-style colored output for test results.

use crate::error::error_db::lookup_error;
use crate::executor::log_reader::get_error_context;
use crate::test::runner::TestResult;
use colored::Colorize;
use std::time::Duration;

/// Terminal width for alignment (conservative default)
const TERM_WIDTH: usize = 60;

/// Format the PASS/FAIL status indicator
pub fn format_status(passed: bool) -> String {
    if passed {
        "PASS".green().bold().to_string()
    } else {
        "FAIL".red().bold().to_string()
    }
}

/// Format a single test result line with right-aligned timing
///
/// Example: "  PASS  test_success                    0.05s"
pub fn format_test_line(result: &TestResult) -> String {
    let status = format_status(result.passed);
    let duration = format!("{:.2}s", result.duration.as_secs_f64());

    // Calculate padding for right-alignment
    // Status is 4 chars visible, name varies, duration at end
    let name_width = TERM_WIDTH - 12 - duration.len(); // 12 for "  PASS  " prefix
    let name = if result.name.len() > name_width {
        format!("{}...", &result.name[..name_width - 3])
    } else {
        result.name.clone()
    };

    let padding = name_width.saturating_sub(name.len());
    format!(
        "  {}  {}{}{}",
        status,
        name,
        " ".repeat(padding),
        duration.dimmed()
    )
}

/// Extract r(CODE) from error message and return code
fn extract_error_code(error_message: &str) -> Option<u32> {
    // Look for r(NNN) pattern where NNN is the error code
    if let Some(start) = error_message.find("r(") {
        if let Some(end) = error_message[start..].find(')') {
            let code_str = &error_message[start + 2..start + end];
            return code_str.parse().ok();
        }
    }
    None
}

/// Get human-readable error description from error code
fn get_error_description(error_message: &str) -> String {
    if let Some(code) = extract_error_code(error_message) {
        if let Some(entry) = lookup_error(code) {
            return entry.message.clone();
        }
    }

    // Fallback: try to extract message after " - "
    if let Some(idx) = error_message.find(" - ") {
        return error_message[idx + 3..].to_string();
    }

    // Last resort: use the original message
    error_message.to_string()
}

/// Extract line number from error message
fn extract_line_number(error_message: &str) -> Option<u32> {
    // Look for "at line N" pattern
    if let Some(idx) = error_message.find("at line ") {
        let after = &error_message[idx + 8..];
        let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        return num_str.parse().ok();
    }
    None
}

/// Format error context for a failed test
///
/// Shows human-readable error description and location.
/// With verbose=true, also shows full log context.
pub fn format_error_context(result: &TestResult, verbose: bool) -> String {
    let mut output = String::new();

    if let Some(ref error_msg) = result.error_message {
        // Get human-readable description
        let description = get_error_description(error_msg);
        output.push_str(&format!("\n        {}\n", description.red()));

        // Show location if available
        if let Some(line_num) = extract_line_number(error_msg) {
            output.push_str(&format!(
                "        {} {}:{}\n",
                "at".dimmed(),
                result.path.display(),
                line_num
            ));
        } else {
            output.push_str(&format!(
                "        {} {}\n",
                "in".dimmed(),
                result.path.display()
            ));
        }

        // Show full log context if verbose
        if verbose {
            if let Some(ref log_file) = result.log_file {
                if let Ok(context) = get_error_context(log_file) {
                    output.push_str(&context);
                }
            }
        }
    }

    output
}

/// Format a heavy horizontal separator line
pub fn format_separator() -> String {
    "\u{2501}".repeat(TERM_WIDTH)
}

/// Format the test summary with colors
///
/// Example:
/// ```text
/// Tests:  5 passed, 7 failed
/// Time:   1.07s
/// ```
pub fn format_summary(passed: usize, failed: usize, duration: Duration) -> String {
    let mut output = String::new();

    // Tests line
    let passed_str = format!("{} passed", passed);
    let failed_str = format!("{} failed", failed);

    output.push_str(&format!(
        "Tests:  {}, {}\n",
        if passed > 0 {
            passed_str.green().to_string()
        } else {
            passed_str
        },
        if failed > 0 {
            failed_str.red().to_string()
        } else {
            failed_str
        }
    ));

    // Time line
    output.push_str(&format!("Time:   {:.2}s\n", duration.as_secs_f64()));

    output
}

/// Print a complete test result (line + error context if failed)
pub fn print_test_result(result: &TestResult, verbose: bool) {
    println!("{}", format_test_line(result));

    if !result.passed {
        print!("{}", format_error_context(result, verbose));
    }
}

/// Print the complete test suite summary
pub fn print_summary(passed: usize, failed: usize, duration: Duration) {
    println!("{}", format_separator().dimmed());
    print!("{}", format_summary(passed, failed, duration));
    println!("{}", format_separator().dimmed());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_status_pass() {
        let status = format_status(true);
        assert!(status.contains("PASS"));
    }

    #[test]
    fn test_format_status_fail() {
        let status = format_status(false);
        assert!(status.contains("FAIL"));
    }

    #[test]
    fn test_extract_error_code() {
        assert_eq!(extract_error_code("r(9) at line 13 - r(9);"), Some(9));
        assert_eq!(extract_error_code("r(601) - file not found"), Some(601));
        assert_eq!(extract_error_code("no error code here"), None);
    }

    #[test]
    fn test_extract_line_number() {
        assert_eq!(extract_line_number("r(9) at line 13 - r(9);"), Some(13));
        assert_eq!(extract_line_number("at line 42"), Some(42));
        assert_eq!(extract_line_number("no line number"), None);
    }

    #[test]
    fn test_get_error_description_with_code() {
        // Without a cache, falls through to the original message
        let desc = get_error_description("r(9) at line 13");
        // Either the cached message (if available) or the original string
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_get_error_description_fallback() {
        let desc = get_error_description("r(999) - custom error message");
        assert_eq!(desc, "custom error message");
    }

    #[test]
    fn test_format_test_line_pass() {
        let result = TestResult {
            name: "test_success".to_string(),
            path: PathBuf::from("tests/test_success.do"),
            passed: true,
            exit_code: 0,
            duration: Duration::from_millis(50),
            error_message: None,
            log_file: None,
        };

        let line = format_test_line(&result);
        assert!(line.contains("PASS"));
        assert!(line.contains("test_success"));
        assert!(line.contains("0.05s"));
    }

    #[test]
    fn test_format_test_line_fail() {
        let result = TestResult {
            name: "test_failure".to_string(),
            path: PathBuf::from("tests/test_failure.do"),
            passed: false,
            exit_code: 1,
            duration: Duration::from_millis(50),
            error_message: Some("r(9) at line 8".to_string()),
            log_file: None,
        };

        let line = format_test_line(&result);
        assert!(line.contains("FAIL"));
        assert!(line.contains("test_failure"));
    }

    #[test]
    fn test_format_summary() {
        let summary = format_summary(5, 7, Duration::from_millis(1070));
        assert!(summary.contains("5 passed"));
        assert!(summary.contains("7 failed"));
        assert!(summary.contains("1.07s"));
    }
}
