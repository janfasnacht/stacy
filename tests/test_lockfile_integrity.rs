//! Regression tests for the lockfile integrity guarantee (#96).
//!
//! `stacy install` materializes `stacy.lock`; it never rewrites it. On a cold
//! cache it must fetch the pinned version and fail if the source no longer
//! serves it, rather than quietly locking whatever the source serves today.
//!
//! These tests use a `local:` package source so they run without network.

use assert_cmd::{cargo_bin_cmd, Command};
use predicates::prelude::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Contents of the vendored package the tests install from.
const PKG_ADO: &[u8] = b"program define testpkg\nend\n";

fn stacy() -> Command {
    cargo_bin_cmd!("stacy")
}

/// Platform-correct cache subdirectory under a temp dir.
fn cache_packages_dir(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join("stacy").join("cache").join("packages")
    } else {
        root.join("stacy").join("packages")
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Combined checksum: per-file SHA256s, sorted, then hashed
/// (matches `ssc::calculate_combined_checksum`).
fn combined_checksum(checksums: &[String]) -> String {
    let mut sorted = checksums.to_vec();
    sorted.sort();
    let mut hasher = Sha256::new();
    for cs in &sorted {
        hasher.update(cs.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// The checksum and version `stacy add` would record for the vendored package.
/// A local package's version is the first 8 chars of its combined checksum.
fn vendored_checksum_and_version() -> (String, String) {
    let checksum = combined_checksum(&[sha256_hex(PKG_ADO)]);
    let version = checksum[..8].to_string();
    (checksum, version)
}

/// Project with a vendored local package, plus an empty cache directory.
///
/// The lockfile pins `version` / `checksum` as given, so callers can pin the
/// truth or a lie. Returns (project, cache).
fn project_pinning(version: &str, checksum: &str) -> (TempDir, TempDir) {
    let project = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();

    // The package source on disk — no network needed to install it.
    let vendor = project.path().join("vendor").join("testpkg");
    fs::create_dir_all(&vendor).unwrap();
    fs::write(vendor.join("testpkg.ado"), PKG_ADO).unwrap();

    fs::write(
        project.path().join("stacy.toml"),
        "[project]\nname = \"test-project\"\n\n\
         [packages.dependencies]\ntestpkg = \"local:vendor/testpkg\"\n",
    )
    .unwrap();

    fs::write(
        project.path().join("stacy.lock"),
        format!(
            r#"version = "1"
stacy_version = "{}"

[packages.testpkg]
version = "{}"
checksum = "sha256:{}"
group = "production"

[packages.testpkg.source]
type = "Local"
path = "vendor/testpkg"
"#,
            env!("CARGO_PKG_VERSION"),
            version,
            checksum,
        ),
    )
    .unwrap();

    (project, cache)
}

fn install(project: &TempDir, cache: &TempDir, extra: &[&str]) -> assert_cmd::assert::Assert {
    stacy()
        .arg("install")
        .args(extra)
        .current_dir(project.path())
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .assert()
}

fn read_lock(project: &TempDir) -> Vec<u8> {
    fs::read(project.path().join("stacy.lock")).unwrap()
}

/// #96: a cold `--frozen` install of a version the source cannot serve must
/// fail — and must leave stacy.lock byte-for-byte untouched.
#[test]
fn test_frozen_install_fails_on_pinned_version_mismatch() {
    let (checksum, _real_version) = vendored_checksum_and_version();
    let (project, cache) = project_pinning("99999999", &checksum);
    let before = read_lock(&project);

    install(&project, &cache, &["--frozen"])
        .failure()
        .stderr(predicate::str::contains("99999999"))
        .stderr(predicate::str::contains("stacy.lock pins version"));

    assert_eq!(
        read_lock(&project),
        before,
        "--frozen must not rewrite stacy.lock"
    );
}

/// #96: the same holds without `--frozen`. Plain `install` installs what the
/// lockfile pins; it does not re-resolve and move the pin.
#[test]
fn test_plain_install_fails_on_pinned_version_mismatch() {
    let (checksum, _real_version) = vendored_checksum_and_version();
    let (project, cache) = project_pinning("99999999", &checksum);
    let before = read_lock(&project);

    install(&project, &cache, &[])
        .failure()
        .stderr(predicate::str::contains("stacy.lock pins version"));

    assert_eq!(
        read_lock(&project),
        before,
        "install must not rewrite stacy.lock"
    );
}

/// #96: a cold install whose bytes hash differently from the locked checksum
/// must fail rather than re-lock the served copy.
#[test]
fn test_frozen_install_fails_on_checksum_mismatch() {
    let (_checksum, version) = vendored_checksum_and_version();
    let wrong = "0".repeat(64);
    let (project, cache) = project_pinning(&version, &wrong);
    let before = read_lock(&project);

    install(&project, &cache, &["--frozen"])
        .failure()
        .stderr(predicate::str::contains("checksum mismatch"));

    assert_eq!(
        read_lock(&project),
        before,
        "--frozen must not rewrite stacy.lock"
    );
}

/// #96: the happy path also leaves the lockfile alone. A cold `--frozen`
/// install of a version the source can serve installs it into the cache and
/// writes nothing.
#[test]
fn test_frozen_install_of_matching_pin_succeeds_without_touching_lock() {
    let (checksum, version) = vendored_checksum_and_version();
    let (project, cache) = project_pinning(&version, &checksum);
    let before = read_lock(&project);

    install(&project, &cache, &["--frozen"]).success();

    assert_eq!(
        read_lock(&project),
        before,
        "a successful install must not rewrite stacy.lock"
    );

    let installed = cache_packages_dir(cache.path())
        .join("testpkg")
        .join(&version)
        .join("testpkg.ado");
    assert!(
        installed.exists(),
        "the pinned version should be in the cache at {}",
        installed.display()
    );
}
