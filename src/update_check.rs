//! Update notification on CLI startup
//!
//! Checks for new stacy releases using cached data from GitHub Releases API.
//! Prints a notification to stderr if an update is available, then spawns a
//! background thread to refresh the cache for the next invocation.

use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache file storing the result of the last version check
const CACHE_FILE: &str = "version-check.json";

/// Plain-text flag file for Stata to read
const FLAG_FILE: &str = "update-available";

/// Cache is considered fresh for 24 hours
const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// Timeout for GitHub API requests
const REQUEST_TIMEOUT_SECS: u64 = 3;

/// GitHub API endpoint for latest release
const RELEASES_URL: &str = "https://api.github.com/repos/janfasnacht/stacy/releases/latest";

/// Cached version check result, serialized to `~/.cache/stacy/version-check.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCheckCache {
    pub current_version: String,
    pub latest_version: String,
    pub checked_at_unix: u64,
    pub update_available: bool,
}

/// How stacy was installed, used to generate upgrade instructions
#[derive(Debug, Clone, PartialEq)]
pub enum InstallMethod {
    Homebrew,
    Cargo,
    Manual,
}

/// Entry point: print cached notification, spawn background refresh.
///
/// Called as the first thing in `main()`. All errors are silently ignored
/// so this never interferes with normal CLI operation.
pub fn maybe_notify_and_spawn() {
    if should_suppress() {
        return;
    }

    // Read cache and print notification if update is available
    if let Some(cache) = load_cached_update() {
        if cache.update_available {
            print_notification(&cache.current_version, &cache.latest_version);
        }

        // If cache is fresh, no need to refresh
        if is_cache_fresh(&cache) {
            return;
        }
    }

    // Spawn background thread to refresh cache
    let current = env!("CARGO_PKG_VERSION").to_string();
    std::thread::spawn(move || {
        refresh_cache(&current);
    });
}

/// Check if update notifications should be suppressed.
fn should_suppress() -> bool {
    // CI environments
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        return true;
    }

    // Explicit opt-out via environment variable
    if std::env::var("STACY_NO_UPDATE_CHECK").is_ok() {
        return true;
    }

    // Not a TTY (piped output, scripts, etc.)
    if !std::io::stderr().is_terminal() {
        return true;
    }

    // User config opt-out
    if let Ok(Some(config)) = crate::project::user_config::load_user_config() {
        if config.update_check == Some(false) {
            return true;
        }
    }

    false
}

/// Load the cached version check from disk.
pub fn load_cached_update() -> Option<VersionCheckCache> {
    let path = cache_dir()?.join(CACHE_FILE);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Check if the cache is still fresh (less than 24 hours old).
pub fn is_cache_fresh(cache: &VersionCheckCache) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(cache.checked_at_unix) < CACHE_TTL_SECS
}

/// Print update notification to stderr.
fn print_notification(current: &str, latest: &str) {
    use colored::Colorize;
    let method = detect_install_method();
    let instruction = upgrade_instruction(&method);
    eprintln!(
        "\n{} v{} {} v{}\nRun {} to update\n",
        "Update available:".yellow().bold(),
        current,
        "→".dimmed(),
        latest.green().bold(),
        format!("`{instruction}`").cyan(),
    );
}

/// Fetch latest version from GitHub and update cache + flag file.
fn refresh_cache(current: &str) {
    let Some(latest) = fetch_latest_version() else {
        return;
    };

    let update_available = compare_versions(current, &latest);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cache = VersionCheckCache {
        current_version: current.to_string(),
        latest_version: latest.clone(),
        checked_at_unix: now,
        update_available,
    };

    // Write JSON cache (atomic: write to .tmp then rename)
    if let Some(dir) = cache_dir() {
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(json) = serde_json::to_string_pretty(&cache) {
            let tmp_path = dir.join(format!("{CACHE_FILE}.tmp"));
            let final_path = dir.join(CACHE_FILE);
            if std::fs::write(&tmp_path, json).is_ok() {
                let _ = std::fs::rename(&tmp_path, &final_path);
            }
        }

        // Write or remove plain-text flag file for Stata (atomic)
        let flag_path = dir.join(FLAG_FILE);
        if update_available {
            let method = detect_install_method();
            let instruction = upgrade_instruction(&method);
            let content = format!("{}\n{}\n{}\n", current, latest, instruction);
            let tmp_flag = dir.join(format!("{FLAG_FILE}.tmp"));
            if std::fs::write(&tmp_flag, content).is_ok() {
                let _ = std::fs::rename(&tmp_flag, &flag_path);
            }
        } else {
            let _ = std::fs::remove_file(flag_path);
        }
    }
}

/// Fetch the latest release version from GitHub Releases API.
fn fetch_latest_version() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .ok()?;

    let resp = client
        .get(RELEASES_URL)
        .header("User-Agent", format!("stacy/{}", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = resp.json().ok()?;
    let tag = body.get("tag_name")?.as_str()?;
    Some(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

/// Compare two semver-like version strings. Returns true if latest > current.
pub fn compare_versions(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split('.')
            .filter_map(|part| part.parse::<u64>().ok())
            .collect()
    };

    let c = parse(current);
    let l = parse(latest);

    // Compare component by component
    for i in 0..c.len().max(l.len()) {
        let cv = c.get(i).copied().unwrap_or(0);
        let lv = l.get(i).copied().unwrap_or(0);
        if lv > cv {
            return true;
        }
        if lv < cv {
            return false;
        }
    }
    false
}

/// Detect how stacy was installed based on the executable path.
pub fn detect_install_method() -> InstallMethod {
    let exe = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return InstallMethod::Manual,
    };

    if exe.contains("/Cellar/") || exe.contains("/homebrew/") {
        InstallMethod::Homebrew
    } else if exe.contains("/.cargo/bin/") {
        InstallMethod::Cargo
    } else {
        InstallMethod::Manual
    }
}

/// Get the upgrade instruction for a given install method.
pub fn upgrade_instruction(method: &InstallMethod) -> &'static str {
    match method {
        InstallMethod::Homebrew => "brew upgrade stacy",
        InstallMethod::Cargo => "cargo install stata-cli",
        InstallMethod::Manual => "download from https://github.com/janfasnacht/stacy/releases",
    }
}

/// Get the cache directory for stacy (`~/.cache/stacy/`).
fn cache_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("AppData")
                    .join("Local")
            })
            .join("stacy")
            .join("cache")
            .into()
    } else {
        std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".cache")
            })
            .join("stacy")
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions_newer() {
        assert!(compare_versions("0.1.0", "0.2.0"));
        assert!(compare_versions("0.1.0", "1.0.0"));
        assert!(compare_versions("1.0.0", "1.0.1"));
        assert!(compare_versions("0.9.9", "1.0.0"));
        assert!(compare_versions("1.2.3", "1.2.4"));
    }

    #[test]
    fn test_compare_versions_equal() {
        assert!(!compare_versions("1.0.0", "1.0.0"));
        assert!(!compare_versions("0.1.0", "0.1.0"));
    }

    #[test]
    fn test_compare_versions_older() {
        assert!(!compare_versions("1.0.0", "0.9.0"));
        assert!(!compare_versions("2.0.0", "1.9.9"));
        assert!(!compare_versions("0.2.0", "0.1.0"));
    }

    #[test]
    fn test_compare_versions_different_lengths() {
        assert!(compare_versions("1.0", "1.0.1"));
        assert!(!compare_versions("1.0.1", "1.0"));
    }

    #[test]
    fn test_cache_round_trip() {
        let cache = VersionCheckCache {
            current_version: "0.1.0".to_string(),
            latest_version: "0.2.0".to_string(),
            checked_at_unix: 1700000000,
            update_available: true,
        };

        let json = serde_json::to_string(&cache).unwrap();
        let deserialized: VersionCheckCache = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.current_version, "0.1.0");
        assert_eq!(deserialized.latest_version, "0.2.0");
        assert_eq!(deserialized.checked_at_unix, 1700000000);
        assert!(deserialized.update_available);
    }

    #[test]
    fn test_cache_freshness() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let fresh = VersionCheckCache {
            current_version: "0.1.0".to_string(),
            latest_version: "0.1.0".to_string(),
            checked_at_unix: now - 3600, // 1 hour ago
            update_available: false,
        };
        assert!(is_cache_fresh(&fresh));

        let stale = VersionCheckCache {
            current_version: "0.1.0".to_string(),
            latest_version: "0.1.0".to_string(),
            checked_at_unix: now - (25 * 3600), // 25 hours ago
            update_available: false,
        };
        assert!(!is_cache_fresh(&stale));
    }

    #[test]
    fn test_should_suppress_ci() {
        // Save and restore env
        let ci_was = std::env::var("CI").ok();
        let tty_issue = !std::io::stderr().is_terminal();

        std::env::set_var("CI", "true");
        // CI should always suppress regardless of TTY
        assert!(should_suppress());

        // Restore
        if let Some(val) = ci_was {
            std::env::set_var("CI", val);
        } else {
            std::env::remove_var("CI");
        }

        // If we're not in a TTY (common in test environments), that also suppresses
        if tty_issue {
            assert!(should_suppress());
        }
    }

    #[test]
    fn test_detect_install_method_manual() {
        // In test environment, exe path won't contain Cellar or .cargo/bin
        // (unless running via cargo test, which does contain .cargo)
        let method = detect_install_method();
        // Should be either Cargo (if running via cargo test) or Manual
        assert!(method == InstallMethod::Cargo || method == InstallMethod::Manual);
    }

    #[test]
    fn test_upgrade_instruction() {
        assert_eq!(
            upgrade_instruction(&InstallMethod::Homebrew),
            "brew upgrade stacy"
        );
        assert_eq!(
            upgrade_instruction(&InstallMethod::Cargo),
            "cargo install stata-cli"
        );
        assert_eq!(
            upgrade_instruction(&InstallMethod::Manual),
            "download from https://github.com/janfasnacht/stacy/releases"
        );
    }
}
