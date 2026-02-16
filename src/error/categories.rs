//! Range-based error code categories
//!
//! Provides category assignment based on Stata's documented error code ranges.
//! Used as a fallback when no cached extraction data is available.

/// Assign a category to a Stata error code based on its numeric range.
///
/// These ranges are documented in Stata's Programming Manual and are not
/// copyrightable facts. They serve as the fallback when no extracted
/// error database is available.
pub fn category_for_code(code: u32) -> &'static str {
    match code {
        1..=99 => "General",
        100..=199 => "Syntax/Command",
        200..=299 => "Reserved",
        300..=399 => "Previously stored result",
        400..=499 => "Statistical problems",
        500..=599 => "Matrix manipulation",
        600..=699 => "File I/O",
        700..=799 => "Operating system",
        800..=899 => "System",
        900..=999 => "Memory/Resources",
        1000..=1999 => "System limits",
        2000..=2999 => "Non-errors",
        3000..=3999 => "Mata runtime",
        4000..=4999 => "Class system",
        7100..=7199 => "Python runtime",
        9000..=9999 => "System failure",
        _ => "General",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_range() {
        assert_eq!(category_for_code(1), "General");
        assert_eq!(category_for_code(9), "General");
        assert_eq!(category_for_code(99), "General");
    }

    #[test]
    fn test_syntax_command_range() {
        assert_eq!(category_for_code(100), "Syntax/Command");
        assert_eq!(category_for_code(199), "Syntax/Command");
    }

    #[test]
    fn test_file_io_range() {
        assert_eq!(category_for_code(601), "File I/O");
        assert_eq!(category_for_code(603), "File I/O");
        assert_eq!(category_for_code(699), "File I/O");
    }

    #[test]
    fn test_memory_range() {
        assert_eq!(category_for_code(900), "Memory/Resources");
        assert_eq!(category_for_code(950), "Memory/Resources");
    }

    #[test]
    fn test_system_range() {
        assert_eq!(category_for_code(800), "System");
        assert_eq!(category_for_code(899), "System");
    }

    #[test]
    fn test_mata_runtime() {
        assert_eq!(category_for_code(3000), "Mata runtime");
        assert_eq!(category_for_code(3999), "Mata runtime");
    }

    #[test]
    fn test_python_runtime() {
        assert_eq!(category_for_code(7100), "Python runtime");
        assert_eq!(category_for_code(7199), "Python runtime");
    }

    #[test]
    fn test_system_failure() {
        assert_eq!(category_for_code(9000), "System failure");
        assert_eq!(category_for_code(9999), "System failure");
    }

    #[test]
    fn test_boundary_between_ranges() {
        // 99 = General, 100 = Syntax/Command
        assert_eq!(category_for_code(99), "General");
        assert_eq!(category_for_code(100), "Syntax/Command");
        // 199 = Syntax/Command, 200 = Reserved
        assert_eq!(category_for_code(199), "Syntax/Command");
        assert_eq!(category_for_code(200), "Reserved");
    }

    #[test]
    fn test_gaps_fall_to_general() {
        // Codes in gaps between defined ranges
        assert_eq!(category_for_code(0), "General");
        assert_eq!(category_for_code(5000), "General");
        assert_eq!(category_for_code(6000), "General");
        assert_eq!(category_for_code(7000), "General"); // before Python range
        assert_eq!(category_for_code(7200), "General"); // after Python range
        assert_eq!(category_for_code(8000), "General"); // between Python and System failure
        assert_eq!(category_for_code(10000), "General");
    }
}
