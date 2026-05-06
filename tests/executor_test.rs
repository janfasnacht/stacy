//! Integration tests for StataExecutor
//!
//! Tests the full execution pipeline:
//! - Spawn Stata
//! - Run script
//! - Parse log
//! - Return correct exit code

use stacy::error::parser::parse_log_for_errors;
use stacy::executor::runner::{run_stata, RunOptions};
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
    assert!(!errors.is_empty(), "Syntax error script should have errors");

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
    assert!(!errors.is_empty(), "File error script should have errors");

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

/// When the configured "Stata" binary doesn't exist, the user must see a
/// real error — not the misleading "Log file incomplete" that shipped before
/// issue #21.
#[test]
fn test_missing_binary_does_not_report_log_incomplete() {
    use stacy::executor::StataExecutor;
    let temp = tempfile::TempDir::new().unwrap();
    let script = temp.path().join("anything.do");
    std::fs::write(&script, "display \"hi\"\n").unwrap();

    let exec = StataExecutor::with_binary("/nonexistent/stata-bogus");
    let err = match exec.run_in_dir(&script, None, temp.path()) {
        Ok(_) => panic!("missing binary should error"),
        Err(e) => e,
    };
    let msg = err.to_string();

    assert!(
        !msg.contains("Log file incomplete"),
        "missing binary must not surface as 'Log file incomplete', got: {msg}"
    );
}

/// When the binary launches but writes a startup error to stderr (license
/// seat, init failure) without producing a log, the captured stderr must
/// reach the user's error message.
#[cfg(unix)]
#[test]
fn test_no_log_produced_surfaces_stderr() {
    use stacy::executor::StataExecutor;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let temp = tempfile::TempDir::new().unwrap();

    // Stand-in "Stata" — writes a license error to stderr, never opens a log.
    // Open with mode 0o755 + fsync to avoid Linux's ETXTBSY race after
    // write+chmod+exec.
    let fake_stata = temp.path().join("fake-stata");
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o755)
        .open(&fake_stata)
        .unwrap();
    f.write_all(b"#!/bin/sh\necho 'license seat exhausted' >&2\nexit 1\n")
        .unwrap();
    f.sync_all().unwrap();
    drop(f);

    let script = temp.path().join("anything.do");
    std::fs::write(&script, "display \"hi\"\n").unwrap();

    let exec = StataExecutor::with_binary(fake_stata.to_str().unwrap());
    let err = match exec.run_in_dir(&script, None, temp.path()) {
        Ok(_) => panic!("startup failure should error"),
        Err(e) => e,
    };
    let msg = err.to_string();

    assert!(
        msg.contains("license seat exhausted"),
        "captured stderr should appear in error message, got: {msg}"
    );
    assert!(
        !msg.contains("Log file incomplete"),
        "should not fall back to 'Log file incomplete', got: {msg}"
    );
}
