//! Tests for the --format stata output mode
//!
//! Verifies that all commands produce valid Stata syntax that can be directly executed.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get the stacy binary
fn stacy() -> Command {
    Command::cargo_bin("stacy").unwrap()
}

// =============================================================================
// Doctor command tests
// =============================================================================

#[test]
fn test_doctor_format_stata_syntax() {
    stacy()
        .arg("doctor")
        .arg("--format")
        .arg("stata")
        .assert()
        .stdout(predicate::str::contains("scalar _stacy_ready"))
        .stdout(predicate::str::contains("scalar _stacy_passed"))
        .stdout(predicate::str::contains("scalar _stacy_warnings"))
        .stdout(predicate::str::contains("scalar _stacy_failed"))
        .stdout(predicate::str::contains("scalar _stacy_check_count"));
}

#[test]
fn test_doctor_format_stata_is_valid_stata() {
    // Verify output is valid Stata syntax by checking specific patterns
    let output = stacy()
        .arg("doctor")
        .arg("--format")
        .arg("stata")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Each scalar assignment should follow the pattern: scalar _stacy_NAME = VALUE
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('*') {
            continue; // Skip empty lines and comments
        }

        // Should be either scalar or local
        assert!(
            line.starts_with("scalar _stacy_") || line.starts_with("local _stacy_"),
            "Line should start with 'scalar _stacy_' or 'local _stacy_': {}",
            line
        );
    }
}

// =============================================================================
// Env command tests
// =============================================================================

#[test]
fn test_env_format_stata_syntax() {
    stacy()
        .arg("env")
        .arg("--format")
        .arg("stata")
        .assert()
        .stdout(predicate::str::contains("scalar _stacy_has_config"))
        .stdout(predicate::str::contains("scalar _stacy_show_progress"))
        .stdout(predicate::str::contains("local _stacy_cache_dir"));
}

#[test]
fn test_env_format_stata_locals_use_compound_quotes() {
    let output = stacy()
        .arg("env")
        .arg("--format")
        .arg("stata")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // String locals should use compound quotes: `"..."'
    for line in stdout.lines() {
        if line.contains("local _stacy_")
            && !line.trim().ends_with("local _stacy_project_root")
            && !line.trim().ends_with("local _stacy_stata_binary")
        {
            // Skip empty locals (for optional fields)
            if line.contains("`\"") {
                assert!(
                    line.contains("`\"") && line.contains("\"'"),
                    "Local should use compound quotes: {}",
                    line
                );
            }
        }
    }
}

// =============================================================================
// Init command tests
// =============================================================================

#[test]
fn test_init_format_stata_syntax() {
    let temp = TempDir::new().unwrap();

    stacy()
        .arg("init")
        .arg(temp.path())
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains("local _stacy_status"))
        .stdout(predicate::str::contains("local _stacy_path"))
        .stdout(predicate::str::contains("scalar _stacy_created_count"));
}

#[test]
fn test_init_format_stata_success_status() {
    let temp = TempDir::new().unwrap();

    stacy()
        .arg("init")
        .arg(temp.path())
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "local _stacy_status `\"success\"'",
        ));
}

// =============================================================================
// Deps command tests
// =============================================================================

#[test]
fn test_deps_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("main.do"), "display \"hello\"").unwrap();

    stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains("local _stacy_script"))
        .stdout(predicate::str::contains("scalar _stacy_unique_count"))
        .stdout(predicate::str::contains("scalar _stacy_has_circular"))
        .stdout(predicate::str::contains("scalar _stacy_has_missing"));
}

#[test]
fn test_deps_format_stata_booleans() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("main.do"), "display \"hello\"").unwrap();

    let output = stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .arg("--format")
        .arg("stata")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // No dependencies, no circular, no missing -> all should be 0
    assert!(stdout.contains("scalar _stacy_has_circular = 0"));
    assert!(stdout.contains("scalar _stacy_has_missing = 0"));
    assert!(stdout.contains("scalar _stacy_unique_count = 0"));
}

// =============================================================================
// Run command tests (with missing file - doesn't require Stata)
// =============================================================================

#[test]
fn test_run_format_stata_missing_file() {
    stacy()
        .arg("run")
        .arg("/nonexistent/file.do")
        .arg("--format")
        .arg("stata")
        .assert()
        .failure()
        .code(3) // File error
        .stdout(predicate::str::contains("scalar _stacy_success = 0"))
        .stdout(predicate::str::contains("scalar _stacy_exit_code = 3"));
}

// =============================================================================
// Edge case tests
// =============================================================================

#[test]
fn test_format_stata_paths_with_spaces() {
    let temp = TempDir::new().unwrap();
    let dir_with_spaces = temp.path().join("path with spaces");
    fs::create_dir(&dir_with_spaces).unwrap();

    stacy()
        .arg("init")
        .arg(&dir_with_spaces)
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        // Path with spaces should be properly quoted
        .stdout(predicate::str::contains("path with spaces"));
}

#[test]
fn test_format_stata_special_chars_in_path() {
    let temp = TempDir::new().unwrap();
    // Create a script in a directory with special chars
    let script = temp.path().join("test.do");
    fs::write(&script, "display \"hello\"").unwrap();

    let output = stacy()
        .arg("deps")
        .arg(&script)
        .arg("--format")
        .arg("stata")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should produce valid Stata output
    assert!(stdout.contains("scalar _stacy_"));
}

#[test]
fn test_format_json_still_works() {
    // Ensure backward compatibility
    stacy()
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .assert()
        .stdout(predicate::str::contains("\"ready\""))
        .stdout(predicate::str::contains("\"checks\""));
}

// =============================================================================
// Stata syntax validity tests
// =============================================================================

#[test]
fn test_stata_output_all_lines_valid() {
    // Test that every line of output is valid Stata syntax
    let output = stacy()
        .arg("doctor")
        .arg("--format")
        .arg("stata")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Valid patterns:
        // 1. Comments: * ...
        // 2. Scalars: scalar _stacy_NAME = VALUE
        // 3. Locals: local _stacy_NAME ...
        let is_valid = line.starts_with('*')
            || line.starts_with("scalar _stacy_")
            || line.starts_with("local _stacy_");

        assert!(is_valid, "Invalid Stata syntax: '{}'", line);
    }
}

#[test]
fn test_stata_scalars_have_numeric_values() {
    let output = stacy()
        .arg("doctor")
        .arg("--format")
        .arg("stata")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.starts_with("scalar _stacy_") {
            // Extract value after "="
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim();
                // Should be a number (int or float)
                assert!(
                    value.parse::<f64>().is_ok(),
                    "Scalar value should be numeric: '{}'",
                    line
                );
            }
        }
    }
}

#[test]
fn test_stata_booleans_are_0_or_1() {
    let output = stacy()
        .arg("doctor")
        .arg("--format")
        .arg("stata")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check boolean fields specifically
    if let Some(ready_line) = stdout.lines().find(|l| l.contains("_stacy_ready")) {
        let value = ready_line.split('=').last().unwrap().trim();
        assert!(
            value == "0" || value == "1",
            "Boolean should be 0 or 1: {}",
            ready_line
        );
    }
}

// =============================================================================
// List command tests
// =============================================================================

#[test]
fn test_list_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    // Create a minimal stacy.toml
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("list")
        .arg("--format")
        .arg("stata")
        .assert()
        .stdout(predicate::str::contains("local _stacy_status"))
        .stdout(predicate::str::contains("scalar _stacy_package_count"));
}

// =============================================================================
// Outdated command tests
// =============================================================================

#[test]
fn test_outdated_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    // Create a minimal stacy.toml and lockfile
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        "version = \"1\"\n\n[packages]\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("outdated")
        .arg("--format")
        .arg("stata")
        .assert()
        .stdout(predicate::str::contains("local _stacy_status"))
        .stdout(predicate::str::contains("scalar _stacy_outdated_count"))
        .stdout(predicate::str::contains("scalar _stacy_total_count"));
}

// =============================================================================
// Lock command tests
// =============================================================================

#[test]
fn test_lock_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    // Create a minimal stacy.toml
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains("local _stacy_status"))
        .stdout(predicate::str::contains("scalar _stacy_package_count"))
        .stdout(predicate::str::contains("scalar _stacy_in_sync"));
}

#[test]
fn test_lock_check_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    // Create a minimal stacy.toml and proper lockfile
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        "version = \"1\"\n\n[packages]\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .arg("--check")
        .arg("--format")
        .arg("stata")
        .assert()
        .stdout(predicate::str::contains("scalar _stacy_in_sync"));
}

// =============================================================================
// Cache command tests
// =============================================================================

#[test]
fn test_cache_info_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    // Create a minimal stacy.toml
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("info")
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains("scalar _stacy_entry_count"))
        .stdout(predicate::str::contains("scalar _stacy_size_bytes"))
        .stdout(predicate::str::contains("local _stacy_cache_path"))
        .stdout(predicate::str::contains("scalar _stacy_cache_exists"));
}

#[test]
fn test_cache_clean_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    // Create a minimal stacy.toml
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("clean")
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains("local _stacy_status"))
        .stdout(predicate::str::contains("scalar _stacy_entries_removed"))
        .stdout(predicate::str::contains("scalar _stacy_entries_remaining"));
}

// =============================================================================
// Task command tests
// =============================================================================

#[test]
fn test_task_list_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    // Create a stacy.toml with tasks
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]
name = "test"

[tasks.build]
scripts = ["main.do"]
"#,
    )
    .unwrap();
    fs::write(temp.path().join("main.do"), "display \"hello\"").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("task")
        .arg("--list")
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains("scalar _stacy_task_count"))
        .stdout(predicate::str::contains("local _stacy_task_names"));
}

// =============================================================================
// Test command tests
// =============================================================================

#[test]
fn test_test_list_format_stata_syntax() {
    let temp = TempDir::new().unwrap();
    // Create a minimal project with tests dir
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();
    fs::create_dir(temp.path().join("tests")).unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--list")
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains("scalar _stacy_test_count"));
}
