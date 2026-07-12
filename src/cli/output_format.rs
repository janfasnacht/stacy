//! Output format handling for CLI commands
//!
//! Provides the `OutputFormat` enum and utilities for formatting command output
//! in human-readable, JSON, or Stata-native formats.

use crate::executor::verbosity::Verbosity;
use clap::ValueEnum;
use std::io::IsTerminal;

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

/// Resolve executor verbosity from CLI flags with TTY-awareness
///
/// Machine-readable formats force `Quiet`: stdout is the payload channel
/// (executed with `do` by the console wrappers), so nothing may be streamed
/// to it (#84). Commands that run Stata derive their verbosity here.
pub fn resolve_verbosity(quiet: bool, verbose: u8, format: OutputFormat) -> Verbosity {
    if quiet || format.is_machine_readable() {
        Verbosity::Quiet
    } else {
        match verbose {
            0 if std::io::stdout().is_terminal() => Verbosity::DefaultInteractive,
            0 => Verbosity::PipedDefault,
            1 => Verbosity::Verbose,
            _ => Verbosity::VeryVerbose,
        }
    }
}

/// Escape a string for use in Stata double-quoted strings
///
/// In Stata, double-quoted strings ("...") do not expand macros,
/// so only embedded double quotes need escaping (replaced with single quotes).
pub fn escape_stata_string(s: &str) -> String {
    s.replace('"', "'")
}

/// Format a boolean as a Stata scalar assignment
pub fn format_stata_scalar_bool(name: &str, value: bool) -> String {
    let stata_val = if value { 1 } else { 0 };
    format!("scalar stacy_{} = {}", name, stata_val)
}

/// Format an integer as a Stata scalar assignment
pub fn format_stata_scalar_int(name: &str, value: i64) -> String {
    format!("scalar stacy_{} = {}", name, value)
}

/// Format a usize as a Stata scalar assignment
pub fn format_stata_scalar_usize(name: &str, value: usize) -> String {
    format!("scalar stacy_{} = {}", name, value)
}

/// Format a float as a Stata scalar assignment
pub fn format_stata_scalar_float(name: &str, value: f64) -> String {
    // Use enough precision for accurate representation
    format!("scalar stacy_{} = {:.6}", name, value)
}

/// Format a string as a Stata global macro assignment
///
/// Uses `global` instead of `local` so that the value survives across program
/// scope boundaries (e.g., from `stacy_exec` back to the calling wrapper).
/// Uses regular double quotes (not compound quotes) since the `global` command
/// does not accept compound quote syntax.
pub fn format_stata_local(name: &str, value: &str) -> String {
    let escaped = escape_stata_string(value);
    format!("global stacy_{} \"{}\"", name, escaped)
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
    fn test_escape_stata_string_with_double_quotes() {
        // Double quotes are replaced with single quotes
        assert_eq!(escape_stata_string("say \"hello\""), "say 'hello'");
    }

    #[test]
    fn test_escape_stata_string_with_backtick() {
        // Backticks are not special inside double-quoted strings
        assert_eq!(escape_stata_string("hello `world`"), "hello `world`");
    }

    #[test]
    fn test_escape_stata_string_with_braces() {
        assert_eq!(escape_stata_string("{key: value}"), "{key: value}");
    }

    #[test]
    fn test_format_stata_scalar_bool() {
        assert_eq!(
            format_stata_scalar_bool("success", true),
            "scalar stacy_success = 1"
        );
        assert_eq!(
            format_stata_scalar_bool("success", false),
            "scalar stacy_success = 0"
        );
    }

    #[test]
    fn test_format_stata_scalar_int() {
        assert_eq!(
            format_stata_scalar_int("exit_code", 0),
            "scalar stacy_exit_code = 0"
        );
        assert_eq!(
            format_stata_scalar_int("exit_code", -1),
            "scalar stacy_exit_code = -1"
        );
    }

    #[test]
    fn test_format_stata_scalar_float() {
        assert_eq!(
            format_stata_scalar_float("duration", 1.5),
            "scalar stacy_duration = 1.500000"
        );
    }

    #[test]
    fn test_format_stata_local() {
        assert_eq!(
            format_stata_local("log_file", "/path/to/file.log"),
            "global stacy_log_file \"/path/to/file.log\""
        );
    }

    #[test]
    fn test_format_stata_local_with_spaces() {
        assert_eq!(
            format_stata_local("path", "/path/with spaces/file.do"),
            "global stacy_path \"/path/with spaces/file.do\""
        );
    }

    #[test]
    fn test_format_stata_local_opt() {
        assert_eq!(
            format_stata_local_opt("name", Some("value")),
            Some("global stacy_name \"value\"".to_string())
        );
        assert_eq!(format_stata_local_opt("name", None), None);
    }

    #[test]
    fn test_output_format_is_machine_readable() {
        assert!(!OutputFormat::Human.is_machine_readable());
        assert!(OutputFormat::Json.is_machine_readable());
        assert!(OutputFormat::Stata.is_machine_readable());
    }

    #[test]
    fn test_resolve_verbosity_machine_readable_is_quiet() {
        for verbose in 0..=2 {
            assert_eq!(
                resolve_verbosity(false, verbose, OutputFormat::Stata),
                Verbosity::Quiet
            );
            assert_eq!(
                resolve_verbosity(false, verbose, OutputFormat::Json),
                Verbosity::Quiet
            );
        }
    }

    #[test]
    fn test_resolve_verbosity_quiet_flag_wins() {
        assert_eq!(
            resolve_verbosity(true, 2, OutputFormat::Human),
            Verbosity::Quiet
        );
    }

    #[test]
    fn test_resolve_verbosity_human_verbose_levels() {
        assert_eq!(
            resolve_verbosity(false, 1, OutputFormat::Human),
            Verbosity::Verbose
        );
        assert_eq!(
            resolve_verbosity(false, 2, OutputFormat::Human),
            Verbosity::VeryVerbose
        );
    }
}
