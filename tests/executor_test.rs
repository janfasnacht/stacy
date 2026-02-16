//! Integration tests for StataExecutor
//!
//! Tests the full execution pipeline:
//! - Spawn Stata
//! - Run script
//! - Parse log
//! - Return correct exit code

use stata_cli::error::parser::parse_log_for_errors;
use stata_cli::executor::runner::{run_stata, RunOptions};
use std::path::PathBuf;
use std::time::Duration;

// Path to Stata binary
const STATA_BINARY: &str = "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp";

// Path to test scripts
fn test_script(name: &str) -> PathBuf {
    PathBuf::from("tests/log-analysis").join(name)
}

#[test]
#[ignore] // Requires Stata installation - run locally with: cargo test -- --ignored
fn test_success_script() {
    let script = test_script("01_success.do");
    let options = RunOptions::new(STATA_BINARY);

    let result = run_stata(&script, options).expect("Failed to run Stata");

    // Stata always returns 0
    assert_eq!(result.exit_code, 0);
    assert!(result.completed);

    // Parse log for errors
    let errors = parse_log_for_errors(&result.log_file).expect("Failed to parse log");
    assert_eq!(errors.len(), 0, "Success script should have no errors");
}

#[test]
#[ignore] // Requires Stata installation - run locally with: cargo test -- --ignored
fn test_syntax_error_script() {
    let script = test_script("02_syntax_error.do");
    let options = RunOptions::new(STATA_BINARY);

    let result = run_stata(&script, options).expect("Failed to run Stata");

    // Stata always returns 0 (the problem!)
    assert_eq!(result.exit_code, 0);
    assert!(result.completed);

    // But our parser should detect the error
    let errors = parse_log_for_errors(&result.log_file).expect("Failed to parse log");
    assert!(errors.len() > 0, "Syntax error script should have errors");

    // Should detect r(199)
    let has_199 = errors.iter().any(|e| e.r_code() == Some(199));
    assert!(has_199, "Should detect r(199) syntax error");
}

#[test]
#[ignore] // Requires Stata installation - run locally with: cargo test -- --ignored
fn test_file_not_found_script() {
    let script = test_script("03_file_not_found.do");
    let options = RunOptions::new(STATA_BINARY);

    let result = run_stata(&script, options).expect("Failed to run Stata");

    // Stata returns 0
    assert_eq!(result.exit_code, 0);

    // Parse should detect file error
    let errors = parse_log_for_errors(&result.log_file).expect("Failed to parse log");
    assert!(errors.len() > 0, "File error script should have errors");

    // Should detect r(601)
    let has_601 = errors.iter().any(|e| e.r_code() == Some(601));
    assert!(has_601, "Should detect r(601) file not found error");
}

#[test]
#[ignore] // Slow test - run manually with: cargo test --test executor_test test_timeout -- --ignored
fn test_timeout() {
    let script = test_script("07_infinite_loop.do");
    let options = RunOptions::new(STATA_BINARY).with_timeout(Duration::from_secs(2));

    let result = run_stata(&script, options).expect("Failed to run Stata");

    // Should be killed by timeout (SIGTERM = 143)
    assert_eq!(result.exit_code, 143);
    assert!(!result.completed, "Should not complete normally");
}

#[test]
fn test_project_isolation() {
    // Test that S_ADO is set correctly for project isolation
    // This test would require a project with ado/ directory
    // Deferred to integration tests
}
