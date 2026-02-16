//! Error to exit code mapping
//!
//! This module implements the stable exit code contract from ARCHITECTURE.md.
//! Uses cached error database with range-based category fallback.

use super::categories::category_for_code;
use super::error_db::lookup_error;
use super::StataError;

/// Map a StataError to an exit code
///
/// # Exit Code Contract (NEVER BREAK)
///
/// | Code | Meaning | Usage |
/// |------|---------|-------|
/// | 0 | Success | Script executed without errors |
/// | 1 | Stata error | r() code detected in log |
/// | 2 | Syntax error | Unrecognized command, invalid syntax |
/// | 3 | File error | File not found, permission denied |
/// | 4 | Memory error | Insufficient memory |
/// | 5 | Internal error | stacy itself failed |
/// | 6 | Statistical error | Convergence failure, model problems |
/// | 10 | Environment error | Stata not found, config invalid |
///
/// # Examples
///
/// ```
/// use stata_cli::error::{StataError, ErrorType};
/// use stata_cli::error::mapper::error_to_exit_code;
///
/// let err = StataError::new(ErrorType::SyntaxError, "unrecognized command".to_string(), 199);
/// assert_eq!(error_to_exit_code(&err), 2);
/// ```
pub fn error_to_exit_code(error: &StataError) -> i32 {
    match error {
        StataError::StataCode { r_code, .. } => {
            // Try cached database first, fall back to range-based category
            let category = match lookup_error(*r_code) {
                Some(entry) => entry.category.as_str(),
                None => category_for_code(*r_code),
            };
            map_category_to_exit_code(category)
        }
        StataError::ProcessKilled { exit_code } => {
            // Pass through signal-based exit codes (143, 130, 137, etc.)
            *exit_code
        }
    }
}

/// Map error category string to exit code
fn map_category_to_exit_code(category: &str) -> i32 {
    match category {
        "Syntax/Command" => 2,
        "File I/O" => 3,
        "Memory/Resources" => 4,
        "Statistical problems" => 6,
        "System" => 10,
        _ => 1, // Generic Stata error for all other categories
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorType;

    #[test]
    fn test_syntax_error_mapping() {
        let err = StataError::new(ErrorType::SyntaxError, "syntax error".to_string(), 199);
        assert_eq!(error_to_exit_code(&err), 2);
    }

    #[test]
    fn test_file_error_mapping() {
        let err = StataError::new(ErrorType::FileError, "file not found".to_string(), 601);
        assert_eq!(error_to_exit_code(&err), 3);
    }

    #[test]
    fn test_memory_error_mapping() {
        let err = StataError::new(ErrorType::MemoryError, "no memory".to_string(), 950);
        assert_eq!(error_to_exit_code(&err), 4);
    }

    #[test]
    fn test_generic_error_mapping() {
        // r(2000) falls in "Non-errors" range → exit code 1
        let err = StataError::new(ErrorType::StataError, "generic error".to_string(), 2000);
        assert_eq!(error_to_exit_code(&err), 1);
    }

    #[test]
    fn test_process_killed_mapping() {
        let err = StataError::ProcessKilled { exit_code: 143 };
        assert_eq!(error_to_exit_code(&err), 143);
    }

    #[test]
    fn test_unknown_r_code() {
        let err = StataError::new(ErrorType::StataError, "unknown".to_string(), 99999);
        assert_eq!(error_to_exit_code(&err), 1);
    }

    #[test]
    fn test_system_error_mapping() {
        let err = StataError::new(ErrorType::SystemError, "system error".to_string(), 800);
        assert_eq!(error_to_exit_code(&err), 10);
    }

    #[test]
    fn test_statistical_error_mapping() {
        // r(430) "convergence not achieved" → exit code 6
        let err = StataError::new(ErrorType::StatisticalError, "convergence".to_string(), 430);
        assert_eq!(error_to_exit_code(&err), 6);
    }

    #[test]
    fn test_all_category_exit_codes_in_range() {
        // Verify that all categories map to valid exit codes
        let categories = [
            "General",
            "Syntax/Command",
            "File I/O",
            "Memory/Resources",
            "System",
            "Non-errors",
            "Mata runtime",
            "Class system",
            "Python runtime",
            "System failure",
        ];

        for category in &categories {
            let exit_code = map_category_to_exit_code(category);
            assert!(
                (0..=10).contains(&exit_code),
                "Invalid exit code {} for category '{}'",
                exit_code,
                category
            );
        }
    }

    #[test]
    fn test_range_based_fallback_spot_checks() {
        // Critical codes should map correctly via range-based fallback alone
        // r(199) → Syntax/Command → exit 2
        let err = StataError::new(ErrorType::SyntaxError, "test".to_string(), 199);
        assert_eq!(error_to_exit_code(&err), 2);

        // r(601) → File I/O → exit 3
        let err = StataError::new(ErrorType::FileError, "test".to_string(), 601);
        assert_eq!(error_to_exit_code(&err), 3);

        // r(950) → Memory/Resources → exit 4
        let err = StataError::new(ErrorType::MemoryError, "test".to_string(), 950);
        assert_eq!(error_to_exit_code(&err), 4);
    }
}
