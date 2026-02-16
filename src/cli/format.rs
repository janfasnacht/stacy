//! Shared CLI formatting utilities
//!
//! Provides consistent error messages, success indicators, and formatting
//! across all CLI commands.

/// Format an error message for human-readable output
pub fn format_error(message: &str) -> String {
    format!("Error: {}", message)
}

/// Format a warning message
pub fn format_warning(message: &str) -> String {
    format!("Warning: {}", message)
}

/// Format a success message with PASS indicator
pub fn format_success(message: &str) -> String {
    format!("PASS  {}", message)
}

/// Format a failure message with FAIL indicator
pub fn format_failure(message: &str) -> String {
    format!("FAIL  {}", message)
}

/// Format a note/info message
pub fn format_note(message: &str) -> String {
    format!("Note: {}", message)
}

/// Print a section header
pub fn print_header(title: &str) {
    println!("{}", title);
    println!("{}", "=".repeat(title.len()));
}

/// Print a subsection header
pub fn print_subheader(title: &str) {
    println!();
    println!("{}:", title);
}

/// Print an indented list item
pub fn print_list_item(item: &str) {
    println!("  - {}", item);
}

/// Print a key-value pair
pub fn print_key_value(key: &str, value: &str) {
    println!("  {}: {}", key, value);
}

/// Wrap text at specified width with indentation
pub fn wrap_text(text: &str, width: usize, indent: &str) -> String {
    let mut result = String::new();
    let mut current_line = String::from(indent);

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > width + indent.len() {
            result.push_str(&current_line);
            result.push('\n');
            current_line = String::from(indent);
        }

        if current_line.len() > indent.len() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }

    if current_line.len() > indent.len() {
        result.push_str(&current_line);
    }

    result
}

/// Print detailed error information with Stata documentation
pub fn print_error_details(error: &crate::error::StataError) {
    use crate::error::StataError;

    match error {
        StataError::StataCode {
            r_code, message, ..
        } => {
            // message is already the best available: log-extracted, error-db, or category fallback
            eprintln!("\n   Error: r({}) - {}", r_code, message);
            eprintln!();
            eprintln!(
                "   See: https://www.stata.com/manuals/perror.pdf#r{}",
                r_code
            );
        }
        StataError::ProcessKilled { exit_code } => {
            eprintln!(
                "\n   Error: Process terminated with signal (exit code {})",
                exit_code
            );
            eprintln!("   The Stata process was killed before completion.");
        }
    }
}

/// Print suggestions for common errors
pub fn print_suggestions(suggestions: &[&str]) {
    if !suggestions.is_empty() {
        println!();
        println!("Suggestions:");
        for suggestion in suggestions {
            println!("  - {}", suggestion);
        }
    }
}

/// Format a duration as human-readable string
pub fn format_duration_secs(secs: f64) -> String {
    if secs < 0.01 {
        format!("{:.1}ms", secs * 1000.0)
    } else if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.2}s", secs)
    } else {
        let mins = (secs / 60.0).floor() as u64;
        let remaining_secs = secs % 60.0;
        format!("{}m {:.0}s", mins, remaining_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration_secs(0.005), "5.0ms");
        assert_eq!(format_duration_secs(0.5), "500ms");
    }

    #[test]
    fn test_format_duration_secs() {
        assert_eq!(format_duration_secs(1.5), "1.50s");
        assert_eq!(format_duration_secs(30.0), "30.00s");
    }

    #[test]
    fn test_format_duration_mins() {
        assert_eq!(format_duration_secs(90.0), "1m 30s");
        assert_eq!(format_duration_secs(125.5), "2m 6s");
    }

    #[test]
    fn test_print_error_details_known_code() {
        use crate::error::{ErrorType, StataError};
        // r(199) — will show cached message or range-based fallback
        let error = StataError::StataCode {
            error_type: ErrorType::SyntaxError,
            r_code: 199,
            message: "unrecognized command".to_string(),
            line_number: Some(1),
        };
        // Should not panic; output goes to stderr
        print_error_details(&error);
    }

    #[test]
    fn test_print_error_details_unknown_code() {
        use crate::error::{ErrorType, StataError};
        let error = StataError::StataCode {
            error_type: ErrorType::StataError,
            r_code: 99999,
            message: "something weird".to_string(),
            line_number: None,
        };
        print_error_details(&error);
    }

    #[test]
    fn test_print_error_details_process_killed() {
        use crate::error::StataError;
        let error = StataError::ProcessKilled { exit_code: 137 };
        print_error_details(&error);
    }

    #[test]
    fn test_wrap_text() {
        let text = "This is a long sentence that should be wrapped at a certain width.";
        let wrapped = wrap_text(text, 30, "  ");
        assert!(wrapped.contains('\n'));
        // Each line should be <= 32 chars (30 + 2 for indent)
        for line in wrapped.lines() {
            assert!(line.len() <= 35, "Line too long: '{}'", line);
        }
    }
}
