//! Edge case tests for stacy
//!
//! Tests handling of:
//! - Large log files (memory efficiency)
//! - Unicode in paths
//! - Spaces in filenames

use std::path::{Path, PathBuf};
use std::process::Command;

const STACY_BINARY: &str = env!("CARGO_BIN_EXE_stacy");

fn edge_case_script(name: &str) -> PathBuf {
    PathBuf::from("tests/edge_cases").join(name)
}

/// Run stacy with `--format=json` and return the log file path it reports.
///
/// Since #20, the log no longer lives at `script.with_extension("log")` —
/// each invocation gets a unique stem. JSON output is the contract for
/// discovering where Stata wrote the log.
fn run_and_get_log(script: &Path) -> PathBuf {
    let output = Command::new(STACY_BINARY)
        .arg("run")
        .arg("--format=json")
        .arg(script)
        .output()
        .expect("Failed to execute stacy");

    assert!(
        output.status.success(),
        "stacy run failed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("stacy run --format=json should emit valid JSON");
    let log = v["log_file"]
        .as_str()
        .expect("JSON output must include log_file");
    PathBuf::from(log)
}

#[test]
#[ignore] // Requires Stata - run locally with: cargo test --test edge_cases_test -- --ignored
fn test_spaces_in_filename() {
    let script = edge_case_script("my script.do");

    // Verify script exists
    assert!(script.exists(), "Test script should exist");

    // Run stacy and ask it where the log landed (#20: not script.with_extension("log") anymore).
    let log_file = run_and_get_log(&script);

    assert!(
        log_file.exists(),
        "Log file should be created: {}",
        log_file.display()
    );
}

#[test]
#[ignore] // Requires Stata - run locally with: cargo test --test edge_cases_test -- --ignored
fn test_unicode_in_filename() {
    let script = edge_case_script("café_analysis.do");

    // Verify script exists
    assert!(script.exists(), "Test script should exist");

    // Run stacy and ask it where the log landed (#20).
    let log_file = run_and_get_log(&script);

    assert!(
        log_file.exists(),
        "Log file should be created: {}",
        log_file.display()
    );
}

#[test]
#[ignore] // Requires Stata - run locally with: cargo test --test edge_cases_test -- --ignored --nocapture
fn test_large_log_file() {
    let script = edge_case_script("large_log_generator.do");

    // Verify script exists
    assert!(script.exists(), "Test script should exist");

    // Run stacy and ask it where the log landed (#20).
    let log_file = run_and_get_log(&script);

    assert!(
        log_file.exists(),
        "Log file should be created: {}",
        log_file.display()
    );

    // Verify log file is actually large (should be several MB)
    let metadata = std::fs::metadata(&log_file).expect("Failed to read log metadata");
    let size_mb = metadata.len() as f64 / 1_048_576.0;
    println!("Log file size: {:.2} MB", size_mb);

    // Should be at least 1 MB (50k lines with ~100 chars each ≈ 5MB)
    assert!(
        size_mb >= 1.0,
        "Log file should be at least 1 MB, got {:.2} MB",
        size_mb
    );

    // Parse the log to ensure we can handle it
    use stacy::error::parser::parse_log_file;
    let parse_result = parse_log_file(&log_file);
    assert!(
        parse_result.is_ok(),
        "Should be able to parse large log file"
    );

    let errors = parse_result.unwrap();
    assert_eq!(
        errors.len(),
        0,
        "Large log should have no errors (successful execution)"
    );
}

#[test]
fn test_large_log_memory_efficiency() {
    // Test that read_last_lines doesn't load entire file
    use stacy::executor::log_reader::read_last_lines;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a large temporary file
    let mut temp = NamedTempFile::new().expect("Failed to create temp file");

    // Write 100,000 lines
    for i in 0..100_000 {
        writeln!(temp, "Line {}: Some content here", i).expect("Failed to write");
    }
    temp.flush().expect("Failed to flush");

    // Read only last 20 lines - should be fast and not load entire file
    let start = std::time::Instant::now();
    let lines = read_last_lines(temp.path(), 20).expect("Failed to read last lines");
    let duration = start.elapsed();

    // Should only get 20 lines
    assert_eq!(lines.len(), 20, "Should read exactly 20 lines");

    // Should be fast (< 500ms even for 100k line file; CI runners can be slow)
    assert!(
        duration.as_millis() < 500,
        "Reading last 20 lines should be fast, took {} ms",
        duration.as_millis()
    );

    // Verify we got the last lines
    assert!(
        lines[19].contains("Line 99999"),
        "Last line should be Line 99999"
    );
}
