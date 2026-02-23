//! Test that we don't get false positives from r() codes in output

use stacy::error::parser::parse_log_for_errors;
use stacy::executor::runner::{run_stata, RunOptions};
use std::path::PathBuf;

const STATA_BINARY: &str = "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp";

fn test_script(name: &str) -> PathBuf {
    PathBuf::from("tests/log-analysis").join(name)
}

#[test]
#[ignore] // Requires Stata installation - run locally with: cargo test -- --ignored
fn test_false_positive_script() {
    // Script displays "r(199);" but succeeds
    let script = test_script("12_false_positive_test.do");
    let options = RunOptions::new(STATA_BINARY);

    let result = run_stata(&script, options).expect("Failed to run Stata");

    // Stata returns 0
    assert_eq!(result.exit_code, 0);
    assert!(result.completed);

    // Parse log - should find NO errors despite r() codes in output
    let errors = parse_log_for_errors(&result.log_file).expect("Failed to parse log");
    assert_eq!(
        errors.len(),
        0,
        "Should not detect false positives from display output"
    );
}
