//! Regression tests for #100: a `local:` source installed whatever the directory
//! contained, regardless of the package name that was asked for.

use assert_cmd::{cargo_bin_cmd, Command};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn stacy() -> Command {
    cargo_bin_cmd!("stacy")
}

#[test]
fn test_add_local_rejects_name_the_directory_does_not_hold() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\nname = \"t\"\n").unwrap();

    let lib = temp.path().join("lib/othername");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("othername.ado"), "program define othername\nend\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .args(["add", "badname", "--source", "local:./lib/othername"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("badname"))
        .stderr(predicate::str::contains("othername.ado"));

    // Nothing was written: no lockfile entry, no manifest entry.
    assert!(!temp.path().join("stacy.lock").exists());
    let config = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
    assert!(!config.contains("badname"));
}

#[test]
fn test_add_local_accepts_matching_name() {
    let temp = TempDir::new().unwrap();
    let cache_home = TempDir::new().unwrap();
    fs::write(temp.path().join("stacy.toml"), "[project]\nname = \"t\"\n").unwrap();

    let lib = temp.path().join("lib/myutils");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("myutils.ado"), "program define myutils\nend\n").unwrap();

    stacy()
        .current_dir(temp.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .args(["add", "myutils", "--source", "local:./lib/myutils"])
        .assert()
        .success();

    let config = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
    assert!(config.contains("myutils"));
    assert!(temp.path().join("stacy.lock").exists());
}
