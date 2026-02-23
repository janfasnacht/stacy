//! Error code mappings
//!
//! Maps Stata r() codes to our internal ErrorType using cached error database
//! with range-based category fallback.

use super::categories::category_for_code;
use super::error_db::lookup_error;
use super::ErrorType;

/// Map Stata r() codes to our ErrorType
///
/// Tries the cached error database first, then falls back to range-based
/// category assignment. The exit code contract is maintained regardless
/// of whether cached data is available.
///
/// # Examples
///
/// ```
/// use stacy::error::codes::r_code_to_error_type;
/// use stacy::error::ErrorType;
///
/// assert_eq!(r_code_to_error_type(199), ErrorType::SyntaxError);
/// assert_eq!(r_code_to_error_type(601), ErrorType::FileError);
/// assert_eq!(r_code_to_error_type(950), ErrorType::MemoryError);
/// ```
pub fn r_code_to_error_type(r_code: u32) -> ErrorType {
    // Try cached database first, fall back to range-based category
    let category = match lookup_error(r_code) {
        Some(entry) => entry.category.as_str(),
        None => category_for_code(r_code),
    };
    match category {
        "Syntax/Command" => ErrorType::SyntaxError,
        "File I/O" => ErrorType::FileError,
        "Memory/Resources" => ErrorType::MemoryError,
        "System" => ErrorType::SystemError,
        "Statistical problems" => ErrorType::StatisticalError,
        _ => ErrorType::StataError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_error_codes() {
        // These work via range-based fallback (100-199 = Syntax/Command)
        assert_eq!(r_code_to_error_type(198), ErrorType::SyntaxError);
        assert_eq!(r_code_to_error_type(199), ErrorType::SyntaxError);
    }

    #[test]
    fn test_file_error_codes() {
        // 600-699 = File I/O
        assert_eq!(r_code_to_error_type(601), ErrorType::FileError);
        assert_eq!(r_code_to_error_type(603), ErrorType::FileError);
    }

    #[test]
    fn test_memory_error_codes() {
        // 900-999 = Memory/Resources
        assert_eq!(r_code_to_error_type(950), ErrorType::MemoryError);
    }

    #[test]
    fn test_unknown_code() {
        assert_eq!(r_code_to_error_type(99999), ErrorType::StataError);
    }

    #[test]
    fn test_system_error_codes() {
        // 800-899 = System
        assert_eq!(r_code_to_error_type(800), ErrorType::SystemError);
    }

    #[test]
    fn test_statistical_error_codes() {
        // 400-499 = Statistical problems
        assert_eq!(r_code_to_error_type(430), ErrorType::StatisticalError);
        assert_eq!(r_code_to_error_type(409), ErrorType::StatisticalError);
    }

    #[test]
    fn test_range_coverage() {
        // Verify range-based fallback covers all critical ranges
        assert_eq!(r_code_to_error_type(50), ErrorType::StataError); // General
        assert_eq!(r_code_to_error_type(150), ErrorType::SyntaxError); // Syntax/Command
        assert_eq!(r_code_to_error_type(450), ErrorType::StatisticalError); // Statistical
        assert_eq!(r_code_to_error_type(650), ErrorType::FileError); // File I/O
        assert_eq!(r_code_to_error_type(850), ErrorType::SystemError); // System
        assert_eq!(r_code_to_error_type(950), ErrorType::MemoryError); // Memory/Resources
    }
}
