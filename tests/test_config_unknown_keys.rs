//! Regression tests for #100: unknown keys in stacy.toml were dropped silently.
//!
//! A dependency under `[dependencies]` instead of `[packages.dependencies]`
//! vanished, and `lock`/`install` then reported success on zero packages.

use assert_cmd::{cargo_bin_cmd, Command};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn stacy() -> Command {
    cargo_bin_cmd!("stacy")
}

#[test]
fn test_lock_rejects_misplaced_dependencies_key() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"t\"\n\n[dependencies]\nestout = \"ssc\"\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("lock")
        .assert()
        .failure()
        .stderr(predicate::str::contains("dependencies"))
        .stderr(predicate::str::contains("[packages.dependencies]"));

    assert!(
        !temp.path().join("stacy.lock").exists(),
        "a rejected config must not produce a lockfile"
    );
}

#[test]
fn test_run_rejects_unknown_key() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        "[run]\nlog_dirs = \"logs\"\n",
    )
    .unwrap();
    fs::write(temp.path().join("x.do"), "display 1\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .args(["run", "x.do"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("log_dirs"));
}

#[test]
fn test_valid_config_still_loads() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        r#"[project]
name = "t"
authors = ["A <a@example.com>"]

[run]
log_dir = "logs"
show_progress = false
progress_interval_seconds = 5
max_log_size_mb = 100

[paths]
ado = ["ado"]

[packages.dependencies]
estout = "ssc"

[scripts]
clean = "src/01_clean.do"
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("env")
        .assert()
        .success();
}
