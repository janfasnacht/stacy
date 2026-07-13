//! Regression tests for #94: a package command whose work did not complete
//! must exit non-zero and must not report `status: "success"`.
//!
//! Every case here is hermetic. Failures come from sources that cannot resolve
//! without a network (an invalid GitHub source, a closed loopback port), and
//! successes come from a stub HTTP server bound to 127.0.0.1. No test needs SSC
//! or github.com to be reachable.

use assert_cmd::{cargo_bin_cmd, Command};
use predicates::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use tempfile::TempDir;

fn stacy() -> Command {
    cargo_bin_cmd!("stacy")
}

/// A port nothing listens on: connections are refused immediately, so any
/// download against it fails without touching the network.
const CLOSED_URL: &str = "http://127.0.0.1:1/";

// ============================================================================
// Stub package server
// ============================================================================

/// Serve a fixed set of paths over HTTP on 127.0.0.1. Unknown paths get a 404,
/// which is how a package "does not exist" at a `net:` source.
fn start_stub_server(routes: HashMap<String, Vec<u8>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let Ok(peek) = stream.try_clone() else {
                continue;
            };
            let mut reader = BufReader::new(peek);

            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }
            let path = request_line
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .trim_start_matches('/')
                .to_string();

            // Drain the request headers.
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.trim().is_empty() => break,
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }

            let response = match routes.get(&path) {
                Some(body) => {
                    let mut head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .into_bytes();
                    head.extend_from_slice(body);
                    head
                }
                None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_vec(),
            };

            let _ = stream.write_all(&response);
            let _ = stream.flush();
        }
    });

    port
}

/// Serve one package (`<name>.pkg` plus its ado file) at the given date.
fn serve_package(name: &str, date: &str) -> String {
    let mut routes = HashMap::new();
    routes.insert(
        format!("{}.pkg", name),
        format!(
            "d '{}': stub package\nd Distribution-Date: {}\nf {}.ado\n",
            name.to_uppercase(),
            date,
            name
        )
        .into_bytes(),
    );
    routes.insert(
        format!("{}.ado", name),
        format!("program define {}\nend\n", name).into_bytes(),
    );

    let port = start_stub_server(routes);
    format!("http://127.0.0.1:{}/", port)
}

// ============================================================================
// Helpers
// ============================================================================

/// Project + isolated package cache, so nothing reaches the user's real cache.
struct Project {
    dir: TempDir,
    cache: TempDir,
}

impl Project {
    fn new(stacy_toml: &str) -> Self {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("stacy.toml"), stacy_toml).unwrap();
        Self {
            dir,
            cache: TempDir::new().unwrap(),
        }
    }

    fn with_lockfile(self, stacy_lock: &str) -> Self {
        std::fs::write(self.dir.path().join("stacy.lock"), stacy_lock).unwrap();
        self
    }

    fn run(&self, args: &[&str]) -> Command {
        let mut cmd = stacy();
        cmd.current_dir(self.dir.path())
            .env("XDG_CACHE_HOME", self.cache.path())
            .env("LOCALAPPDATA", self.cache.path())
            .args(args);
        cmd
    }
}

/// Run a command with `--format json`, assert the exit code, return the JSON.
fn json_after_exit(project: &Project, args: &[&str], expected_code: i32) -> Value {
    let mut argv = args.to_vec();
    argv.extend_from_slice(&["--format", "json"]);

    let output = project.run(&argv).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected_code),
        "expected exit {} from `stacy {}`, got {:?}\nstdout: {}\nstderr: {}",
        expected_code,
        args.join(" "),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {stdout}"))
}

fn assert_not_success(json: &Value) {
    let status = json["status"].as_str().unwrap_or("<missing>");
    assert_ne!(
        status, "success",
        "machine-readable status must not be success: {json}"
    );
}

// ============================================================================
// install
// ============================================================================

/// A lockfile package that cannot be fetched leaves the environment
/// incomplete: `install` must fail, not report "N skipped" and exit 0.
#[test]
fn test_install_fails_when_package_cannot_be_installed() {
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(&format!(
        r#"version = "1"

[packages.stubpkg]
version = "20200101"
group = "production"

[packages.stubpkg.source]
type = "Net"
url = "{CLOSED_URL}"
"#
    ));

    let json = json_after_exit(&project, &["install"], 1);
    assert_not_success(&json);
    assert_eq!(json["summary"]["installed"], 0);
    assert_eq!(json["summary"]["skipped"], 1);
}

/// The human surface must say so too — #94 saw failures that only JSON knew about.
#[test]
fn test_install_failure_is_reported_in_human_output() {
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(&format!(
        r#"version = "1"

[packages.stubpkg]
version = "20200101"
group = "production"

[packages.stubpkg.source]
type = "Net"
url = "{CLOSED_URL}"
"#
    ));

    project
        .run(&["install"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("failed to install"))
        .stdout(predicate::str::contains("Install complete").not())
        .stderr(predicate::str::contains("Error:"));
}

/// `--frozen` is the CI path: a skipped package there is fatal.
#[test]
fn test_install_frozen_fails_when_package_cannot_be_installed() {
    let project = Project::new(
        r#"[project]
name = "t"

[packages.dependencies]
stubpkg = "ssc"
"#,
    )
    .with_lockfile(&format!(
        r#"version = "1"

[packages.stubpkg]
version = "20200101"
group = "production"

[packages.stubpkg.source]
type = "Net"
url = "{CLOSED_URL}"
"#
    ));

    let json = json_after_exit(&project, &["install", "--frozen"], 1);
    assert_not_success(&json);
}

// ============================================================================
// lock
// ============================================================================

/// An unresolvable package means the lockfile does not describe stacy.toml.
#[test]
fn test_lock_fails_on_unresolvable_package() {
    let project = Project::new(
        r#"[project]
name = "t"

[packages.dependencies]
badpkg = "github:notauserreposlash"
"#,
    );

    let json = json_after_exit(&project, &["lock"], 1);
    assert_not_success(&json);
    assert_eq!(json["package_count"], 0);
    assert_eq!(json["failed"], 1);
    assert_eq!(json["in_sync"], false);
    assert!(!project.dir.path().join("stacy.lock").exists());
}

#[test]
fn test_lock_unresolvable_package_reported_in_human_output() {
    let project = Project::new(
        r#"[project]
name = "t"

[packages.dependencies]
badpkg = "github:notauserreposlash"
"#,
    );

    project
        .run(&["lock"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("up to date").not())
        .stderr(predicate::str::contains("could not resolve badpkg"))
        .stderr(predicate::str::contains("Error:"));
}

/// `net:` and `local:` packages carry no resolvable version. Recording nothing
/// and claiming success hid them from `install`; now lock says it cannot do it.
#[test]
fn test_lock_fails_on_source_it_cannot_resolve() {
    let project = Project::new(
        r#"[project]
name = "t"

[packages.dependencies]
mypkg = "local:./lib/mypkg/"
"#,
    );

    let json = json_after_exit(&project, &["lock"], 1);
    assert_not_success(&json);
    assert_eq!(json["failed"], 1);
}

// ============================================================================
// outdated
// ============================================================================

/// A failed version check is not evidence that everything is current.
#[test]
fn test_outdated_fails_when_a_check_fails() {
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(
        r#"version = "1"

[packages.badpkg]
version = "1.0.0"
group = "production"

[packages.badpkg.source]
type = "GitHub"
repo = "notauserreposlash"
tag = "v1.0.0"
"#,
    );

    let json = json_after_exit(&project, &["outdated"], 1);
    assert_not_success(&json);
    assert_eq!(json["failed"], 1);
    assert_eq!(json["outdated_count"], 0);
}

#[test]
fn test_outdated_does_not_claim_up_to_date_when_a_check_fails() {
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(
        r#"version = "1"

[packages.badpkg]
version = "1.0.0"
group = "production"

[packages.badpkg.source]
type = "GitHub"
repo = "notauserreposlash"
tag = "v1.0.0"
"#,
    );

    project
        .run(&["outdated"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("All packages are up to date").not())
        .stderr(predicate::str::contains("could not check badpkg"));
}

// ============================================================================
// add
// ============================================================================

/// A partial batch is a failure: the caller asked for two packages and got one.
#[test]
fn test_add_partial_batch_fails() {
    let url = serve_package("goodpkg", "20260101");
    let project = Project::new("[project]\nname = \"t\"\n");

    let json = json_after_exit(
        &project,
        &[
            "add",
            "goodpkg",
            "missingpkg",
            "--source",
            &format!("net:{}", url),
        ],
        1,
    );

    assert_not_success(&json);
    assert_eq!(json["summary"]["added"], 1);
    assert_eq!(json["summary"]["failed"], 1);

    // The package that did install is still recorded — progress is kept.
    let config = std::fs::read_to_string(project.dir.path().join("stacy.toml")).unwrap();
    assert!(config.contains("goodpkg"), "config: {config}");
}

// ============================================================================
// update
// ============================================================================

/// A partial update is a failure, even though another package updated fine.
#[test]
fn test_update_partial_batch_fails() {
    let url = serve_package("goodpkg", "20260101");
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(&format!(
        r#"version = "1"

[packages.goodpkg]
version = "20200101"
group = "production"

[packages.goodpkg.source]
type = "Net"
url = "{url}"

[packages.badpkg]
version = "20200101"
group = "production"

[packages.badpkg.source]
type = "Net"
url = "{CLOSED_URL}"
"#
    ));

    let json = json_after_exit(&project, &["update"], 1);
    assert_not_success(&json);
    assert_eq!(json["summary"]["updated"], 1);
    assert_eq!(json["summary"]["failed"], 1);
}

/// `--dry-run` used to skip the version check entirely and always answer
/// "up to date". It must really ask the source.
#[test]
fn test_update_dry_run_reports_a_real_update() {
    let url = serve_package("goodpkg", "20260101");
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(&format!(
        r#"version = "1"

[packages.goodpkg]
version = "20200101"
group = "production"

[packages.goodpkg.source]
type = "Net"
url = "{url}"
"#
    ));

    let json = json_after_exit(&project, &["update", "--dry-run"], 0);
    assert_eq!(json["status"], "success");
    assert_eq!(json["summary"]["updates_available"], 1);
    assert_eq!(json["summary"]["updated"], 0);
    assert_eq!(json["packages"][0]["new_version"], "20260101");

    // A dry run installs nothing: the lockfile keeps the old version.
    let lock = std::fs::read_to_string(project.dir.path().join("stacy.lock")).unwrap();
    assert!(lock.contains("20200101"), "lockfile: {lock}");
}

/// A check that could not run is not an "up to date".
#[test]
fn test_update_dry_run_fails_when_a_check_fails() {
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(&format!(
        r#"version = "1"

[packages.badpkg]
version = "20200101"
group = "production"

[packages.badpkg.source]
type = "Net"
url = "{CLOSED_URL}"
"#
    ));

    let json = json_after_exit(&project, &["update", "--dry-run"], 1);
    assert_not_success(&json);
    assert_eq!(json["summary"]["failed"], 1);

    project
        .run(&["update", "--dry-run"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("All packages are up to date").not());
}

/// A `local:` package is not fetched from anywhere, so `update` has nothing to
/// do for it. Skipping it is not a failure — the command must still exit 0 when
/// every other package updated.
#[test]
fn test_update_skips_local_package_without_failing() {
    let url = serve_package("goodpkg", "20260101");
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(&format!(
        r#"version = "1"

[packages.goodpkg]
version = "20200101"
group = "production"

[packages.goodpkg.source]
type = "Net"
url = "{url}"

[packages.mylocal]
version = "20200101"
group = "production"

[packages.mylocal.source]
type = "Local"
path = "./lib/mylocal/"
"#
    ));

    let json = json_after_exit(&project, &["update"], 0);
    assert_eq!(json["status"], "success");
    assert_eq!(json["summary"]["updated"], 1);
    assert_eq!(json["summary"]["failed"], 0);
    assert_eq!(json["summary"]["skipped"], 1);
}

/// The same holds for a dry run, and for a local package named on its own.
#[test]
fn test_update_local_package_alone_succeeds() {
    let project = Project::new("[project]\nname = \"t\"\n").with_lockfile(
        r#"version = "1"

[packages.mylocal]
version = "20200101"
group = "production"

[packages.mylocal.source]
type = "Local"
path = "./lib/mylocal/"
"#,
    );

    let json = json_after_exit(&project, &["update", "mylocal", "--dry-run"], 0);
    assert_eq!(json["status"], "success");
    assert_eq!(json["summary"]["failed"], 0);
    assert_eq!(json["summary"]["skipped"], 1);
}

// ============================================================================
// deps
// ============================================================================

/// A cycle means the graph could not be resolved.
#[test]
fn test_deps_circular_fails() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.do"), "do \"b.do\"").unwrap();
    std::fs::write(temp.path().join("b.do"), "do \"a.do\"").unwrap();

    let output = stacy()
        .arg("deps")
        .arg(temp.path().join("a.do"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_not_success(&json);
    assert_eq!(json["summary"]["has_circular"], true);
}

/// A dependency that does not exist is a file error, like a missing script.
#[test]
fn test_deps_missing_dependency_fails() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("main.do"), "do \"gone.do\"").unwrap();

    let output = stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_not_success(&json);
    assert_eq!(json["summary"]["has_missing"], true);
}

/// The Stata surface branches on `$stacy_status`, so it must not say success.
#[test]
fn test_deps_missing_dependency_stata_status_is_error() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("main.do"), "do \"gone.do\"").unwrap();

    let output = stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .arg("--format")
        .arg("stata")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("global stacy_status \"error\""),
        "stdout: {stdout}"
    );
}

#[test]
fn test_deps_clean_graph_still_succeeds() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("main.do"), "do \"helper.do\"").unwrap();
    std::fs::write(temp.path().join("helper.do"), "display 1").unwrap();

    stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""));
}

/// A path built from a macro (`do "$root/prep.do"`) only exists once Stata has
/// run the script. stacy reads the script, it does not run it, so the path is
/// unresolved — not missing. It must not fail the command.
#[test]
fn test_deps_macro_path_is_unresolved_not_missing() {
    let temp = TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("main.do"),
        "global root \"/data\"\nlocal sub \"steps\"\ndo \"$root/prep.do\"\ndo \"`sub'/clean.do\"\ndo \"helper.do\"\n",
    )
    .unwrap();
    std::fs::write(temp.path().join("helper.do"), "display 1").unwrap();

    let output = stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "success");
    assert_eq!(json["summary"]["has_missing"], false);
    assert_eq!(json["summary"]["missing_count"], 0);
    assert_eq!(json["summary"]["unresolved_count"], 2);
}

/// A macro path is reported, but as a note, not an error.
#[test]
fn test_deps_macro_path_is_reported_in_human_output() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("main.do"), "do \"$root/prep.do\"\n").unwrap();

    stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .assert()
        .success()
        .stdout(predicate::str::contains("$root/prep.do"))
        .stderr(predicate::str::contains("Error").not());
}

/// A real missing file next to a macro path is still an error.
#[test]
fn test_deps_missing_file_still_fails_alongside_macro_path() {
    let temp = TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("main.do"),
        "do \"$root/prep.do\"\ndo \"gone.do\"\n",
    )
    .unwrap();

    let output = stacy()
        .arg("deps")
        .arg(temp.path().join("main.do"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_not_success(&json);
    assert_eq!(json["summary"]["missing_count"], 1);
}
