//! Integration tests for stacy CLI commands
//!
//! Tests the complete CLI workflow from init to run.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get the stacy binary
fn stacy() -> Command {
    Command::cargo_bin("stacy").unwrap()
}

#[test]
fn test_help() {
    stacy()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("stacy"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("init"));
}

#[test]
fn test_version() {
    stacy()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.0.2"));
}

#[test]
fn test_init_creates_files() {
    let temp = TempDir::new().unwrap();

    stacy()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized"));

    // Verify files created
    assert!(temp.path().join("stacy.toml").exists());
    assert!(temp.path().join(".gitignore").exists());

    // Verify directories NOT created (minimal init)
    assert!(!temp.path().join("ado").exists());
    assert!(!temp.path().join("logs").exists());
}

#[test]
fn test_init_json_output() {
    let temp = TempDir::new().unwrap();

    stacy()
        .arg("init")
        .arg(temp.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\""))
        .stdout(predicate::str::contains("\"success\""));
}

#[test]
fn test_init_existing_project_fails() {
    let temp = TempDir::new().unwrap();

    // First init succeeds
    stacy().arg("init").arg(temp.path()).assert().success();

    // Second init fails (without --force)
    stacy()
        .arg("init")
        .arg(temp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_init_force_overwrites() {
    let temp = TempDir::new().unwrap();

    // First init
    stacy().arg("init").arg(temp.path()).assert().success();

    // Modify stacy.toml
    fs::write(temp.path().join("stacy.toml"), "modified").unwrap();

    // Second init with --force succeeds
    stacy()
        .arg("init")
        .arg(temp.path())
        .arg("--force")
        .assert()
        .success();

    // Verify content restored
    let content = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
    assert!(content.contains("[project]"));
}

#[test]
fn test_env_shows_info() {
    stacy()
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("Stata"))
        .stdout(predicate::str::contains("Project"));
}

#[test]
fn test_env_json_output() {
    stacy()
        .arg("env")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"stata\""))
        .stdout(predicate::str::contains("\"project\""));
}

#[test]
fn test_doctor_runs_checks() {
    stacy()
        .arg("doctor")
        .assert()
        // May succeed or fail depending on Stata installation
        .stdout(predicate::str::contains("Diagnostics"))
        .stdout(predicate::str::contains("Stata Installation"));
}

#[test]
fn test_doctor_json_output() {
    stacy()
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .assert()
        .stdout(predicate::str::contains("\"checks\""))
        .stdout(predicate::str::contains("\"summary\""));
}

#[test]
fn test_deps_analyzes_script() {
    let temp = TempDir::new().unwrap();

    // Create test scripts
    fs::write(temp.path().join("main.do"), "do \"helper.do\"").unwrap();
    fs::write(temp.path().join("helper.do"), "display \"hello\"").unwrap();

    stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .assert()
        .success()
        .stdout(predicate::str::contains("main.do"))
        .stdout(predicate::str::contains("helper.do"))
        .stdout(predicate::str::contains("1 dependency"));
}

#[test]
fn test_deps_detects_circular() {
    let temp = TempDir::new().unwrap();

    // Create circular dependency
    fs::write(temp.path().join("a.do"), "do \"b.do\"").unwrap();
    fs::write(temp.path().join("b.do"), "do \"a.do\"").unwrap();

    stacy()
        .arg("deps")
        .arg(temp.path().join("a.do"))
        .assert()
        .success()
        .stdout(predicate::str::contains("circular"));
}

#[test]
fn test_deps_json_output() {
    let temp = TempDir::new().unwrap();

    fs::write(temp.path().join("main.do"), "display \"hello\"").unwrap();

    stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"dependencies\""))
        .stdout(predicate::str::contains("\"summary\""));
}

#[test]
fn test_deps_missing_file() {
    stacy()
        .arg("deps")
        .arg("/nonexistent/file.do")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_run_missing_script() {
    stacy()
        .arg("run")
        .arg("/nonexistent/script.do")
        .assert()
        .failure()
        .code(3) // File error exit code
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_install_from_lockfile_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

#[test]
fn test_install_from_lockfile_no_lockfile() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No stacy.lock"));
}

#[test]
fn test_full_workflow_init_to_deps() {
    let temp = TempDir::new().unwrap();

    // Step 1: Init project
    stacy().arg("init").arg(temp.path()).assert().success();

    // Step 2: Create a script
    fs::write(
        temp.path().join("analysis.do"),
        "sysuse auto\nsummarize price",
    )
    .unwrap();

    // Step 3: Analyze dependencies
    stacy()
        .arg("deps")
        .arg(temp.path().join("analysis.do"))
        .assert()
        .success()
        .stdout(predicate::str::contains("analysis.do"));

    // Step 4: Check environment
    // Note: Don't check exact temp path as Windows adds \\?\ prefix when canonicalized
    stacy()
        .current_dir(temp.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("stacy.toml (found)"));
}

// ============================================================================
// Install command tests
// ============================================================================

#[test]
fn test_install_help() {
    stacy()
        .arg("install")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Install all packages from lockfile",
        ))
        .stdout(predicate::str::contains("--no-verify"));
}

// ============================================================================
// Add command tests
// ============================================================================

#[test]
fn test_add_help() {
    stacy()
        .arg("add")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("github:user/repo"))
        .stdout(predicate::str::contains("--dev"));
}

#[test]
fn test_add_invalid_source() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("testpkg")
        .arg("--source")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown package source"));
}

#[test]
fn test_add_github_invalid_format() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    // Missing repo part
    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("testpkg")
        .arg("--source")
        .arg("github:useronly")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid GitHub source"));
}

#[test]
fn test_add_github_empty_ref() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    // Empty ref after @
    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("testpkg")
        .arg("--source")
        .arg("github:user/repo@")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Empty git ref"));
}

#[test]
fn test_add_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("testpkg")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

// ============================================================================
// Remove command tests
// ============================================================================

#[test]
fn test_remove_help() {
    stacy()
        .arg("remove")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Remove packages"));
}

#[test]
fn test_remove_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("remove")
        .arg("testpkg")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

// ============================================================================
// Update command tests
// ============================================================================

#[test]
fn test_update_help() {
    stacy()
        .arg("update")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("Update packages"));
}

#[test]
fn test_update_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("update")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

// ============================================================================
// Install from lockfile tests
// ============================================================================

#[test]
fn test_install_no_verify_flag() {
    // Test that --no-verify flag is accepted
    stacy()
        .arg("install")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-verify"));
}

#[test]
fn test_install_from_lockfile_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        "version = \"1\"\n\n[packages]\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("install")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\""))
        .stdout(predicate::str::contains("\"success\""));
}

// ============================================================================
// Doctor command tests
// ============================================================================

#[test]
fn test_doctor_shows_environment() {
    // Doctor should now always show environment info (no --verbose needed)
    stacy()
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("Environment"));
}

#[test]
fn test_doctor_no_verbose_flag() {
    // --verbose flag should no longer exist
    stacy()
        .arg("doctor")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--format"))
        // Should NOT contain --verbose
        .stdout(predicate::str::contains("--verbose").not());
}

// ============================================================================
// Run command tests
// ============================================================================

#[test]
fn test_run_no_dry_run_flag() {
    // --dry-run flag should no longer exist
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        // Should NOT contain --dry-run
        .stdout(predicate::str::contains("--dry-run").not());
}

#[test]
fn test_run_no_version_flag() {
    // -V/--version should not be on run subcommand
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        // Should NOT contain -V or --version
        .stdout(predicate::str::contains("-V").not())
        .stdout(predicate::str::contains("--version").not());
}

#[test]
fn test_run_verbosity_flags() {
    // Test that verbosity flags are documented
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("-q"))
        .stdout(predicate::str::contains("--quiet"))
        .stdout(predicate::str::contains("-v"))
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
#[ignore] // Requires Stata installation
fn test_run_quiet_flag() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display \"hello\"").unwrap();

    // With --quiet, should succeed with no output
    stacy()
        .arg("run")
        .arg(temp.path().join("test.do"))
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

// ============================================================================
// Inline code (-c/--code) tests
// ============================================================================

#[test]
fn test_run_inline_code_flag_exists() {
    // Test that -c/--code flag is documented
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("-c"))
        .stdout(predicate::str::contains("--code"))
        .stdout(predicate::str::contains("inline"));
}

#[test]
fn test_run_inline_code_mutual_exclusion() {
    let temp = TempDir::new().unwrap();
    let script = temp.path().join("test.do");
    fs::write(&script, "display 1").unwrap();

    // Providing both script and -c should fail
    stacy()
        .arg("run")
        .arg(&script)
        .arg("-c")
        .arg("display 2")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_run_inline_code_requires_value() {
    // -c without a value should fail
    stacy().arg("run").arg("-c").assert().failure();
}

#[test]
fn test_run_either_script_or_code_required() {
    // Neither script nor -c should fail
    stacy()
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_run_inline_code_empty_rejected() {
    // Empty code should be rejected
    stacy()
        .arg("run")
        .arg("-c")
        .arg("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn test_run_inline_code_whitespace_rejected() {
    // Whitespace-only code should be rejected
    stacy()
        .arg("run")
        .arg("-c")
        .arg("   ")
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

// ============================================================================
// Inline code tests that require Stata (marked #[ignore])
// Run with: cargo test test_run_inline -- --ignored
// ============================================================================

#[test]
#[ignore]
fn test_run_inline_code_success() {
    // Simple successful inline code
    stacy()
        .arg("run")
        .arg("-c")
        .arg("display 1+1")
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"))
        .stdout(predicate::str::contains("<inline code>"));
}

#[test]
#[ignore]
fn test_run_inline_code_error_detected() {
    // Inline code with error should return proper exit code
    stacy()
        .arg("run")
        .arg("-c")
        .arg("notacommand")
        .assert()
        .failure()
        .code(2) // Syntax error exit code
        .stderr(predicate::str::contains("r(199)"));
}

#[test]
#[ignore]
fn test_run_inline_code_json_output() {
    // JSON output should include source field
    stacy()
        .arg("run")
        .arg("-c")
        .arg("display 1")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"source\": \"inline\""))
        .stdout(predicate::str::contains("\"success\": true"));
}

#[test]
#[ignore]
fn test_run_inline_code_cleanup() {
    // Temp files should be cleaned up after execution
    let temp = TempDir::new().unwrap();

    // Run inline code in temp directory
    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("-c")
        .arg("display 1")
        .assert()
        .success();

    // Check no _stacy_inline_* files remain
    let remaining: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("_stacy_inline_")
        })
        .collect();

    assert!(
        remaining.is_empty(),
        "Temp files should be cleaned up: {:?}",
        remaining
    );
}

#[test]
#[ignore]
fn test_run_inline_code_multiline() {
    // Test multiline code with newlines (Stata uses newlines, not semicolons)
    stacy()
        .arg("run")
        .arg("-c")
        .arg("display 1\ndisplay 2\ndisplay 3")
        .assert()
        .success();
}

// ============================================================================
// Multiple -c flags tests
// ============================================================================

#[test]
fn test_run_multiple_c_flags_accepted() {
    // Test that multiple -c flags are accepted by the CLI parser
    stacy()
        .arg("run")
        .arg("-c")
        .arg("display 1")
        .arg("-c")
        .arg("display 2")
        .arg("--help")
        .assert()
        .success();
}

#[test]
#[ignore]
fn test_run_multiple_c_flags_execution() {
    // Test that multiple -c flags execute correctly (requires Stata)
    stacy()
        .arg("run")
        .arg("-c")
        .arg("display 1")
        .arg("-c")
        .arg("display 2")
        .arg("-c")
        .arg("display 3")
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"));
}

// ============================================================================
// Stdin support tests
// ============================================================================

#[test]
fn test_run_stdin_conflicts_with_code() {
    // Test that stdin marker (-) conflicts with -c flag
    // Note: clap catches this at the argument level since -c conflicts_with scripts
    stacy()
        .arg("run")
        .arg("-")
        .arg("-c")
        .arg("display 1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_run_stdin_empty_error() {
    // Test that empty stdin shows an error
    // Note: In test harness, stdin is not a TTY so it goes straight to "empty" check
    stacy()
        .arg("run")
        .arg("-")
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("stdin is empty"));
}

#[test]
fn test_run_stdin_whitespace_only_error() {
    // Test that whitespace-only stdin shows an error
    stacy()
        .arg("run")
        .arg("-")
        .write_stdin("   \n\t\n  ")
        .assert()
        .failure()
        .stderr(predicate::str::contains("stdin is empty"));
}

#[test]
#[ignore]
fn test_run_stdin_execution() {
    // Test that stdin execution works (requires Stata)
    stacy()
        .arg("run")
        .arg("-")
        .write_stdin("display 42")
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"))
        .stdout(predicate::str::contains("42"));
}

// ============================================================================
// Semicolon warning tests
// ============================================================================

#[test]
#[ignore]
fn test_run_semicolon_warning_shown() {
    // Test that semicolons trigger a warning (requires Stata)
    stacy()
        .arg("run")
        .arg("-c")
        .arg("display 1; display 2")
        .assert()
        // Will likely fail because semicolons aren't command separators in Stata
        .stderr(predicate::str::contains("warning").and(predicate::str::contains("semicolons")));
}

#[test]
#[ignore]
fn test_run_semicolon_warning_suppressed_with_delimit() {
    // Test that #delimit suppresses the semicolon warning (requires Stata)
    stacy()
        .arg("run")
        .arg("-c")
        .arg("#delimit ;")
        .arg("-c")
        .arg("display 1; display 2;")
        .assert()
        .success()
        // Should NOT contain the semicolon warning
        .stderr(predicate::str::contains("semicolons").not());
}

// ============================================================================
// stacy run output display tests (require Stata, #[ignore])
// ============================================================================

#[test]
#[ignore]
fn test_run_single_script_shows_output() {
    let temp = TempDir::new().unwrap();
    let script = temp.path().join("test.do");
    fs::write(&script, "display 42").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg(script.to_str().unwrap())
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"))
        .stdout(predicate::str::contains("42"));
}

#[test]
#[ignore]
fn test_run_single_script_failure_shows_log_path() {
    let temp = TempDir::new().unwrap();
    let script = temp.path().join("bad.do");
    fs::write(&script, "notacommand").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg(script.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("FAIL"))
        .stderr(predicate::str::contains("Log:"));
}

#[test]
#[ignore]
fn test_run_inline_shows_clean_output() {
    stacy()
        .arg("run")
        .arg("-c")
        .arg("display 42")
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"))
        .stdout(predicate::str::contains("42"));
}

// ============================================================================
// Integration tests that require network (marked #[ignore])
// ============================================================================

#[test]
#[ignore]
fn test_add_from_ssc_integration() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("estout")
        .assert()
        .success()
        .stdout(predicate::str::contains("estout"));

    // Check lockfile was created (packages go to global cache, not local ado/)
    assert!(temp.path().join("stacy.lock").exists());

    // Check stacy.toml contains the dependency
    let config = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
    assert!(config.contains("estout"));
}

#[test]
#[ignore]
fn test_add_from_github_integration() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("ftools")
        .arg("--source")
        .arg("github:sergiocorreia/ftools")
        .assert()
        .success()
        .stdout(predicate::str::contains("ftools"));

    // Check lockfile was created (packages go to global cache, not local ado/)
    assert!(temp.path().join("stacy.lock").exists());

    // Check lockfile contains GitHub source
    let lockfile = fs::read_to_string(temp.path().join("stacy.lock")).unwrap();
    assert!(lockfile.contains("sergiocorreia/ftools"));

    // Check stacy.toml contains the dependency
    let config = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
    assert!(config.contains("ftools"));
}

#[test]
#[ignore]
fn test_add_github_with_tag_integration() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("ftools")
        .arg("--source")
        .arg("github:sergiocorreia/ftools@master")
        .assert()
        .success();

    // Check lockfile contains the tag
    let lockfile = fs::read_to_string(temp.path().join("stacy.lock")).unwrap();
    assert!(lockfile.contains("master"));

    // Check stacy.toml contains the dependency
    let config = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
    assert!(config.contains("ftools"));
}

// ============================================================================
// List command tests
// ============================================================================

#[test]
fn test_list_help() {
    stacy()
        .arg("list")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("List installed packages"))
        .stdout(predicate::str::contains("--tree"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn test_list_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

#[test]
fn test_list_empty_lockfile() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No packages installed"));
}

#[test]
fn test_list_with_packages() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("estout"))
        .stdout(predicate::str::contains("2024.03.15"))
        .stdout(predicate::str::contains("ssc"))
        .stdout(predicate::str::contains("1 package"));
}

#[test]
fn test_list_tree_mode() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"

[packages.dev]
mdesc = "ssc"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"
group = "production"

[packages.estout.source]
type = "SSC"
name = "estout"

[packages.mdesc]
version = "2024.01.01"
checksum = "sha256:def456"
group = "dev"

[packages.mdesc.source]
type = "SSC"
name = "mdesc"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("list")
        .arg("--tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("production"))
        .stdout(predicate::str::contains("estout"))
        .stdout(predicate::str::contains("dev"))
        .stdout(predicate::str::contains("mdesc"));
}

#[test]
fn test_list_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("list")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\""))
        .stdout(predicate::str::contains("\"packages\""))
        .stdout(predicate::str::contains("\"estout\""));
}

// ============================================================================
// Outdated command tests
// ============================================================================

#[test]
fn test_outdated_help() {
    stacy()
        .arg("outdated")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Check for outdated packages"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn test_outdated_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("outdated")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

#[test]
fn test_outdated_no_lockfile() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("outdated")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No stacy.lock"));
}

#[test]
fn test_outdated_empty_lockfile() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        "version = \"1\"\n\n[packages]\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("outdated")
        .assert()
        .success()
        .stdout(predicate::str::contains("No packages installed"));
}

#[test]
fn test_outdated_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        "version = \"1\"\n\n[packages]\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("outdated")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\""))
        .stdout(predicate::str::contains("\"outdated_count\""));
}

// ============================================================================
// Lock command tests
// ============================================================================

#[test]
fn test_lock_help() {
    stacy()
        .arg("lock")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Generate or verify lockfile"))
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn test_lock_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

#[test]
fn test_lock_check_in_sync() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .arg("--check")
        .assert()
        .success()
        .stdout(predicate::str::contains("in sync"));
}

#[test]
fn test_lock_check_out_of_sync_missing() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"
newpkg = "ssc"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .arg("--check")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("out of sync"))
        .stdout(predicate::str::contains("newpkg"));
}

#[test]
fn test_lock_check_out_of_sync_extra() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.extrapkg]
version = "1.0.0"
checksum = "sha256:abc123"

[packages.extrapkg.source]
type = "SSC"
name = "extrapkg"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .arg("--check")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("out of sync"))
        .stdout(predicate::str::contains("extrapkg"));
}

#[test]
fn test_lock_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\""))
        .stdout(predicate::str::contains("\"in_sync\""));
}

// ============================================================================
// Add --test flag tests
// ============================================================================

#[test]
fn test_add_test_flag_exists() {
    stacy()
        .arg("add")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--test"))
        .stdout(predicate::str::contains("test dependency"));
}

#[test]
fn test_add_dev_test_mutually_exclusive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("testpkg")
        .arg("--dev")
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// ============================================================================
// Install --with flag tests
// ============================================================================

#[test]
fn test_install_with_flag_exists() {
    stacy()
        .arg("install")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--with"))
        .stdout(predicate::str::contains("dev"))
        .stdout(predicate::str::contains("test"));
}

#[test]
fn test_install_with_invalid_group() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        "version = \"1\"\n\n[packages]\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("install")
        .arg("--with")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown dependency group"));
}

#[test]
fn test_install_with_dev() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
prodpkg = "ssc"

[packages.dev]
devpkg = "ssc"
"#,
    )
    .unwrap();
    // Create lockfile with packages in different groups
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.prodpkg]
version = "1.0.0"
group = "production"

[packages.prodpkg.source]
type = "SSC"
name = "prodpkg"

[packages.devpkg]
version = "1.0.0"
group = "dev"

[packages.devpkg.source]
type = "SSC"
name = "devpkg"
"#,
    )
    .unwrap();

    // Should accept the flag and mention the groups
    stacy()
        .current_dir(temp.path())
        .arg("install")
        .arg("--with")
        .arg("dev")
        .assert()
        .success()
        .stdout(predicate::str::contains("production").or(predicate::str::contains("dev")));
}

#[test]
fn test_install_with_multiple_groups() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        "version = \"1\"\n\n[packages]\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("install")
        .arg("--with")
        .arg("dev,test")
        .assert()
        .success();
}

// ============================================================================
// Install --frozen tests
// ============================================================================

#[test]
fn test_install_frozen_flag_exists() {
    stacy()
        .arg("install")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--frozen"))
        .stdout(predicate::str::contains("CI"));
}

#[test]
fn test_install_frozen_in_sync() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"
group = "production"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("install")
        .arg("--frozen")
        .arg("--no-verify")
        .assert()
        .success();
}

#[test]
fn test_install_frozen_out_of_sync_missing() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"
newpkg = "ssc"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"
group = "production"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("install")
        .arg("--frozen")
        .assert()
        .failure()
        .stderr(predicate::str::contains("out of sync"))
        .stderr(predicate::str::contains("newpkg"))
        .stderr(predicate::str::contains("stacy lock"));
}

#[test]
fn test_install_frozen_out_of_sync_extra() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.extrapkg]
version = "1.0.0"
checksum = "sha256:abc123"
group = "production"

[packages.extrapkg.source]
type = "SSC"
name = "extrapkg"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("install")
        .arg("--frozen")
        .assert()
        .failure()
        .stderr(predicate::str::contains("out of sync"))
        .stderr(predicate::str::contains("extrapkg"));
}

// ============================================================================
// Task --frozen tests
// ============================================================================

#[test]
fn test_task_frozen_flag_exists() {
    stacy()
        .arg("task")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--frozen"))
        .stdout(predicate::str::contains("CI"));
}

#[test]
fn test_task_frozen_in_sync() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"

[scripts]
build = "main.do"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"
group = "production"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();
    fs::write(temp.path().join("main.do"), "display 1").unwrap();

    // --frozen check passes, but task will fail because Stata isn't installed
    // We just need to verify --frozen doesn't block it
    let result = stacy()
        .current_dir(temp.path())
        .arg("task")
        .arg("build")
        .arg("--frozen")
        .assert();

    // Either succeeds (Stata installed) or fails for Stata-related reasons (not lockfile sync)
    let output = result.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("out of sync"),
        "Should not fail due to lockfile sync"
    );
}

#[test]
fn test_task_frozen_out_of_sync() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"
newpkg = "ssc"

[scripts]
build = "main.do"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2024.03.15"
checksum = "sha256:abc123"
group = "production"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();
    fs::write(temp.path().join("main.do"), "display 1").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("task")
        .arg("build")
        .arg("--frozen")
        .assert()
        .failure()
        .stderr(predicate::str::contains("out of sync"))
        .stderr(predicate::str::contains("newpkg"));
}

#[test]
fn test_task_frozen_no_lockfile_with_packages() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"

[scripts]
build = "main.do"
"#,
    )
    .unwrap();
    fs::write(temp.path().join("main.do"), "display 1").unwrap();
    // No stacy.lock file

    stacy()
        .current_dir(temp.path())
        .arg("task")
        .arg("build")
        .arg("--frozen")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No stacy.lock"))
        .stderr(predicate::str::contains("stacy lock"));
}

#[test]
fn test_task_frozen_no_lockfile_no_packages() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[scripts]
build = "main.do"
"#,
    )
    .unwrap();
    fs::write(temp.path().join("main.do"), "display 1").unwrap();
    // No stacy.lock file, but also no packages - this is fine

    let result = stacy()
        .current_dir(temp.path())
        .arg("task")
        .arg("build")
        .arg("--frozen")
        .assert();

    // Should not fail due to lockfile sync (may fail for Stata reasons)
    let output = result.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("out of sync") && !stderr.contains("No stacy.lock"),
        "Should not fail due to lockfile: {}",
        stderr
    );
}

// ============================================================================
// Integration tests that require network (marked #[ignore])
// ============================================================================

#[test]
#[ignore]
fn test_outdated_checks_ssc_integration() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    // Use an old date that's definitely outdated
    fs::write(
        temp.path().join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "2020.01.01"
checksum = "sha256:abc123"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("outdated")
        .assert()
        .success()
        .stdout(predicate::str::contains("estout"))
        .stdout(predicate::str::contains("2020.01.01"))
        .stdout(predicate::str::contains("updates available"));
}

#[test]
#[ignore]
fn test_lock_resolves_ssc_integration() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]

[packages.dependencies]
estout = "ssc"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .assert()
        .success()
        .stdout(predicate::str::contains("estout"));

    // Check lockfile was created with version
    let lockfile = fs::read_to_string(temp.path().join("stacy.lock")).unwrap();
    assert!(lockfile.contains("estout"));
    assert!(lockfile.contains("version"));
}

#[test]
#[ignore]
fn test_add_test_dependency_integration() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("add")
        .arg("estout")
        .arg("--test")
        .assert()
        .success()
        .stdout(predicate::str::contains("test dependency"));

    // Check stacy.toml contains the test dependency
    let config = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
    assert!(config.contains("[packages.test]"));
    assert!(config.contains("estout"));
}

// ============================================================================
// Test command tests
// ============================================================================

#[test]
fn test_test_help() {
    stacy()
        .arg("test")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Run tests by convention"))
        .stdout(predicate::str::contains("--filter"))
        .stdout(predicate::str::contains("--parallel"))
        .stdout(predicate::str::contains("--list"));
}

#[test]
fn test_test_no_tests_found() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("No tests found"));
}

#[test]
fn test_test_list_empty() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No tests found"));
}

#[test]
fn test_test_list_discovers_tests() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(temp.path().join("tests/test_foo.do"), "display 1").unwrap();
    fs::write(temp.path().join("tests/test_bar.do"), "display 2").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test_foo"))
        .stdout(predicate::str::contains("test_bar"))
        .stdout(predicate::str::contains("2 tests"));
}

#[test]
fn test_test_discovers_by_naming_convention() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test_something.do"), "display 1").unwrap();
    fs::write(temp.path().join("other_test.do"), "display 2").unwrap();
    fs::write(temp.path().join("regular.do"), "display 3").unwrap(); // Should NOT be discovered

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test_something"))
        .stdout(predicate::str::contains("other_test"))
        .stdout(predicate::str::contains("regular").not())
        .stdout(predicate::str::contains("2 tests"));
}

#[test]
fn test_test_filter_by_pattern() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(temp.path().join("tests/test_foo.do"), "display 1").unwrap();
    fs::write(temp.path().join("tests/test_bar.do"), "display 2").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--list")
        .arg("--filter")
        .arg("foo")
        .assert()
        .success()
        .stdout(predicate::str::contains("test_foo"))
        .stdout(predicate::str::contains("test_bar").not())
        .stdout(predicate::str::contains("1 test"));
}

#[test]
fn test_test_list_json_output() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(temp.path().join("tests/test_foo.do"), "display 1").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--list")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"test_count\""))
        .stdout(predicate::str::contains("\"tests\""))
        .stdout(predicate::str::contains("test_foo"));
}

#[test]
fn test_test_not_found() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("nonexistent_test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_test_json_output_no_tests() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"test_count\": 0"))
        .stdout(predicate::str::contains("\"success\": true"));
}

// ============================================================================
// Test command tests that require Stata (marked #[ignore])
// ============================================================================

#[test]
#[ignore]
fn test_test_runs_passing_test() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("tests/test_success.do"),
        "display \"Running test\"\nassert 1 == 1\ndisplay \"Test passed!\"",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("passed"))
        .stdout(predicate::str::contains("test_success"));
}

#[test]
#[ignore]
fn test_test_runs_failing_test() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("tests/test_failure.do"),
        "display \"Running test\"\nassert 1 == 2\ndisplay \"This should not be reached\"",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("failed"))
        .stdout(predicate::str::contains("test_failure"));
}

#[test]
#[ignore]
fn test_test_json_output() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(temp.path().join("tests/test_success.do"), "display 1").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"test_count\""))
        .stdout(predicate::str::contains("\"passed\""))
        .stdout(predicate::str::contains("\"success\": true"));
}

#[test]
#[ignore]
fn test_test_parallel_execution() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(temp.path().join("tests/test_a.do"), "display \"a\"").unwrap();
    fs::write(temp.path().join("tests/test_b.do"), "display \"b\"").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("test")
        .arg("--parallel")
        .assert()
        .success()
        .stdout(predicate::str::contains("passed"));
}

// ============================================================================
// Run command working directory tests
// ============================================================================

#[test]
fn test_run_cd_flag_exists() {
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--cd"))
        .stdout(predicate::str::contains("script's parent directory"));
}

#[test]
fn test_run_directory_flag_exists() {
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("-C"))
        .stdout(predicate::str::contains("--directory"));
}

#[test]
fn test_run_cd_conflicts_with_code() {
    stacy()
        .arg("run")
        .arg("--cd")
        .arg("-c")
        .arg("display 1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_run_directory_conflicts_with_code() {
    stacy()
        .arg("run")
        .arg("-C")
        .arg(".")
        .arg("-c")
        .arg("display 1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_run_cd_conflicts_with_directory() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    stacy()
        .arg("run")
        .arg("--cd")
        .arg("-C")
        .arg(".")
        .arg(temp.path().join("test.do"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// ============================================================================
// Run command parallel execution tests
// ============================================================================

#[test]
fn test_run_parallel_flag_exists() {
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--parallel"))
        .stdout(predicate::str::contains("-P"))
        .stdout(predicate::str::contains("-j"))
        .stdout(predicate::str::contains("--jobs"));
}

#[test]
fn test_run_help_shows_parallel_examples() {
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--parallel"))
        .stdout(predicate::str::contains("-j4"));
}

#[test]
fn test_run_jobs_requires_parallel() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // -j without --parallel should fail
    stacy()
        .arg("run")
        .arg(temp.path().join("test.do"))
        .arg("-j")
        .arg("4")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--parallel"));
}

#[test]
fn test_run_multiple_missing_script() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("exists.do"), "display 1").unwrap();

    // Multiple scripts with one missing
    stacy()
        .arg("run")
        .arg(temp.path().join("exists.do"))
        .arg(temp.path().join("missing.do"))
        .assert()
        .failure()
        .code(3) // File error exit code
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_run_parallel_missing_script() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("exists.do"), "display 1").unwrap();

    // Parallel with missing script
    stacy()
        .arg("run")
        .arg("--parallel")
        .arg(temp.path().join("exists.do"))
        .arg(temp.path().join("missing.do"))
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("not found"));
}

// ============================================================================
// Run parallel tests that require Stata (marked #[ignore])
// ============================================================================

#[test]
#[ignore]
fn test_run_sequential_multiple_scripts() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("first.do"), "display \"first\"").unwrap();
    fs::write(temp.path().join("second.do"), "display \"second\"").unwrap();

    stacy()
        .arg("run")
        .arg(temp.path().join("first.do"))
        .arg(temp.path().join("second.do"))
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"))
        .stderr(predicate::str::contains("first.do"))
        .stderr(predicate::str::contains("second.do"))
        .stderr(predicate::str::contains("2 passed"));
}

#[test]
#[ignore]
fn test_run_sequential_fail_fast() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("first.do"), "notacommand").unwrap();
    fs::write(temp.path().join("second.do"), "display \"second\"").unwrap();

    // Sequential execution should stop at first failure
    stacy()
        .arg("run")
        .arg(temp.path().join("first.do"))
        .arg(temp.path().join("second.do"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("FAIL"))
        .stderr(predicate::str::contains("first.do"))
        // Second script should not run in fail-fast mode
        .stderr(predicate::str::contains("1 failed"));
}

#[test]
#[ignore]
fn test_run_parallel_multiple_scripts() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("first.do"), "display \"first\"").unwrap();
    fs::write(temp.path().join("second.do"), "display \"second\"").unwrap();

    stacy()
        .arg("run")
        .arg("--parallel")
        .arg(temp.path().join("first.do"))
        .arg(temp.path().join("second.do"))
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"))
        .stderr(predicate::str::contains("parallel"))
        .stderr(predicate::str::contains("2 passed"));
}

#[test]
#[ignore]
fn test_run_parallel_with_jobs() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("first.do"), "display 1").unwrap();
    fs::write(temp.path().join("second.do"), "display 2").unwrap();

    stacy()
        .arg("run")
        .arg("--parallel")
        .arg("-j")
        .arg("2")
        .arg(temp.path().join("first.do"))
        .arg(temp.path().join("second.do"))
        .assert()
        .success()
        .stdout(predicate::str::contains("2 jobs"));
}

#[test]
#[ignore]
fn test_run_parallel_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("first.do"), "display 1").unwrap();
    fs::write(temp.path().join("second.do"), "display 2").unwrap();

    stacy()
        .arg("run")
        .arg("--parallel")
        .arg("--format")
        .arg("json")
        .arg(temp.path().join("first.do"))
        .arg(temp.path().join("second.do"))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"parallel\": true"))
        .stdout(predicate::str::contains("\"passed\": 2"))
        .stdout(predicate::str::contains("\"scripts\""));
}

#[test]
#[ignore]
fn test_run_parallel_runs_all_on_failure() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("success.do"), "display 1").unwrap();
    fs::write(temp.path().join("fail.do"), "notacommand").unwrap();

    // Parallel should run all scripts even if some fail
    stacy()
        .arg("run")
        .arg("--parallel")
        .arg(temp.path().join("success.do"))
        .arg(temp.path().join("fail.do"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("1 passed"))
        .stderr(predicate::str::contains("1 failed"));
}

#[test]
#[ignore]
fn test_run_parallel_verbose_warning() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // --verbose with --parallel should warn
    stacy()
        .arg("run")
        .arg("--parallel")
        .arg("-v")
        .arg(temp.path().join("test.do"))
        .assert()
        .success()
        .stderr(predicate::str::contains("verbose").or(predicate::str::contains("ignored")));
}

// ============================================================================
// Cache command tests
// ============================================================================

#[test]
fn test_cache_help() {
    stacy()
        .arg("cache")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage the build cache"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("info"));
}

#[test]
fn test_cache_info_help() {
    stacy()
        .arg("cache")
        .arg("info")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Show build cache statistics"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn test_cache_clean_help() {
    stacy()
        .arg("cache")
        .arg("clean")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Remove cached build entries"))
        .stdout(predicate::str::contains("--older-than"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--quiet"));
}

#[test]
fn test_cache_info_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("info")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

#[test]
fn test_cache_clean_no_project() {
    let temp = TempDir::new().unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("clean")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a stacy project"));
}

#[test]
fn test_cache_info_empty_cache() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("info")
        .assert()
        .success()
        .stdout(predicate::str::contains("Build Cache Info"))
        .stdout(predicate::str::contains("Entries:"))
        .stdout(predicate::str::contains("0"));
}

#[test]
fn test_cache_info_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("info")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"entry_count\""))
        .stdout(predicate::str::contains("\"size_bytes\""))
        .stdout(predicate::str::contains("\"cache_exists\""));
}

#[test]
fn test_cache_clean_empty_cache() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("clean")
        .assert()
        .success()
        .stdout(predicate::str::contains("empty").or(predicate::str::contains("nothing")));
}

#[test]
fn test_cache_clean_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("clean")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"entries_removed\""))
        .stdout(predicate::str::contains("\"entries_remaining\""))
        .stdout(predicate::str::contains("\"status\""));
}

#[test]
fn test_cache_clean_quiet() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("cache")
        .arg("clean")
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ============================================================================
// Bench command tests
// ============================================================================

#[test]
fn test_bench_help() {
    stacy()
        .arg("bench")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Benchmark script execution"))
        .stdout(predicate::str::contains("-n"))
        .stdout(predicate::str::contains("--runs"))
        .stdout(predicate::str::contains("-w"))
        .stdout(predicate::str::contains("--warmup"))
        .stdout(predicate::str::contains("--no-warmup"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--engine"))
        .stdout(predicate::str::contains("--quiet"));
}

#[test]
fn test_bench_help_shows_examples() {
    stacy()
        .arg("bench")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Examples"))
        .stdout(predicate::str::contains("10 times"))
        .stdout(predicate::str::contains("warmup"));
}

#[test]
fn test_bench_missing_script_arg() {
    stacy()
        .arg("bench")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_bench_script_not_found() {
    stacy()
        .arg("bench")
        .arg("/nonexistent/script.do")
        .assert()
        .failure()
        .code(3) // File error exit code
        .stderr(predicate::str::contains("not found"));
}

// ============================================================================
// Run cache flags tests
// ============================================================================

#[test]
fn test_run_cache_flags_exist() {
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--cache"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--cache-only"));
}

#[test]
fn test_run_force_requires_cache() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // --force without --cache should fail
    stacy()
        .arg("run")
        .arg(temp.path().join("test.do"))
        .arg("--force")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--cache"));
}

#[test]
fn test_run_cache_only_requires_cache() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // --cache-only without --cache should fail
    stacy()
        .arg("run")
        .arg(temp.path().join("test.do"))
        .arg("--cache-only")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--cache"));
}

// ============================================================================
// Bench and cache tests that require Stata (marked #[ignore])
// ============================================================================

#[test]
#[ignore]
fn test_bench_runs_script() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    stacy()
        .arg("bench")
        .arg(temp.path().join("test.do"))
        .arg("-n")
        .arg("2")
        .arg("--no-warmup")
        .assert()
        .success()
        .stdout(predicate::str::contains("Benchmark Results"))
        .stdout(predicate::str::contains("mean"))
        .stdout(predicate::str::contains("median"));
}

#[test]
#[ignore]
fn test_bench_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    stacy()
        .arg("bench")
        .arg(temp.path().join("test.do"))
        .arg("-n")
        .arg("2")
        .arg("--no-warmup")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mean_secs\""))
        .stdout(predicate::str::contains("\"median_secs\""))
        .stdout(predicate::str::contains("\"stddev_secs\""))
        .stdout(predicate::str::contains("\"min_secs\""))
        .stdout(predicate::str::contains("\"max_secs\""))
        .stdout(predicate::str::contains("\"measured_runs\""));
}

#[test]
#[ignore]
fn test_bench_quiet_mode() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // Even in quiet mode, results should be shown
    stacy()
        .arg("bench")
        .arg(temp.path().join("test.do"))
        .arg("-n")
        .arg("2")
        .arg("--no-warmup")
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("Benchmark Results"));
}

#[test]
#[ignore]
fn test_run_with_cache() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // First run - should execute and cache
    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("--cache")
        .arg("test.do")
        .assert()
        .success();

    // Second run - should be a cache hit
    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("--cache")
        .arg("test.do")
        .assert()
        .success()
        .stdout(predicate::str::contains("cache").or(predicate::str::contains("cached")));
}

#[test]
#[ignore]
fn test_run_cache_force_rebuild() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // First run
    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("--cache")
        .arg("test.do")
        .assert()
        .success();

    // Second run with --force should rebuild
    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("--cache")
        .arg("--force")
        .arg("test.do")
        .assert()
        .success();
}

#[test]
#[ignore]
fn test_run_cache_only_miss() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // Cache-only mode with no cache should fail
    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("--cache")
        .arg("--cache-only")
        .arg("test.do")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not in cache"));
}

#[test]
#[ignore]
fn test_run_cache_json_output() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();
    fs::write(temp.path().join("test.do"), "display 1").unwrap();

    // First run
    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("--cache")
        .arg("test.do")
        .assert()
        .success();

    // Second run with JSON should show cache info
    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("--cache")
        .arg("--format")
        .arg("json")
        .arg("test.do")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"cached\"").or(predicate::str::contains("\"success\"")));
}

// =============================================================================
// Package Cache Tests
// =============================================================================

#[test]
fn test_cache_packages_path_help() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("path")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Show the global package cache path",
        ));
}

#[test]
fn test_cache_packages_list_help() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("list")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("List all cached packages"));
}

#[test]
fn test_cache_packages_clean_help() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("clean")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Clean the package cache"));
}

#[test]
fn test_cache_packages_path() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains("stacy").and(predicate::str::contains("packages")));
}

#[test]
fn test_cache_packages_path_json() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("path")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"path\""));
}

#[test]
fn test_cache_packages_list_empty() {
    // This test assumes no packages are cached, which might not always be true
    // but it at least tests the command runs without error
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("list")
        .assert()
        .success();
}

#[test]
fn test_cache_packages_list_json() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("list")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"packages\""));
}

#[test]
fn test_cache_packages_list_stata_format() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("list")
        .arg("--format")
        .arg("stata")
        .assert()
        .success()
        .stdout(predicate::str::contains("scalar stacy_package_count"));
}

#[test]
fn test_cache_packages_clean_requires_all_flag() {
    // Without --all flag, should just show info message
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("clean")
        .assert()
        .success()
        .stdout(predicate::str::contains("--all"));
}

#[test]
fn test_cache_packages_clean_json_without_all() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("clean")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"info\""));
}

#[test]
fn test_cache_help_shows_packages_subcommand() {
    stacy()
        .arg("cache")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("packages"))
        .stdout(predicate::str::contains("Manage the global package cache"));
}

#[test]
fn test_cache_packages_help() {
    stacy()
        .arg("cache")
        .arg("packages")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("path"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("clean"));
}

#[test]
fn test_env_shows_cache_path() {
    stacy()
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache:"))
        .stdout(predicate::str::contains("packages"));
}

#[test]
fn test_env_json_has_cache_path() {
    stacy()
        .arg("env")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"cache\""));
}

#[test]
#[ignore]
fn test_doctor_shows_package_cache_check() {
    stacy()
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Package Cache"));
}

#[test]
#[ignore]
fn test_doctor_json_has_cache_check() {
    stacy()
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("Package Cache"));
}

// ============================================================================
// Working directory tests that require Stata (marked #[ignore])
// ============================================================================

#[test]
#[ignore]
fn test_run_cd_flag_changes_directory() {
    // Create a nested directory structure with a script that uses relative paths
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("reports").join("pilot");
    fs::create_dir_all(&subdir).unwrap();

    // Create a script that writes to a file in the working directory
    fs::write(
        subdir.join("test.do"),
        "file open fh using \"output.txt\", write replace\nfile write fh \"success\"\nfile close fh",
    )
    .unwrap();

    // Run with --cd flag
    stacy()
        .arg("run")
        .arg("--cd")
        .arg(subdir.join("test.do"))
        .assert()
        .success();

    // Verify output file was created in the script's directory, not CWD
    assert!(
        subdir.join("output.txt").exists(),
        "output.txt should be created in script's directory"
    );
}

#[test]
#[ignore]
fn test_run_directory_flag_changes_directory() {
    // Create a directory structure
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("workdir");
    fs::create_dir_all(&subdir).unwrap();

    // Create a script in the temp root
    fs::write(
        temp.path().join("test.do"),
        "file open fh using \"output.txt\", write replace\nfile write fh \"success\"\nfile close fh",
    )
    .unwrap();

    // Run with -C flag pointing to subdir
    stacy()
        .arg("run")
        .arg("-C")
        .arg(&subdir)
        .arg(temp.path().join("test.do"))
        .assert()
        .success();

    // Verify output file was created in the specified directory
    assert!(
        subdir.join("output.txt").exists(),
        "output.txt should be created in -C directory"
    );
}

#[test]
fn test_run_directory_flag_validates_directory_exists() {
    // Passing a non-existent directory to -C should fail with a clear error
    let temp = TempDir::new().unwrap();
    let script = temp.path().join("test.do");
    fs::write(&script, "display 1").unwrap();

    stacy()
        .arg("run")
        .arg("-C")
        .arg("/nonexistent/directory/path")
        .arg(&script)
        .assert()
        .failure()
        .stderr(predicates::str::contains("Directory not found"));
}

#[test]
#[ignore]
fn test_run_cd_with_relative_adopath() {
    // Test that --cd helps scripts with relative adopath
    let temp = TempDir::new().unwrap();

    // Create directory structure: project/lib/ado/ and project/reports/
    let lib_dir = temp.path().join("lib").join("ado");
    let reports_dir = temp.path().join("reports");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::create_dir_all(&reports_dir).unwrap();

    // Create a simple ado file
    fs::write(
        lib_dir.join("myhelper.ado"),
        "program define myhelper\n  display \"helper loaded\"\nend",
    )
    .unwrap();

    // Create a script that uses relative adopath
    fs::write(
        reports_dir.join("analysis.do"),
        "adopath ++ \"../lib/ado\"\nmyhelper",
    )
    .unwrap();

    // Run with --cd so the relative path resolves correctly
    stacy()
        .arg("run")
        .arg("--cd")
        .arg(reports_dir.join("analysis.do"))
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"));
}

// ============================================================================
// Run --trace flag tests
// ============================================================================

#[test]
fn test_run_trace_flag_exists() {
    stacy()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--trace"))
        .stdout(predicate::str::contains("DEPTH"));
}

#[test]
fn test_run_trace_conflicts_with_quiet() {
    stacy()
        .arg("run")
        .arg("--trace")
        .arg("2")
        .arg("--quiet")
        .arg("-c")
        .arg("display 1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_run_trace_conflicts_with_parallel() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.do"), "display 1").unwrap();
    fs::write(temp.path().join("b.do"), "display 2").unwrap();

    stacy()
        .arg("run")
        .arg("--trace")
        .arg("2")
        .arg("--parallel")
        .arg(temp.path().join("a.do"))
        .arg(temp.path().join("b.do"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// Tests that require Stata installed (marked #[ignore])

#[test]
#[ignore]
fn test_run_trace_inline_success() {
    stacy()
        .arg("run")
        .arg("--trace")
        .arg("1")
        .arg("-c")
        .arg("display 42")
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"));
}

#[test]
#[ignore]
fn test_run_trace_inline_shows_trace_verbose() {
    // With --trace and -v, trace output is streamed to stdout via the log
    stacy()
        .arg("run")
        .arg("--trace")
        .arg("1")
        .arg("-v")
        .arg("-c")
        .arg("sysuse auto, clear")
        .assert()
        .success()
        .stdout(predicate::str::contains("set trace on"));
}

#[test]
#[ignore]
fn test_run_trace_file_success() {
    let temp = TempDir::new().unwrap();
    let script = temp.path().join("test.do");
    fs::write(&script, "display 42").unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("run")
        .arg("--trace")
        .arg("2")
        .arg(script.to_str().unwrap())
        .assert()
        .success()
        .stderr(predicate::str::contains("PASS"))
        .stderr(predicate::str::contains("test.do"));
}

#[test]
#[ignore]
fn test_run_trace_inline_error_shows_context() {
    // On error with trace, should show more context lines than the default 5
    let output = stacy()
        .arg("run")
        .arg("--trace")
        .arg("1")
        .arg("-c")
        .arg("display x")
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should have error details
    assert!(stderr.contains("FAIL"), "should show FAIL: {}", stderr);
}

#[test]
#[ignore]
fn test_run_trace_preserves_working_dir() {
    // Create a nested directory structure
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("reports");
    fs::create_dir_all(&subdir).unwrap();

    // Script writes a file to verify working dir
    fs::write(
        subdir.join("test.do"),
        "file open fh using \"trace_output.txt\", write replace\nfile write fh \"success\"\nfile close fh",
    )
    .unwrap();

    stacy()
        .arg("run")
        .arg("--trace")
        .arg("2")
        .arg("-C")
        .arg(&subdir)
        .arg(subdir.join("test.do"))
        .assert()
        .success();

    assert!(
        subdir.join("trace_output.txt").exists(),
        "output file should be in the -C directory"
    );
}
