//! Integration tests for Stata wrapper files
//!
//! These tests verify that the Stata wrappers (stacy_*.ado) work correctly
//! when invoked from within Stata. They require Stata to be installed.
//!
//! The tests set up a proper project fixture with stacy.toml, stacy.lock,
//! and the necessary ado files, then run test_wrappers.do and parse
//! the results.

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Copy a directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Set up a test project with all necessary fixtures
fn setup_test_project(temp: &TempDir) -> std::io::Result<()> {
    let fixtures = Path::new("tests/stata/fixtures");
    let stata_dir = Path::new("stata");

    // Copy fixture files (stacy.toml, stacy.lock, scripts/)
    copy_dir_all(fixtures, temp.path())?;

    // Create ado directory and copy all .ado and .sthlp files
    let ado_dir = temp.path().join("ado");
    fs::create_dir_all(&ado_dir)?;

    for entry in fs::read_dir(stata_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "ado" || ext == "sthlp" {
                let dest = ado_dir.join(entry.file_name());
                fs::copy(&path, &dest)?;
            }
        }
    }

    // Create logs directory
    fs::create_dir_all(temp.path().join("logs"))?;

    // Copy the test file
    fs::copy(
        "tests/stata/test_wrappers.do",
        temp.path().join("test_wrappers.do"),
    )?;

    Ok(())
}

/// Parse the test log and count PASS/FAIL results
/// Only counts actual output lines, not source code lines
/// Stata log format:
///   Source code lines start with ". " (dot space)
///   Actual output lines don't start with "."
fn parse_test_results(log_content: &str) -> (usize, usize) {
    let mut passed = 0;
    let mut failed = 0;

    for line in log_content.lines() {
        let trimmed = line.trim_start();
        // Skip source code lines (start with ".")
        if trimmed.starts_with('.') {
            continue;
        }
        // Count actual output
        if trimmed.contains("[PASS]") {
            passed += 1;
        } else if trimmed.contains("[FAIL]") {
            failed += 1;
        }
    }

    (passed, failed)
}

/// Get the stacy binary path
fn stacy_binary() -> String {
    // Use release binary if it exists, otherwise debug
    let release = "target/release/stacy";
    let debug = "target/debug/stacy";

    if Path::new(release).exists() {
        std::env::current_dir()
            .unwrap()
            .join(release)
            .to_string_lossy()
            .to_string()
    } else {
        std::env::current_dir()
            .unwrap()
            .join(debug)
            .to_string_lossy()
            .to_string()
    }
}

#[test]
#[ignore] // Requires Stata to be installed
fn test_stata_wrappers_in_project_context() {
    let temp = TempDir::new().unwrap();

    // Set up the test project
    setup_test_project(&temp).expect("Failed to set up test project");

    // Get absolute path to stacy binary
    let stacy_path = stacy_binary();

    // Run the test file using stacy run
    let output = Command::new(&stacy_path)
        .current_dir(temp.path())
        .env("STACY_BINARY", &stacy_path)
        .arg("run")
        .arg("test_wrappers.do")
        .output()
        .expect("Failed to execute stacy run");

    // Read the log file
    let log_path = temp.path().join("logs").join("test_wrappers.log");
    let log_content = if log_path.exists() {
        fs::read_to_string(&log_path).unwrap_or_default()
    } else {
        // Try current directory
        let alt_log = temp.path().join("test_wrappers.log");
        fs::read_to_string(&alt_log).unwrap_or_default()
    };

    // Parse results
    let (passed, failed) = parse_test_results(&log_content);

    // Print summary for debugging
    println!("Test results: {} passed, {} failed", passed, failed);
    println!("Exit code: {:?}", output.status.code());

    if !log_content.is_empty() {
        println!("\n--- Log excerpt ---");
        for line in log_content
            .lines()
            .filter(|l| l.contains("[PASS]") || l.contains("[FAIL]") || l.contains("TEST"))
        {
            println!("{}", line);
        }
    }

    // Assert all tests passed
    assert!(
        failed == 0,
        "Stata wrapper tests failed: {} passed, {} failed\nLog:\n{}",
        passed,
        failed,
        log_content
    );

    // Ensure we actually ran some tests
    assert!(passed > 0, "No tests were executed. Log:\n{}", log_content);
}

#[test]
fn test_fixtures_exist() {
    // Verify fixture files exist (quick sanity check, doesn't need Stata)
    assert!(
        Path::new("tests/stata/fixtures/stacy.toml").exists(),
        "Fixture stacy.toml missing"
    );
    assert!(
        Path::new("tests/stata/fixtures/stacy.lock").exists(),
        "Fixture stacy.lock missing"
    );
    assert!(
        Path::new("tests/stata/fixtures/scripts/hello.do").exists(),
        "Fixture hello.do missing"
    );
    assert!(
        Path::new("tests/stata/test_wrappers.do").exists(),
        "test_wrappers.do missing"
    );
}

#[test]
fn test_all_wrapper_files_exist() {
    // Verify all wrapper .ado files exist
    let expected_wrappers = [
        "stacy.ado",
        "stacy_add.ado",
        "stacy_bench.ado",
        "stacy_cache_clean.ado",
        "stacy_cache_info.ado",
        "stacy_deps.ado",
        "stacy_doctor.ado",
        "stacy_env.ado",
        "stacy_explain.ado",
        "stacy_init.ado",
        "stacy_install.ado",
        "stacy_list.ado",
        "stacy_lock.ado",
        "stacy_outdated.ado",
        "stacy_remove.ado",
        "stacy_run.ado",
        "stacy_task.ado",
        "stacy_test.ado",
        "stacy_update.ado",
    ];

    for wrapper in expected_wrappers {
        let path = format!("stata/{}", wrapper);
        assert!(Path::new(&path).exists(), "Missing wrapper: {}", wrapper);

        // Also check for corresponding .sthlp file
        let help_file = wrapper.replace(".ado", ".sthlp");
        let help_path = format!("stata/{}", help_file);
        assert!(
            Path::new(&help_path).exists(),
            "Missing help file: {}",
            help_file
        );
    }
}
