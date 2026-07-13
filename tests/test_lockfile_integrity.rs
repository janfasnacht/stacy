//! Regression tests for the lockfile integrity guarantees (#96, #97).
//!
//! #96: `stacy install` materializes `stacy.lock`; it never rewrites it. On a
//! cold cache it must fetch the pinned version and fail if the source no longer
//! serves it, rather than quietly locking whatever the source serves today.
//!
//! #97: `stacy run` checks the locked packages against the cache before it
//! starts Stata, so a modified or absent package cannot run silently.
//!
//! These tests use a `local:` package source, a package server on 127.0.0.1,
//! and a fake Stata binary, so they run without an internet connection and
//! without Stata.

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
    project_pinning_in_group(version, checksum, "production")
}

/// As `project_pinning`, with the locked package placed in a dependency group.
fn project_pinning_in_group(version: &str, checksum: &str, group: &str) -> (TempDir, TempDir) {
    let project = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();

    // The package source on disk — no network needed to install it.
    let vendor = project.path().join("vendor").join("testpkg");
    fs::create_dir_all(&vendor).unwrap();
    fs::write(vendor.join("testpkg.ado"), PKG_ADO).unwrap();

    let toml_table = match group {
        "production" => "[packages.dependencies]",
        "dev" => "[packages.dev]",
        "test" => "[packages.test]",
        other => panic!("unknown group {}", other),
    };
    fs::write(
        project.path().join("stacy.toml"),
        format!(
            "[project]\nname = \"test-project\"\n\n\
             {}\ntestpkg = \"local:vendor/testpkg\"\n",
            toml_table
        ),
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
group = "{}"

[packages.testpkg.source]
type = "Local"
path = "vendor/testpkg"
"#,
            env!("CARGO_PKG_VERSION"),
            version,
            checksum,
            group,
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

// ============================================================================
// #97: `stacy run` checks the cache against the lockfile before starting Stata
// ============================================================================

/// Put a package into the global cache by hand, as a completed install would.
#[cfg(unix)]
fn seed_cache(cache: &TempDir, version: &str, contents: &[u8]) {
    let dir = cache_packages_dir(cache.path())
        .join("testpkg")
        .join(version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("testpkg.ado"), contents).unwrap();
}

/// Stand-in for Stata: writes the log `stacy` expects, runs nothing.
#[cfg(unix)]
fn write_fake_stata(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("fake-stata");
    fs::write(
        &path,
        "#!/bin/sh\n\
         for arg in \"$@\"; do last=\"$arg\"; done\n\
         stem=$(basename \"$last\" .do)\n\
         printf '%s\\n' 'output' 'end of do-file' > \"$stem.log\"\n",
    )
    .unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
fn run_script(project: &TempDir, cache: &TempDir) -> assert_cmd::assert::Assert {
    run_script_with(project, cache, &[])
}

#[cfg(unix)]
fn run_script_with(
    project: &TempDir,
    cache: &TempDir,
    extra: &[&str],
) -> assert_cmd::assert::Assert {
    fs::write(project.path().join("analysis.do"), "display 1\n").unwrap();
    let fake = write_fake_stata(project.path());

    stacy()
        .arg("run")
        .arg("analysis.do")
        .args(extra)
        .current_dir(project.path())
        .env("STATA_BINARY", &fake)
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .assert()
}

/// #97: a cached package modified after install must not run.
#[cfg(unix)]
#[test]
fn test_run_fails_on_modified_cached_package() {
    let (checksum, version) = vendored_checksum_and_version();
    let (project, cache) = project_pinning(&version, &checksum);

    seed_cache(
        &cache,
        &version,
        b"program define testpkg\nend\n* TAMPERED\n",
    );

    run_script(&project, &cache)
        .failure()
        .stderr(predicate::str::contains("does not match stacy.lock"))
        .stderr(predicate::str::contains("modified since install"))
        .stderr(predicate::str::contains("testpkg"));
}

/// #97: a locked package that is not in the cache must not run.
#[cfg(unix)]
#[test]
fn test_run_fails_on_absent_cached_package() {
    let (checksum, version) = vendored_checksum_and_version();
    let (project, cache) = project_pinning(&version, &checksum);

    // Cache left empty — nothing was ever installed.
    run_script(&project, &cache)
        .failure()
        .stderr(predicate::str::contains("does not match stacy.lock"))
        .stderr(predicate::str::contains("not installed"))
        .stderr(predicate::str::contains("testpkg"));
}

/// #97 control: an intact cache runs. Without this, the two failure tests
/// above would pass even if `run` were broken for unrelated reasons.
#[cfg(unix)]
#[test]
fn test_run_succeeds_with_intact_cache() {
    let (checksum, version) = vendored_checksum_and_version();
    let (project, cache) = project_pinning(&version, &checksum);

    seed_cache(&cache, &version, PKG_ADO);

    run_script(&project, &cache).success();
}

// ============================================================================
// Dependency groups: `stacy install` installs production, so that is the group
// `run` requires. A dev or test package is only checked once it is installed.
// ============================================================================

/// `stacy install` (production only) followed by `stacy run` must work in a
/// project that has a dev dependency. The dev package is not installed, and
/// requiring it would break the standard install-then-run flow.
#[cfg(unix)]
#[test]
fn test_run_succeeds_with_uninstalled_dev_package() {
    let (checksum, version) = vendored_checksum_and_version();
    let (project, cache) = project_pinning_in_group(&version, &checksum, "dev");

    // Installs the production group: nothing, and no error.
    install(&project, &cache, &[]).success();

    run_script(&project, &cache).success();
}

/// A dev package that *is* installed is still checked: it sits on the ado-path
/// like any other locked package, so modified bytes must not run.
#[cfg(unix)]
#[test]
fn test_run_fails_on_modified_dev_package() {
    let (checksum, version) = vendored_checksum_and_version();
    let (project, cache) = project_pinning_in_group(&version, &checksum, "dev");

    seed_cache(
        &cache,
        &version,
        b"program define testpkg\nend\n* TAMPERED\n",
    );

    run_script(&project, &cache)
        .failure()
        .stderr(predicate::str::contains("modified since install"))
        .stderr(predicate::str::contains("testpkg"));
}

// ============================================================================
// `--no-verify` on install and on run are counterparts: a cache installed
// without checking does not match the lockfile, so run needs the same opt-out.
// ============================================================================

/// `stacy install --no-verify` installs a copy that does not match the locked
/// checksum. `run` rejects that cache, and `run --no-verify` accepts it — the
/// escape hatch the install hint points at has to lead somewhere.
#[cfg(unix)]
#[test]
fn test_no_verify_install_then_run_needs_no_verify() {
    let (_checksum, version) = vendored_checksum_and_version();
    let stale = "0".repeat(64);
    let (project, cache) = project_pinning(&version, &stale);

    install(&project, &cache, &["--no-verify"]).success();

    run_script(&project, &cache)
        .failure()
        .stderr(predicate::str::contains("does not match stacy.lock"));

    run_script_with(&project, &cache, &["--no-verify"]).success();
}

// ============================================================================
// Packages whose manifest names no version. `stacy add` records the date it
// fetched them, which says nothing about the bytes — so the checksum, not the
// version, decides whether a cold-cache install satisfies the pin.
// ============================================================================

/// A `.pkg` with no `Distribution-Date` line, plus the one file it lists.
const NODATE_PKG: &[u8] =
    b"d 'NODATE': a package that declares no distribution date\nf nodate.ado\n";
const NODATE_ADO: &[u8] = b"program define nodate\nend\n";

/// Serve fixed routes over HTTP on 127.0.0.1. Returns the base URL.
///
/// One request per connection, answered with `Connection: close`, which is all
/// the package downloader needs and keeps the server to a few lines.
fn serve(routes: Vec<(String, Vec<u8>)>) -> String {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };

            let mut buf = [0u8; 2048];
            let read = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..read]).to_string();
            let path = request.split_whitespace().nth(1).unwrap_or("").to_string();

            let body = routes
                .iter()
                .find(|(route, _)| *route == path)
                .map(|(_, body)| body.clone());

            let response = match body {
                Some(body) => {
                    let mut head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .into_bytes();
                    head.extend_from_slice(&body);
                    head
                }
                None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_vec(),
            };

            let _ = stream.write_all(&response);
            let _ = stream.flush();
        }
    });

    base
}

/// Project whose only package comes from `url`, pinned as given.
fn project_pinning_net(url: &str, version: &str, checksum: &str) -> (TempDir, TempDir) {
    let project = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();

    fs::write(
        project.path().join("stacy.toml"),
        format!(
            "[project]\nname = \"test-project\"\n\n\
             [packages.dependencies]\nnodate = \"net:{}\"\n",
            url
        ),
    )
    .unwrap();

    fs::write(
        project.path().join("stacy.lock"),
        format!(
            r#"version = "1"
stacy_version = "{}"

[packages.nodate]
version = "{}"
checksum = "sha256:{}"
group = "production"

[packages.nodate.source]
type = "Net"
url = "{}"
"#,
            env!("CARGO_PKG_VERSION"),
            version,
            checksum,
            url,
        ),
    )
    .unwrap();

    (project, cache)
}

/// A package with no `Distribution-Date` was locked with the date it was added.
/// A cold-cache install on any later day must still install it: the checksum
/// proves the bytes are the ones that were locked. Enforcing the version here
/// would make the package permanently uninstallable the day after `stacy add`.
#[test]
fn test_install_of_undated_package_succeeds_when_checksum_matches() {
    let url = serve(vec![
        ("/nodate.pkg".to_string(), NODATE_PKG.to_vec()),
        ("/nodate.ado".to_string(), NODATE_ADO.to_vec()),
    ]);
    let checksum = combined_checksum(&[sha256_hex(NODATE_ADO)]);

    // Locked on some earlier day — the date the package was added.
    let (project, cache) = project_pinning_net(&url, "20200101", &checksum);
    let before = read_lock(&project);

    install(&project, &cache, &["--frozen"]).success();

    assert_eq!(
        read_lock(&project),
        before,
        "install must not rewrite stacy.lock"
    );

    let installed = cache_packages_dir(cache.path())
        .join("nodate")
        .join("20200101")
        .join("nodate.ado");
    assert!(
        installed.exists(),
        "the package should be cached under the pinned version, at {}",
        installed.display()
    );
}

/// The checksum still guards an undated package: different bytes under the same
/// pin fail, since the checksum is the only identity such a package has.
#[test]
fn test_install_of_undated_package_fails_on_checksum_mismatch() {
    let url = serve(vec![
        ("/nodate.pkg".to_string(), NODATE_PKG.to_vec()),
        ("/nodate.ado".to_string(), NODATE_ADO.to_vec()),
    ]);
    let wrong = "0".repeat(64);

    let (project, cache) = project_pinning_net(&url, "20200101", &wrong);
    let before = read_lock(&project);

    install(&project, &cache, &["--frozen"])
        .failure()
        .stderr(predicate::str::contains("checksum mismatch"));

    assert_eq!(
        read_lock(&project),
        before,
        "a failed install must not rewrite stacy.lock"
    );
}
