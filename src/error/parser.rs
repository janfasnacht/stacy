use super::{codes::r_code_to_error_type, Error, Result, StataError};
use regex::Regex;
use std::fs;
use std::path::Path;

lazy_static::lazy_static! {
    /// Pattern for r() error code: r(123);
    /// Must be on its own line (possibly with whitespace)
    static ref R_CODE_PATTERN: Regex = Regex::new(r"^\s*r\((\d+)\);\s*$").unwrap();
}

/// Maximum number of non-empty lines to collect as error context
const MAX_MESSAGE_LINES: usize = 3;

/// Check if a line is a Stata command echo (`. command`, numbered `2. ...`, or `> ...` continuation)
///
/// Local helper to avoid coupling with executor::log_reader.
fn is_command_echo(trimmed: &str) -> bool {
    // Standard: `. ` prefix or bare `.`
    if trimmed.starts_with(". ") || trimmed == "." {
        return true;
    }

    // Continuation lines: `> ` prefix from long commands or #delimit ; mode
    if trimmed.starts_with("> ") {
        return true;
    }

    // Numbered continuation: `2. `, `10. `, etc.
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return false;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Must be followed by `. ` or `.` at end
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        return i >= bytes.len() || bytes[i] == b' ';
    }
    false
}

/// Extract the human-readable error message from the log body.
///
/// Scans lines before `body_end_idx` for the FIRST occurrence of `r(N);`
/// matching the given code, then collects up to 3 non-empty, non-echo lines
/// immediately preceding it.
fn extract_error_message(lines: &[&str], body_end_idx: usize, r_code: u32) -> Option<String> {
    let target = format!("r({});", r_code);

    // Find the FIRST r(N); in the body (not the post-marker one)
    let body_r_idx = lines[..body_end_idx]
        .iter()
        .position(|line| line.trim() == target)?;

    // Collect up to MAX_MESSAGE_LINES meaningful lines before the r(N);
    let mut context_lines: Vec<&str> = Vec::new();
    for &line in lines[..body_r_idx].iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // Stop at first blank line once we have some context
            if !context_lines.is_empty() {
                break;
            }
            continue;
        }
        if trimmed == "--Break--" {
            continue;
        }
        if is_command_echo(trimmed) {
            // Command echo found — stop collecting (the error text is above the echo)
            break;
        }
        context_lines.push(trimmed);
        if context_lines.len() >= MAX_MESSAGE_LINES {
            break;
        }
    }

    if context_lines.is_empty() {
        return None;
    }

    // Reverse to restore original order
    context_lines.reverse();
    Some(context_lines.join("\n"))
}

/// Parse a Stata log file for errors (main entry point)
pub fn parse_log_for_errors(log_path: &Path) -> Result<Vec<StataError>> {
    parse_log_file(log_path)
}

/// Parse a Stata log file for errors
pub fn parse_log_file(log_path: &Path) -> Result<Vec<StataError>> {
    let content = fs::read_to_string(log_path).map_err(Error::Io)?;

    parse_log_content(&content)
}

/// Parse log file content for errors
///
/// # Algorithm (from Phase 0, Batch 0.2)
///
/// 1. Find "end of do-file" marker (LAST occurrence for nested do-files)
/// 2. Check lines AFTER marker for r(\d+); pattern
/// 3. First match after marker = error code
/// 4. No match after marker = success
///
/// This approach is robust against false positives:
/// - Display output with "r(199)" appears BEFORE "end of do-file"
/// - Real error code appears AFTER "end of do-file"
/// - Stata only writes r() code after marker on actual errors
///
/// # Nested Do-Files
///
/// When a do-file calls another do-file (e.g., `do nested/helper.do`),
/// Stata writes multiple "end of do-file" markers to the log:
///
/// ```text
/// bad_var not found
/// r(111);
///
/// end of do-file    <- from nested/helper_with_error.do
/// r(111);
///
/// end of do-file    <- from main script
/// r(111);
/// ```
///
/// We use `rposition()` to find the LAST "end of do-file" marker,
/// which corresponds to the outermost script. This ensures we check
/// for errors in the correct location regardless of nesting depth.
///
/// # Example Error Log
///
/// ```text
/// command thisisnotacommand is unrecognized
/// r(199);
///
/// end of do-file
/// r(199);
/// ```
///
/// # Example Success Log (with false positive)
///
/// ```text
/// . display "r(199);"
/// r(199);
///
/// end of do-file
/// ```
///
pub fn parse_log_content(content: &str) -> Result<Vec<StataError>> {
    let lines: Vec<&str> = content.lines().collect();

    // 1. Find "end of do-file" marker
    let end_marker_idx = lines
        .iter()
        .rposition(|line| line.trim() == "end of do-file");

    if let Some(marker_idx) = end_marker_idx {
        // 2. Check lines AFTER marker for r() code
        let lines_after_marker = &lines[marker_idx + 1..];

        for (idx, line) in lines_after_marker.iter().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Skip Stata's --Break-- marker (appears before r() from `error` command)
            if trimmed == "--Break--" {
                continue;
            }

            // 3. Match r(\d+); pattern
            if let Some(captures) = R_CODE_PATTERN.captures(trimmed) {
                if let Ok(r_code) = captures[1].parse::<u32>() {
                    // Found error code after "end of do-file"
                    let error_type = r_code_to_error_type(r_code);
                    let line_number = marker_idx + 1 + idx + 1; // +1 for 1-indexed

                    // Extract actual error message from log body (before marker)
                    let message = extract_error_message(&lines, marker_idx, r_code)
                        .unwrap_or_else(|| super::error_db::lookup_error_message(r_code));

                    let error =
                        StataError::new(error_type, message, r_code).with_line_number(line_number);

                    return Ok(vec![error]);
                }
            }

            // If we hit a non-empty, non-r() line, stop searching
            // (shouldn't happen in practice, but be defensive)
            break;
        }

        // No r() code after marker = success
        Ok(vec![])
    } else {
        // No "end of do-file" marker = incomplete/crashed log
        // This happens when Stata is killed (SIGTERM, SIGKILL)
        Err(Error::Parse(
            "Log file incomplete: no 'end of do-file' marker found".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorType;

    #[test]
    fn test_syntax_error_detection() {
        let log = "command foo is unrecognized\nr(199);\n\nend of do-file\nr(199);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type(), ErrorType::SyntaxError);
        assert_eq!(errors[0].r_code(), Some(199));
    }

    #[test]
    fn test_r_code_extraction() {
        let log = "file data.dta not found\nr(601);\n\nend of do-file\nr(601);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].r_code(), Some(601));
        assert_eq!(errors[0].error_type(), ErrorType::FileError);
    }

    #[test]
    fn test_no_errors() {
        let log = "Some normal Stata output\nRegression results\n\nend of do-file";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_memory_error() {
        let log = "insufficient memory\nr(950);\n\nend of do-file\nr(950);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type(), ErrorType::MemoryError);
    }

    #[test]
    fn test_false_positive_in_output() {
        // User displays "r(199);" but script succeeds
        let log =
            ". display \"r(199);\"\nr(199);\n\n. display \"r(601);\"\nr(601);\n\nend of do-file";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(
            errors.len(),
            0,
            "Should not detect false positives before 'end of do-file'"
        );
    }

    #[test]
    fn test_false_positive_with_real_error() {
        // False positive before marker, real error after
        let log =
            ". display \"r(601);\"\nr(601);\n\nactual error\nr(199);\n\nend of do-file\nr(199);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].r_code(),
            Some(199),
            "Should only detect error after marker"
        );
    }

    #[test]
    fn test_incomplete_log() {
        // No "end of do-file" marker (killed process)
        let log = "Some output\nmore output";
        let result = parse_log_content(log);
        assert!(result.is_err(), "Should error on incomplete log");
        assert!(result.unwrap_err().to_string().contains("incomplete"));
    }

    #[test]
    fn test_break_marker_before_error_code() {
        // Stata's `error N` command outputs --Break-- before r(N);
        let log = "some output\n\nend of do-file\n--Break--\nr(1);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(
            errors.len(),
            1,
            "Should detect error after --Break-- marker"
        );
        assert_eq!(errors[0].r_code(), Some(1));
    }

    #[test]
    fn test_whitespace_handling() {
        // r() code with extra whitespace
        let log = "error message\nr(199);\n\nend of do-file\n  r(199);  \n";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].r_code(), Some(199));
    }

    // C6 regression: Multiple --Break-- markers before r(N); still detected
    #[test]
    fn test_multiple_break_markers_before_error() {
        let log = "some output\n\nend of do-file\n--Break--\n--Break--\n--Break--\nr(1);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(
            errors.len(),
            1,
            "Should detect error after multiple --Break-- markers"
        );
        assert_eq!(errors[0].r_code(), Some(1));
    }

    // =========================================================================
    // Error message extraction tests
    // =========================================================================

    #[test]
    fn test_error_message_extracted_from_body() {
        let log = ". badcmd\nunrecognized command:  badcmd\nr(199);\n\nend of do-file\nr(199);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].r_code(), Some(199));
        match &errors[0] {
            StataError::StataCode { message, .. } => {
                assert_eq!(message, "unrecognized command:  badcmd");
            }
            _ => panic!("Expected StataCode"),
        }
    }

    #[test]
    fn test_error_message_multiline() {
        let log = "\
. use nonexistent.dta
file nonexistent.dta not found
  (No data loaded.)
r(601);

end of do-file
r(601);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            StataError::StataCode { message, .. } => {
                assert!(
                    message.contains("file nonexistent.dta not found"),
                    "got: {}",
                    message
                );
                assert!(message.contains("(No data loaded.)"), "got: {}", message);
            }
            _ => panic!("Expected StataCode"),
        }
    }

    #[test]
    fn test_error_message_skips_command_echo() {
        // The `. badcmd` line should NOT be included in the message
        let log = ". badcmd\nunrecognized command:  badcmd\nr(199);\n\nend of do-file\nr(199);";
        let errors = parse_log_content(log).unwrap();
        match &errors[0] {
            StataError::StataCode { message, .. } => {
                assert!(
                    !message.starts_with(". "),
                    "Message should not include command echo, got: {}",
                    message
                );
                assert_eq!(message, "unrecognized command:  badcmd");
            }
            _ => panic!("Expected StataCode"),
        }
    }

    #[test]
    fn test_error_message_skips_break_marker() {
        // --Break-- in the body should be excluded from the message
        let log = "some error text\n--Break--\nr(1);\n\nend of do-file\n--Break--\nr(1);";
        let errors = parse_log_content(log).unwrap();
        match &errors[0] {
            StataError::StataCode { message, .. } => {
                assert!(
                    !message.contains("--Break--"),
                    "Message should not contain --Break--, got: {}",
                    message
                );
                assert_eq!(message, "some error text");
            }
            _ => panic!("Expected StataCode"),
        }
    }

    #[test]
    fn test_error_message_no_body_occurrence() {
        // r(199); appears only after marker, not in body — falls back to error-db/category
        let log = "some normal output\n\nend of do-file\nr(199);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            StataError::StataCode { message, .. } => {
                // Should be a fallback message (from error-db or category)
                assert!(
                    !message.contains("r(199);"),
                    "Should not contain raw r() code, got: {}",
                    message
                );
                assert!(!message.is_empty(), "Should have a fallback message");
            }
            _ => panic!("Expected StataCode"),
        }
    }

    #[test]
    fn test_error_message_nested_dofiles() {
        // Nested do-file: error appears with multiple "end of do-file" markers
        let log = "\
. do nested_helper.do
. badcmd
unrecognized command:  badcmd
r(199);

end of do-file
r(199);

end of do-file
r(199);";
        let errors = parse_log_content(log).unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            StataError::StataCode { message, .. } => {
                assert_eq!(message, "unrecognized command:  badcmd");
            }
            _ => panic!("Expected StataCode"),
        }
    }

    // =========================================================================
    // is_command_echo tests
    // =========================================================================

    #[test]
    fn test_is_command_echo_standard() {
        assert!(is_command_echo(". display 1"));
        assert!(is_command_echo("."));
        assert!(is_command_echo(". use auto.dta"));
    }

    #[test]
    fn test_is_command_echo_numbered() {
        assert!(is_command_echo("2. display 1"));
        assert!(is_command_echo("10. end"));
        assert!(is_command_echo("3."));
    }

    #[test]
    fn test_is_command_echo_not_echo() {
        assert!(!is_command_echo("unrecognized command"));
        assert!(!is_command_echo("r(199);"));
        assert!(!is_command_echo("file not found"));
        assert!(!is_command_echo("--Break--"));
        assert!(!is_command_echo(""));
    }

    // =========================================================================
    // extract_error_message tests
    // =========================================================================

    #[test]
    fn test_extract_message_simple() {
        let lines: Vec<&str> = vec![
            ". badcmd",
            "unrecognized command:  badcmd",
            "r(199);",
            "",
            "end of do-file",
        ];
        let msg = extract_error_message(&lines, 4, 199);
        assert_eq!(msg.unwrap(), "unrecognized command:  badcmd");
    }

    #[test]
    fn test_extract_message_caps_at_three_lines() {
        let lines: Vec<&str> = vec![
            "line one",
            "line two",
            "line three",
            "line four",
            "r(100);",
            "",
            "end of do-file",
        ];
        // Should only get the 3 lines closest to r(100);
        let msg = extract_error_message(&lines, 6, 100).unwrap();
        assert!(msg.contains("line two"));
        assert!(msg.contains("line three"));
        assert!(msg.contains("line four"));
        assert!(!msg.contains("line one"));
    }

    #[test]
    fn test_extract_message_none_when_no_body_match() {
        let lines: Vec<&str> = vec!["normal output", "", "end of do-file"];
        let msg = extract_error_message(&lines, 2, 199);
        assert!(msg.is_none());
    }
}
