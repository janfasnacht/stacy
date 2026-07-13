//! Regression tests for #98: `[run] log_dir` is honored, and per-script logs
//! follow the documented contract — removed on success, kept on failure.
//!
//! Before this, `log_dir` was parsed and reported by `stacy env` but nothing
//! consumed it: logs were written next to the run (the project root) and `task`,
//! `test` and `bench` never cleaned them up.

#![cfg(unix)]

use assert_cmd::{cargo_bin_cmd, Command};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn stacy() -> Command {
    cargo_bin_cmd!("stacy")
}

/// Fake Stata: writes `<wrapper stem>.log` into its cwd, as `stata -b do` does.
/// `outcome` "pass" writes a clean log; "fail" appends an r(198) trailer.
fn write_fake_stata(dir: &Path, outcome: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(format!("fake-stata-{}", outcome));
    let body = if outcome == "fail" {
        "#!/bin/sh\n\
         for arg in \"$@\"; do last=\"$arg\"; done\n\
         stem=$(basename \"$last\" .do)\n\
         printf '%s\\n' '. display xx' 'invalid syntax' 'r(198);' '' 'end of do-file' 'r(198);' \
         > \"$stem.log\"\n"
            .to_string()
    } else {
        "#!/bin/sh\n\
         for arg in \"$@\"; do last=\"$arg\"; done\n\
         stem=$(basename \"$last\" .do)\n\
         printf '%s\\n' '. display 1' '1' '' 'end of do-file' > \"$stem.log\"\n"
            .to_string()
    };
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// Project with a script, a task and a test, optionally setting `[run] log_dir`.
fn setup_project(root: &Path, log_dir: Option<&str>) {
    let run_section = match log_dir {
        Some(dir) => format!("\n[run]\nlog_dir = \"{}\"\n", dir),
        None => String::new(),
    };
    fs::write(
        root.join("stacy.toml"),
        format!(
            "[project]\nname = \"t\"\n{}\n[scripts]\nclean = \"src/01_clean.do\"\n",
            run_section
        ),
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/01_clean.do"), "display 1\n").unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(root.join("tests/test_smoke.do"), "display 1\n").unwrap();
}

/// `.log` files sitting directly in `dir`.
fn logs_in(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("log"))
        .collect();
    found.sort();
    found
}

#[test]
fn test_successful_run_leaves_no_log_behind() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "pass");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "src/01_clean.do"])
        .assert()
        .success();

    assert!(
        logs_in(temp.path()).is_empty(),
        "a successful run must not leave a log in the project root"
    );
    assert!(
        !temp.path().join("logs").exists(),
        "a successful run has nothing to keep, so log_dir stays absent"
    );
}

#[test]
fn test_failed_run_keeps_log_in_log_dir() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "fail");

    let output = stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "src/01_clean.do"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "syntax error exit code");
    assert!(
        logs_in(temp.path()).is_empty(),
        "the kept log belongs in log_dir, not the project root"
    );

    let kept = logs_in(&temp.path().join("logs"));
    assert_eq!(kept.len(), 1, "failed run should keep exactly one log");

    // The path stacy printed must be the path the log actually lives at.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let printed = stderr
        .lines()
        .find_map(|l| l.trim().strip_prefix("Log: "))
        .expect("failure output should name the kept log");
    let printed = Path::new(printed.trim());
    assert!(
        printed.exists(),
        "printed log path does not exist: {}",
        printed.display()
    );
    // macOS resolves the temp dir through /private, so compare canonical paths.
    assert_eq!(
        printed.canonicalize().unwrap(),
        kept[0].canonicalize().unwrap()
    );
}

#[test]
fn test_custom_log_dir_is_used() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("output/logs"));
    let fake = write_fake_stata(temp.path(), "fail");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "src/01_clean.do"])
        .output()
        .unwrap();

    assert_eq!(logs_in(&temp.path().join("output/logs")).len(), 1);
}

#[test]
fn test_log_flag_wins_over_log_dir() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "fail");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "src/01_clean.do", "--log", "run.log"])
        .output()
        .unwrap();

    let dest = temp.path().join("run.log");
    assert!(dest.exists(), "--log destination must be written");
    assert!(
        !temp.path().join("logs").exists(),
        "--log must not also populate log_dir"
    );
}

#[test]
fn test_log_flag_written_on_success() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "pass");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "src/01_clean.do", "--log", "run.log"])
        .assert()
        .success();

    assert!(
        temp.path().join("run.log").exists(),
        "--log is a durable artifact, kept even when the run passes"
    );
    assert!(logs_in(temp.path()).len() == 1, "only the --log artifact");
}

#[test]
fn test_failed_inline_run_keeps_the_log_it_printed() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "fail");

    let output = stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "-c", "display xx"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let printed = stderr
        .lines()
        .find_map(|l| l.trim().strip_prefix("Log: "))
        .expect("failure output should name the kept log");
    assert!(
        Path::new(printed.trim()).exists(),
        "inline failure printed a log path that was then deleted: {}",
        printed
    );
    assert!(
        logs_in(temp.path()).is_empty(),
        "the inline log belongs in log_dir"
    );
}

#[test]
fn test_successful_machine_format_runs_leave_no_logs_behind() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "pass");

    // `--format stata` is what the in-Stata wrappers always pass, so one log per
    // successful `stacy_run` would accumulate forever.
    for _ in 0..3 {
        stacy()
            .current_dir(temp.path())
            .env("STATA_BINARY", &fake)
            .args(["run", "--format", "stata", "src/01_clean.do"])
            .assert()
            .success();
    }

    assert!(logs_in(temp.path()).is_empty());
    assert!(
        !temp.path().join("logs").exists(),
        "successful machine-format runs must not accumulate logs"
    );
}

#[test]
fn test_successful_json_run_reports_no_log() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "pass");

    let output = stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "--format", "json", "src/01_clean.do"])
        .output()
        .unwrap();

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        v["log_file"].as_str(),
        Some(""),
        "a removed log must not be reported as a path that does not exist"
    );
    assert!(logs_in(temp.path()).is_empty());
    assert!(!temp.path().join("logs").exists());
}

#[test]
fn test_failed_json_run_reports_the_kept_log() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "fail");

    let output = stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "--format", "json", "src/01_clean.do"])
        .output()
        .unwrap();

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let reported = Path::new(v["log_file"].as_str().expect("log_file field"));
    assert!(
        reported.exists(),
        "failure must report a log that exists: {}",
        reported.display()
    );
    assert_eq!(logs_in(&temp.path().join("logs")).len(), 1);
}

#[test]
fn test_successful_machine_format_task_leaves_no_log_behind() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "pass");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["task", "clean", "--format", "stata"])
        .assert()
        .success();

    assert!(logs_in(temp.path()).is_empty());
    assert!(!temp.path().join("logs").exists());
}

#[test]
fn test_successful_task_leaves_no_log_behind() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "pass");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["task", "clean"])
        .assert()
        .success();

    assert!(
        logs_in(temp.path()).is_empty(),
        "stacy task must not leave logs in the project root"
    );
    assert!(!temp.path().join("logs").exists());
}

#[test]
fn test_failed_task_keeps_log_in_log_dir() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "fail");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["task", "clean"])
        .output()
        .unwrap();

    assert!(logs_in(temp.path()).is_empty());
    assert_eq!(logs_in(&temp.path().join("logs")).len(), 1);
}

#[test]
fn test_successful_test_run_leaves_no_log_behind() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "pass");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .arg("test")
        .assert()
        .success();

    assert!(
        logs_in(temp.path()).is_empty(),
        "stacy test must not leave logs in the project root"
    );
    assert!(logs_in(&temp.path().join("tests")).is_empty());
}

#[test]
fn test_successful_bench_leaves_no_log_behind() {
    let temp = TempDir::new().unwrap();
    setup_project(temp.path(), Some("logs"));
    let fake = write_fake_stata(temp.path(), "pass");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["bench", "src/01_clean.do", "--runs", "2", "--no-warmup"])
        .assert()
        .success();

    assert!(
        logs_in(temp.path()).is_empty(),
        "stacy bench must not leave one log per run behind"
    );
}

#[test]
fn test_failure_outside_a_project_keeps_log_in_place() {
    let temp = TempDir::new().unwrap();
    // No stacy.toml — nothing configures log_dir, so the log stays where Stata
    // wrote it rather than vanishing.
    fs::write(temp.path().join("x.do"), "display xx\n").unwrap();
    let fake = write_fake_stata(temp.path(), "fail");

    stacy()
        .current_dir(temp.path())
        .env("STATA_BINARY", &fake)
        .args(["run", "x.do"])
        .output()
        .unwrap();

    assert_eq!(
        logs_in(temp.path()).len(),
        1,
        "a failed run outside a project keeps its log next to the run"
    );
}
