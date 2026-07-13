//! Regression tests for #84: machine-readable stdout carries only the payload.
//!
//! The console wrappers execute `--format stata` stdout with `do`, so anything
//! else on stdout — streamed script output or progress prints — breaks it.
//! #84 hit `stacy task`, but the invariant applies to every command with
//! `--format`: this matrix runs each offline-capable command against a fake
//! Stata binary that writes a noisy log and asserts stdout stays pure.
//!
//! Not covered (require a lockfile with installed packages, i.e. network):
//! `install`, `update`, `outdated`, `add`, `remove`.

#![cfg(unix)]

use assert_cmd::{cargo_bin_cmd, Command};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A line Stata would reject if executed as a command (#84 failed on `(`).
const NOISE: &str = "( 42 observations deleted )";

/// Commands exercised against the standard fixture project.
/// (label, args, runs_stata: executes Stata, so its log could stream)
const MATRIX: &[(&str, &[&str], bool)] = &[
    ("run", &["run", "src/01_clean.do"], true),
    ("task", &["task", "all"], true),
    ("test", &["test"], true),
    ("bench", &["bench", "src/01_clean.do", "--runs", "1"], true),
    ("deps", &["deps", "src/01_clean.do"], false),
    ("list", &["list"], false),
    ("lock", &["lock"], false),
    ("env", &["env"], false),
    ("explain", &["explain", "601"], false),
    ("cache-info", &["cache", "info"], false),
];

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

/// Project with a script task, a composite task (as in #84), and a test file.
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
    fs::create_dir(temp.path().join("tests")).unwrap();
    fs::write(temp.path().join("tests/test_smoke.do"), "display 1\n").unwrap();
}

/// Every non-empty stdout line must be executable as a Stata assignment
/// (comment, scalar, or global) — nothing else may reach stdout.
fn assert_stata_payload_only(label: &str, stdout: &str, ran_stata: bool) {
    assert!(
        !stdout.trim().is_empty(),
        "{}: --format stata produced no payload",
        label
    );
    if ran_stata {
        assert!(
            !stdout.contains(NOISE),
            "{}: script output leaked into --format stata stdout:\n{}",
            label,
            stdout
        );
    }
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        assert!(
            line.starts_with('*')
                || line.starts_with("scalar stacy_")
                || line.starts_with("global stacy_"),
            "{}: line not executable as Stata assignment: '{}'",
            label,
            line
        );
    }
}

fn run_in_fixture(args: &[&str], format: &str) -> std::process::Output {
    let temp = TempDir::new().unwrap();
    setup_project(&temp);
    let fake = write_fake_stata(temp.path());

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(args)
        .args(["--format", format])
        .output()
        .unwrap()
}

#[test]
fn test_format_stata_stdout_is_only_payload() {
    for (label, args, runs_stata) in MATRIX {
        let output = run_in_fixture(args, "stata");
        assert!(
            output.status.success(),
            "{}: command failed; stderr: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_stata_payload_only(label, &stdout, *runs_stata);
    }
}

#[test]
fn test_format_json_stdout_is_parseable() {
    for (label, args, _) in MATRIX {
        let output = run_in_fixture(args, "json");
        assert!(
            output.status.success(),
            "{}: command failed; stderr: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_else(|e| {
            panic!(
                "{}: --format json stdout not parseable ({}):\n{}",
                label, e, stdout
            )
        });
    }
}

/// `doctor` exits 1 when a diagnostic fails, which can legitimately happen on
/// a CI host — the payload must stay pure either way, so exit code is not
/// asserted.
#[test]
fn test_doctor_format_stata_stdout_is_only_payload() {
    let output = run_in_fixture(&["doctor"], "stata");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_stata_payload_only("doctor", &stdout, false);

    let output = run_in_fixture(&["doctor"], "json");
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_else(|e| {
        panic!(
            "doctor: --format json stdout not parseable ({}):\n{}",
            e, stdout
        )
    });
}

/// `init` needs a directory without an existing project.
#[test]
fn test_init_format_stata_stdout_is_only_payload() {
    let temp = TempDir::new().unwrap();
    let output = stacy()
        .current_dir(temp.path())
        .args(["init", "--format", "stata"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "init: command failed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_stata_payload_only("init", &stdout, false);
}

#[test]
fn test_task_format_stata_result_scalars_present() {
    let output = run_in_fixture(&["task", "all"], "stata");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("scalar stacy_success = 1"));
    assert!(stdout.contains("scalar stacy_script_count = 1"));
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
