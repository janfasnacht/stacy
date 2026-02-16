//! Regression tests for architecture review findings (Issue #33)
//!
//! These CLI-level integration tests exercise the full pipeline end-to-end,
//! validating fixes for cache integrity (C1), error honesty (C4),
//! S_ADO isolation (NEW-1), deterministic ordering (M1), and --Break-- detection (C6).

use assert_cmd::Command;
use predicates::prelude::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Get the platform-correct cache subdirectory under a temp dir.
/// Unix: {root}/stacy/packages, Windows: {root}/stacy/cache/packages
fn cache_packages_dir(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join("stacy").join("cache").join("packages")
    } else {
        root.join("stacy").join("packages")
    }
}

/// Get the stacy binary for testing.
fn stacy() -> Command {
    Command::cargo_bin("stacy").unwrap()
}

// ============================================================================
// Test helpers
// ============================================================================

/// Compute SHA-256 of raw bytes (matches ssc::calculate_sha256).
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Compute combined checksum from individual file checksums
/// (matches ssc::calculate_combined_checksum — sorts before hashing).
fn combined_checksum(checksums: &[String]) -> String {
    let mut sorted = checksums.to_vec();
    sorted.sort();
    let mut hasher = Sha256::new();
    for cs in &sorted {
        hasher.update(cs.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Set up a temp project directory with stacy.toml and stacy.lock,
/// plus a pre-populated cache with known checksums.
///
/// Returns (project_dir, cache_dir, expected_checksum).
fn create_test_project_with_cache() -> (TempDir, TempDir, String) {
    let project = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();

    // Known file contents
    let content_ado = b"program define testpkg\nend";
    let content_sthlp = b"help for testpkg";

    // Compute expected checksum
    let cs_ado = sha256_hex(content_ado);
    let cs_sthlp = sha256_hex(content_sthlp);
    let expected = combined_checksum(&[cs_ado, cs_sthlp]);

    // Create cache structure using platform-correct path
    let pkg_cache = cache_packages_dir(cache.path())
        .join("testpkg")
        .join("1.0.0");
    fs::create_dir_all(&pkg_cache).unwrap();
    fs::write(pkg_cache.join("testpkg.ado"), content_ado).unwrap();
    fs::write(pkg_cache.join("testpkg.sthlp"), content_sthlp).unwrap();

    // Create stacy.toml
    let stacy_toml = r#"[project]
name = "test-project"

[packages.dependencies]
testpkg = "ssc"
"#;
    fs::write(project.path().join("stacy.toml"), stacy_toml).unwrap();

    // Create stacy.lock with correct checksum
    let stacy_lock = format!(
        r#"version = "1"
stacy_version = "{}"

[packages.testpkg]
version = "1.0.0"
checksum = "sha256:{}"
group = "production"

[packages.testpkg.source]
type = "SSC"
name = "testpkg"
"#,
        env!("CARGO_PKG_VERSION"),
        expected
    );
    fs::write(project.path().join("stacy.lock"), &stacy_lock).unwrap();

    (project, cache, expected)
}

/// Modify a cached file to simulate cache poisoning.
fn poison_cache(cache_dir: &Path, pkg_name: &str, version: &str) {
    let pkg_path = cache_packages_dir(cache_dir).join(pkg_name).join(version);
    let ado_file = pkg_path.join(format!("{}.ado", pkg_name));
    fs::write(ado_file, b"TAMPERED CONTENT").unwrap();
}

/// Write invalid TOML as stacy.lock.
fn create_corrupt_lockfile(project_dir: &Path) {
    fs::write(
        project_dir.join("stacy.lock"),
        "this is not valid TOML {{{[[[",
    )
    .unwrap();
}

// ============================================================================
// CLI integration tests
// ============================================================================

/// C1 regression: Poisoned cache must fail `stacy install --frozen`.
///
/// Validates that checksum verification catches tampered cache files
/// and returns a non-zero exit code with a meaningful error message.
#[test]
fn test_poisoned_cache_fails_install() {
    let (project, cache, _checksum) = create_test_project_with_cache();

    // Tamper with a cached file
    poison_cache(cache.path(), "testpkg", "1.0.0");

    stacy()
        .arg("install")
        .arg("--frozen")
        .current_dir(project.path())
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("checksum"));
}

/// Behavior preservation: --no-verify skips checksum on tampered cache.
///
/// Even with a poisoned cache, `--no-verify` should allow install to succeed
/// (the user explicitly opted out of verification).
#[test]
fn test_no_verify_skips_checksum_on_tampered_cache() {
    let (project, cache, _checksum) = create_test_project_with_cache();

    // Tamper with a cached file
    poison_cache(cache.path(), "testpkg", "1.0.0");

    stacy()
        .arg("install")
        .arg("--frozen")
        .arg("--no-verify")
        .current_dir(project.path())
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .assert()
        .success();
}

/// Happy path: Clean cache passes install verification.
///
/// With an untampered cache and correct checksum in lockfile,
/// `stacy install --frozen` should succeed.
#[test]
fn test_clean_cache_passes_install_verification() {
    let (project, cache, _checksum) = create_test_project_with_cache();

    stacy()
        .arg("install")
        .arg("--frozen")
        .current_dir(project.path())
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .assert()
        .success();
}

/// NEW-1 regression: S_ADO in strict mode excludes global paths.
///
/// When a project has a lockfile, `stacy env --format json` should report
/// an s_ado that contains only package paths and BASE — no SITE, PERSONAL,
/// PLUS, or OLDPLACE (which would break reproducibility).
#[test]
fn test_env_strict_mode_excludes_global_paths() {
    let (project, cache, _checksum) = create_test_project_with_cache();

    let output = stacy()
        .arg("env")
        .arg("--format")
        .arg("json")
        .current_dir(project.path())
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let s_ado = json["s_ado"].as_array().unwrap();
    let s_ado_text: Vec<&str> = s_ado.iter().map(|v| v.as_str().unwrap()).collect();

    // Should have BASE as last entry
    assert_eq!(
        s_ado_text.last().unwrap(),
        &"BASE",
        "Last S_ADO entry must be BASE"
    );

    // Should NOT contain global paths
    let joined = s_ado_text.join(";");
    assert!(
        !joined.contains("SITE"),
        "S_ADO must not contain SITE in strict mode"
    );
    assert!(
        !joined.contains("PERSONAL"),
        "S_ADO must not contain PERSONAL in strict mode"
    );
    assert!(
        !joined.contains("PLUS"),
        "S_ADO must not contain PLUS in strict mode"
    );
    assert!(
        !joined.contains("OLDPLACE"),
        "S_ADO must not contain OLDPLACE in strict mode"
    );
}

/// M1 regression: S_ADO lists package paths in alphabetical order.
///
/// With multiple packages in the lockfile, their paths in S_ADO must
/// appear alphabetically (deterministic ordering), with BASE last.
#[test]
fn test_env_shows_package_paths_alphabetically() {
    let project = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();

    // Create cache for three packages
    let packages = [("alpha", "1.0.0"), ("middle", "2.0.0"), ("zebra", "3.0.0")];
    for (name, version) in &packages {
        let pkg_dir = cache_packages_dir(cache.path()).join(name).join(version);
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join(format!("{}.ado", name)), "content").unwrap();
    }

    // Create stacy.toml
    let stacy_toml = r#"[project]
name = "test-project"

[packages.dependencies]
zebra = "ssc"
alpha = "ssc"
middle = "ssc"
"#;
    fs::write(project.path().join("stacy.toml"), stacy_toml).unwrap();

    // Create stacy.lock with packages in reverse alpha order (to test sorting)
    let stacy_lock = format!(
        r#"version = "1"
stacy_version = "{version}"

[packages.zebra]
version = "3.0.0"
group = "production"

[packages.zebra.source]
type = "SSC"
name = "zebra"

[packages.alpha]
version = "1.0.0"
group = "production"

[packages.alpha.source]
type = "SSC"
name = "alpha"

[packages.middle]
version = "2.0.0"
group = "production"

[packages.middle.source]
type = "SSC"
name = "middle"
"#,
        version = env!("CARGO_PKG_VERSION"),
    );
    fs::write(project.path().join("stacy.lock"), &stacy_lock).unwrap();

    let output = stacy()
        .arg("env")
        .arg("--format")
        .arg("json")
        .current_dir(project.path())
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let s_ado = json["s_ado"].as_array().unwrap();
    let s_ado_text: Vec<&str> = s_ado.iter().map(|v| v.as_str().unwrap()).collect();

    // Last entry should be BASE
    assert_eq!(s_ado_text.last().unwrap(), &"BASE");

    // Package entries (excluding BASE)
    let pkg_entries: Vec<&&str> = s_ado_text.iter().filter(|s| **s != "BASE").collect();
    assert_eq!(pkg_entries.len(), 3, "Should have 3 package paths");

    // They should appear in alpha, middle, zebra order
    let alpha_pos = s_ado_text.iter().position(|s| s.contains("alpha")).unwrap();
    let middle_pos = s_ado_text
        .iter()
        .position(|s| s.contains("middle"))
        .unwrap();
    let zebra_pos = s_ado_text.iter().position(|s| s.contains("zebra")).unwrap();

    assert!(
        alpha_pos < middle_pos && middle_pos < zebra_pos,
        "Packages must be in alphabetical order: alpha < middle < zebra, got positions {}, {}, {}",
        alpha_pos,
        middle_pos,
        zebra_pos
    );
}

/// C4 regression: Corrupt lockfile produces a warning on stderr.
///
/// When stacy.lock contains invalid TOML, `stacy env` should still succeed
/// (it's informational) but warn on stderr about the parse failure.
#[test]
fn test_env_warns_on_corrupt_lockfile() {
    let project = TempDir::new().unwrap();

    // Create stacy.toml so it's recognized as a project
    fs::write(
        project.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();

    // Write corrupt lockfile
    create_corrupt_lockfile(project.path());

    stacy()
        .arg("env")
        .current_dir(project.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("stacy.lock"));
}

/// Stata acceptance test: S_ADO isolation at runtime.
///
/// Requires Stata to be installed. Verifies that `stacy run` sets S_ADO
/// such that adopath contains only package cache paths and BASE.
#[test]
#[ignore]
fn test_s_ado_isolation_at_runtime() {
    let project = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();

    // Create minimal project with lockfile
    fs::write(
        project.path().join("stacy.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();

    let stacy_lock = format!(
        r#"version = "1"
stacy_version = "{}"

[packages]
"#,
        env!("CARGO_PKG_VERSION"),
    );
    fs::write(project.path().join("stacy.lock"), &stacy_lock).unwrap();

    // Copy the acceptance test fixture
    let fixture_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_s_ado_isolation.do");

    let fixture_dst = project.path().join("test_s_ado_isolation.do");
    fs::copy(&fixture_src, &fixture_dst).unwrap();

    stacy()
        .arg("run")
        .arg("test_s_ado_isolation.do")
        .current_dir(project.path())
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .assert()
        .success();
}
