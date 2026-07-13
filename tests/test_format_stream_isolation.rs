//! Regression tests for #84: machine-readable stdout carries only the payload.
//!
//! The console wrappers execute `--format stata` stdout with `do`, so script
//! output streamed to stdout during the run breaks it. Runs `stacy task`
//! against a fake Stata binary that writes a noisy log.

#![cfg(unix)]

use assert_cmd::{cargo_bin_cmd, Command};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A line Stata would reject if executed as a command (#84 failed on `(`).
const NOISE: &str = "( 42 observations deleted )";

fn stacy() -> Command {
    cargo_bin_cmd!("stacy")
}

/// Fake Stata binary: mimics `stata -b -q do wrapper.do` by writing
/// `<wrapper stem>.log` into its cwd (noisy output + completion trailer).
fn write_fake_stata(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("fake-stata");
    let body = format!(
        "#!/bin/sh\n\
         for arg in \"$@\"; do last=\"$arg\"; done\n\
         stem=$(basename \"$last\" .do)\n\
         printf '%s\\n' '{NOISE}' 'more script output' '' 'end of do-file' > \"$stem.log\"\n"
    );
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// Project with a script task and a composite task, as in #84.
fn setup_project(temp: &TempDir) {
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]
name = "test"

[scripts]
clean = "src/01_clean.do"
all = ["clean"]
"#,
    )
    .unwrap();
    fs::create_dir(temp.path().join("src")).unwrap();
    fs::write(temp.path().join("src/01_clean.do"), "display 1\n").unwrap();
}

#[test]
fn test_task_format_stata_stdout_is_only_payload() {
    let temp = TempDir::new().unwrap();
    setup_project(&temp);
    let fake = write_fake_stata(temp.path());

    let output = stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["task", "all", "--format", "stata"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "task failed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains(NOISE),
        "script output leaked into --format stata stdout:\n{}",
        stdout
    );
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        assert!(
            line.starts_with('*')
                || line.starts_with("scalar stacy_")
                || line.starts_with("global stacy_"),
            "line not executable as Stata assignment: '{}'",
            line
        );
    }
    assert!(stdout.contains("scalar stacy_success = 1"));
    assert!(stdout.contains("scalar stacy_script_count = 1"));
}

#[test]
fn test_task_format_json_stdout_is_parseable() {
    let temp = TempDir::new().unwrap();
    setup_project(&temp);
    let fake = write_fake_stata(temp.path());

    let output = stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["task", "all", "--format", "json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "task failed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("--format json stdout not parseable ({}):\n{}", e, stdout));
    assert_eq!(parsed["success"], serde_json::Value::Bool(true));
}

#[test]
fn test_task_format_human_still_streams_script_output() {
    let temp = TempDir::new().unwrap();
    setup_project(&temp);
    let fake = write_fake_stata(temp.path());

    let output = stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["task", "all"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "task failed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Piped human mode still live-streams the script's log to stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(NOISE),
        "human mode should stream script output; stdout:\n{}",
        stdout
    );
}
