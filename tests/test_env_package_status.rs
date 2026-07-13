//! Regression tests for #99: `stacy env` must check the cache before calling a
//! locked package installed.
//!
//! `env` built each adopath entry from `global_cache::package_path`, which only
//! constructs a path. A cold cache therefore reported every locked package as
//! installed.

use assert_cmd::{cargo_bin_cmd, Command};
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn stacy() -> Command {
    cargo_bin_cmd!("stacy")
}

/// Platform-correct cache subdirectory under a temp dir.
///
/// Windows resolves the cache from `LOCALAPPDATA` and adds a `cache` segment;
/// the other platforms use `XDG_CACHE_HOME`. Writing to the wrong one leaves
/// the package where stacy will not look for it.
fn cache_packages_dir(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join("stacy").join("cache").join("packages")
    } else {
        root.join("stacy").join("packages")
    }
}

/// Project with one locked package (estout 1.0.0).
fn setup_project(root: &Path) {
    fs::write(
        root.join("stacy.toml"),
        "[project]\nname = \"t\"\n\n[packages.dependencies]\nestout = \"ssc\"\n",
    )
    .unwrap();
    fs::write(
        root.join("stacy.lock"),
        r#"version = "1"

[packages.estout]
version = "1.0.0"
checksum = "deadbeef"
group = "production"

[packages.estout.source]
type = "SSC"
name = "estout"
"#,
    )
    .unwrap();
}

/// Populate the global cache with the package files stacy expects.
fn install_into_cache(cache_home: &Path) {
    let pkg = cache_packages_dir(cache_home).join("estout").join("1.0.0");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("estout.ado"), "program define estout\nend\n").unwrap();
}

#[test]
fn test_env_cold_cache_reports_package_missing() {
    let temp = TempDir::new().unwrap();
    let cache_home = TempDir::new().unwrap();
    setup_project(temp.path());

    stacy()
        .current_dir(temp.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LOCALAPPDATA", cache_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("Packages: 0 installed, 1 missing"))
        .stdout(predicate::str::contains("not installed"));
}

#[test]
fn test_env_warm_cache_reports_package_installed() {
    let temp = TempDir::new().unwrap();
    let cache_home = TempDir::new().unwrap();
    setup_project(temp.path());
    install_into_cache(cache_home.path());

    stacy()
        .current_dir(temp.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LOCALAPPDATA", cache_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("Packages: 1 installed"))
        .stdout(predicate::str::contains("missing").not())
        .stdout(predicate::str::contains("not installed").not());
}

#[test]
fn test_env_json_reports_missing_package() {
    let temp = TempDir::new().unwrap();
    let cache_home = TempDir::new().unwrap();
    setup_project(temp.path());

    let output = stacy()
        .current_dir(temp.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LOCALAPPDATA", cache_home.path())
        .args(["env", "--format", "json"])
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["project"]["package_count"], 0);
    assert_eq!(json["project"]["missing_package_count"], 1);

    let entry = json["s_ado"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["source"] == "package")
        .expect("package entry in s_ado");
    assert_eq!(entry["label"], "estout");
    assert_eq!(entry["installed"], false);
}

#[test]
fn test_env_json_reports_installed_package() {
    let temp = TempDir::new().unwrap();
    let cache_home = TempDir::new().unwrap();
    setup_project(temp.path());
    install_into_cache(cache_home.path());

    let output = stacy()
        .current_dir(temp.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LOCALAPPDATA", cache_home.path())
        .args(["env", "--format", "json"])
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["project"]["package_count"], 1);
    assert_eq!(json["project"]["missing_package_count"], 0);

    let entry = json["s_ado"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["source"] == "package")
        .expect("package entry in s_ado");
    assert_eq!(entry["installed"], true);
}

#[test]
fn test_env_stata_format_reports_package_counts() {
    let temp = TempDir::new().unwrap();
    let cache_home = TempDir::new().unwrap();
    setup_project(temp.path());

    let output = stacy()
        .current_dir(temp.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LOCALAPPDATA", cache_home.path())
        .args(["env", "--format", "stata"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("scalar stacy_package_count = 0"),
        "missing package_count scalar:\n{}",
        stdout
    );
    assert!(
        stdout.contains("scalar stacy_missing_package_count = 1"),
        "missing missing_package_count scalar:\n{}",
        stdout
    );
}
