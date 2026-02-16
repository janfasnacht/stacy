use stata_cli::executor::runner::{run_stata, RunOptions};
/// Integration tests for timeout handling
///
/// Tests that long-running scripts are properly terminated
use std::path::PathBuf;
use std::time::Duration;

// Stata binary path
const STATA_BINARY: &str = "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp";

// Path to test scripts
fn test_script(name: &str) -> PathBuf {
    PathBuf::from("tests/log-analysis").join(name)
}

#[test]
#[ignore] // Manual test - takes 2 seconds
fn test_infinite_loop_with_timeout() {
    // Test that an infinite loop is killed after timeout
    let script = test_script("07_infinite_loop.do");

    // Set 2-second timeout
    let options = RunOptions::new(STATA_BINARY).with_timeout(Duration::from_secs(2));

    let result = run_stata(&script, options).expect("Failed to run Stata");

    // Should be killed by timeout
    assert!(!result.completed, "Script should not complete normally");

    // SIGTERM = 143 (128 + 15)
    assert_eq!(result.exit_code, 143, "Should exit with SIGTERM code");

    // Should take approximately 2 seconds
    assert!(
        result.duration.as_secs() >= 2,
        "Should run for at least 2 seconds"
    );
    assert!(
        result.duration.as_secs() < 3,
        "Should be killed within 3 seconds"
    );
}

#[test]
#[ignore] // Manual test - takes 5+ seconds
fn test_long_running_script_completes() {
    // Test that a long-running script that DOES complete is handled correctly
    let script = test_script("06_long_running.do");

    // No timeout - should complete naturally
    let options = RunOptions::new(STATA_BINARY);

    let result = run_stata(&script, options).expect("Failed to run Stata");

    // Should complete successfully
    assert!(result.completed, "Script should complete normally");
    assert_eq!(result.exit_code, 0, "Should exit successfully");

    // Should take at least 5 seconds (script has sleep commands)
    assert!(
        result.duration.as_secs() >= 5,
        "Script should take at least 5 seconds"
    );
}

#[test]
#[ignore] // Manual test - takes 1 second
fn test_timeout_longer_than_script() {
    // Test that a timeout longer than script duration doesn't interfere
    let script = test_script("01_success.do");

    // 10-second timeout but script finishes in <1 second
    let options = RunOptions::new(STATA_BINARY).with_timeout(Duration::from_secs(10));

    let result = run_stata(&script, options).expect("Failed to run Stata");

    // Should complete normally
    assert!(result.completed, "Script should complete normally");
    assert_eq!(result.exit_code, 0, "Should exit successfully");
    assert!(result.duration.as_secs() < 2, "Should complete quickly");
}
