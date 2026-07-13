//! Regression tests for issue #101: a package's own internal files must not be
//! reported as missing external dependencies.
//!
//! `ftools` ships `fcollapse_main.mata` and `ftools_type_aliases.mata` and loads
//! them from its own `.ado` files with `findfile`. Those names are provided by
//! the package itself, so `stacy doctor` must not suggest installing them, while
//! a reference to a package that is genuinely absent must still be reported.

use assert_cmd::{cargo_bin_cmd, Command};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Platform-correct cache subdirectory under a temp dir.
fn cache_packages_dir(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join("stacy").join("cache").join("packages")
    } else {
        root.join("stacy").join("packages")
    }
}

fn stacy() -> Command {
    cargo_bin_cmd!("stacy")
}

/// Build a project whose single dependency is a package that ships `files` and
/// whose main `.ado` contains `ado_body`. Returns (project_dir, cache_dir).
fn project_with_package(ado_body: &str, files: &[(&str, &str)]) -> (TempDir, TempDir) {
    let project = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();

    let pkg_cache = cache_packages_dir(cache.path())
        .join("fakepkg")
        .join("1.0.0");
    fs::create_dir_all(&pkg_cache).unwrap();
    fs::write(pkg_cache.join("fakepkg.ado"), ado_body).unwrap();
    for (name, content) in files {
        fs::write(pkg_cache.join(name), content).unwrap();
    }

    fs::write(
        project.path().join("stacy.toml"),
        "[project]\nname = \"test-project\"\n\n[packages.dependencies]\nfakepkg = \"ssc\"\n",
    )
    .unwrap();

    fs::write(
        project.path().join("stacy.lock"),
        format!(
            r#"version = "1"
stacy_version = "{}"

[packages.fakepkg]
version = "1.0.0"
checksum = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
group = "production"

[packages.fakepkg.source]
type = "SSC"
name = "fakepkg"
"#,
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();

    (project, cache)
}

/// Run `stacy doctor --format json` and return the "Package Dependencies" check.
fn package_dependencies_check(project: &TempDir, cache: &TempDir) -> serde_json::Value {
    let assert = stacy()
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .current_dir(project.path())
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .assert();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("doctor did not emit JSON ({}): {}", e, stdout));

    json["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .find(|c| c["name"] == "Package Dependencies")
        .unwrap_or_else(|| panic!("no Package Dependencies check in: {}", stdout))
        .clone()
}

/// Issue #101: names defined by files the package ships itself -- Mata source,
/// classes, do-files, further ado-files -- are not missing dependencies.
#[test]
fn test_self_provided_files_are_not_missing_deps() {
    let ado = "program define fakepkg\n\
               findfile \"fakepkg_aliases.mata\"\n\
               findfile \"fakepkg_main.mata\"\n\
               findfile \"fakepkg_helper.class\"\n\
               which fakepkg_sub\n\
               end\n";
    let (project, cache) = project_with_package(
        ado,
        &[
            ("fakepkg_aliases.mata", "// aliases"),
            ("fakepkg_main.mata", "// main"),
            ("fakepkg_helper.class", "// class"),
            ("fakepkg_sub.ado", "program define fakepkg_sub\nend"),
        ],
    );

    let check = package_dependencies_check(&project, &cache);
    assert_eq!(
        check["status"], "pass",
        "package shipping its own files must not warn: {}",
        check
    );
    assert!(
        !check["message"].as_str().unwrap().contains("fakepkg_"),
        "internal files must not be listed as dependencies: {}",
        check
    );
}

/// A reference to a package that is not shipped and not installed is still
/// reported, with a `stacy add` suggestion.
#[test]
fn test_external_dep_still_reported() {
    let ado = "program define fakepkg\n\
               findfile \"fakepkg_main.mata\"\n\
               require moremata\n\
               end\n";
    let (project, cache) = project_with_package(ado, &[("fakepkg_main.mata", "// main")]);

    let check = package_dependencies_check(&project, &cache);
    assert_eq!(
        check["status"], "warn",
        "genuinely missing dependency must warn: {}",
        check
    );
    let message = check["message"].as_str().unwrap();
    assert!(
        message.contains("moremata"),
        "external dep must be named: {}",
        check
    );
    assert!(
        !message.contains("fakepkg_main"),
        "internal file must not be named: {}",
        check
    );
    assert_eq!(
        check["suggestion"], "Run: stacy add moremata",
        "suggestion must only list installable packages: {}",
        check
    );
}
