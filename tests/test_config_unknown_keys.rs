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
fn test_unknown_key_in_a_package_table_is_rejected() {
    let temp = TempDir::new().unwrap();
    // A typo in `version` used to drop the pin and resolve the latest instead.
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"t\"\n\n[packages.dependencies]\nestout = { source = \"ssc\", verison = \"1.0.0\" }\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("env")
        .assert()
        .failure()
        .stderr(predicate::str::contains("verison"));
}

#[test]
fn test_unknown_key_in_a_task_table_is_rejected() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("stacy.toml"),
        "[project]\nname = \"t\"\n\n[scripts]\nbuild = { script = \"x.do\", parralel = [\"a\"] }\n",
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("env")
        .assert()
        .failure()
        .stderr(predicate::str::contains("parralel"));
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
reghdfe = { source = "github:sergiocorreia/reghdfe", version = "v6.12.3" }

[scripts]
clean = "src/01_clean.do"
all = ["clean"]
analyze = { script = "src/02_analyze.do", args = ["--fast"], description = "Estimates" }
outputs = { parallel = ["clean"] }
"#,
    )
    .unwrap();

    stacy()
        .current_dir(temp.path())
        .arg("env")
        .assert()
        .success();
}
