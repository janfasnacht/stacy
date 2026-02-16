//! Output format handling for CLI commands
//!
//! Provides the `OutputFormat` enum and utilities for formatting command output
//! in human-readable, JSON, or Stata-native formats.

use clap::ValueEnum;

/// Output format for CLI commands
///
/// - `Human`: Colored, human-readable output (default)
/// - `Json`: Machine-readable JSON output
/// - `Stata`: Stata-native commands that can be directly executed with `do`
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable colored output (default)
    #[default]
    Human,
    /// Machine-readable JSON output
    Json,
    /// Stata-native commands for direct execution
    Stata,
}

impl OutputFormat {
    /// Returns true if this format should suppress human-friendly messages
    pub fn is_machine_readable(&self) -> bool {
        matches!(self, OutputFormat::Json | OutputFormat::Stata)
    }
}

/// Escape a string for use in Stata compound quotes
///
/// Compound quotes in Stata are: `"..."'
/// The only character that needs escaping is backtick (`), which is replaced
/// with `=char(96)' to safely embed it.
///
/// Note: Compound quotes handle `{`, `}`, and `"` without escaping.
pub fn escape_stata_string(s: &str) -> String {
    // Replace backticks with Stata's char(96) expression
    // The format is: `=char(96)'
    s.replace('`', "`=char(96)'")
}

/// Format a boolean as a Stata scalar assignment
pub fn format_stata_scalar_bool(name: &str, value: bool) -> String {
    let stata_val = if value { 1 } else { 0 };
    format!("scalar _stacy_{} = {}", name, stata_val)
}

/// Format an integer as a Stata scalar assignment
pub fn format_stata_scalar_int(name: &str, value: i64) -> String {
    format!("scalar _stacy_{} = {}", name, value)
}

/// Format a usize as a Stata scalar assignment
pub fn format_stata_scalar_usize(name: &str, value: usize) -> String {
    format!("scalar _stacy_{} = {}", name, value)
}

/// Format a float as a Stata scalar assignment
pub fn format_stata_scalar_float(name: &str, value: f64) -> String {
    // Use enough precision for accurate representation
    format!("scalar _stacy_{} = {:.6}", name, value)
}

/// Format a string as a Stata local assignment using compound quotes
pub fn format_stata_local(name: &str, value: &str) -> String {
    let escaped = escape_stata_string(value);
    format!("local _stacy_{} `\"{}\"'", name, escaped)
}

/// Format an optional string as a Stata local assignment
/// Returns None if the value is None (should be skipped)
pub fn format_stata_local_opt(name: &str, value: Option<&str>) -> Option<String> {
    value.map(|v| format_stata_local(name, v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_stata_string_simple() {
        assert_eq!(escape_stata_string("hello world"), "hello world");
    }

    #[test]
    fn test_escape_stata_string_with_backtick() {
        assert_eq!(
            escape_stata_string("hello `world`"),
            "hello `=char(96)'world`=char(96)'"
        );
    }

    #[test]
    fn test_escape_stata_string_with_braces() {
        // Braces don't need escaping in compound quotes
        assert_eq!(escape_stata_string("{key: value}"), "{key: value}");
    }

    #[test]
    fn test_escape_stata_string_with_quotes() {
        // Regular quotes don't need escaping in compound quotes
        assert_eq!(escape_stata_string("say \"hello\""), "say \"hello\"");
    }

    #[test]
    fn test_format_stata_scalar_bool() {
        assert_eq!(
            format_stata_scalar_bool("success", true),
            "scalar _stacy_success = 1"
        );
        assert_eq!(
            format_stata_scalar_bool("success", false),
            "scalar _stacy_success = 0"
        );
    }

    #[test]
    fn test_format_stata_scalar_int() {
        assert_eq!(
            format_stata_scalar_int("exit_code", 0),
            "scalar _stacy_exit_code = 0"
        );
        assert_eq!(
            format_stata_scalar_int("exit_code", -1),
            "scalar _stacy_exit_code = -1"
        );
    }

    #[test]
    fn test_format_stata_scalar_float() {
        assert_eq!(
            format_stata_scalar_float("duration", 1.5),
            "scalar _stacy_duration = 1.500000"
        );
    }

    #[test]
    fn test_format_stata_local() {
        assert_eq!(
            format_stata_local("log_file", "/path/to/file.log"),
            "local _stacy_log_file `\"/path/to/file.log\"'"
        );
    }

    #[test]
    fn test_format_stata_local_with_spaces() {
        assert_eq!(
            format_stata_local("path", "/path/with spaces/file.do"),
            "local _stacy_path `\"/path/with spaces/file.do\"'"
        );
    }

    #[test]
    fn test_format_stata_local_opt() {
        assert_eq!(
            format_stata_local_opt("name", Some("value")),
            Some("local _stacy_name `\"value\"'".to_string())
        );
        assert_eq!(format_stata_local_opt("name", None), None);
    }

    #[test]
    fn test_output_format_is_machine_readable() {
        assert!(!OutputFormat::Human.is_machine_readable());
        assert!(OutputFormat::Json.is_machine_readable());
        assert!(OutputFormat::Stata.is_machine_readable());
    }
}
