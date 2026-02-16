//! Global package cache management
//!
//! Manages a global cache of Stata packages at `~/.cache/stacy/packages/`.
//! Packages are stored by name and version: `{cache_dir}/{name}/{version}/`
//!
//! At runtime, S_ADO is constructed dynamically from the lockfile to point
//! to the cached packages.

use crate::error::{Error, Result};
use crate::project::Lockfile;
use std::path::PathBuf;

/// Get the global package cache directory.
///
/// Uses XDG Base Directory Specification:
/// - Linux/macOS: `~/.cache/stacy/packages/`
/// - Windows: `%LOCALAPPDATA%/stacy/cache/packages/`
///
/// Falls back to `~/.cache/stacy/packages/` if XDG_CACHE_HOME is not set.
pub fn cache_dir() -> Result<PathBuf> {
    let cache_base = if cfg!(windows) {
        // Windows: use LOCALAPPDATA
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
    } else {
        // Unix: use XDG_CACHE_HOME or ~/.cache
        std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".cache")
            })
            .join("stacy")
    };

    Ok(cache_base.join("packages"))
}

/// Get the path to a specific package version in the cache.
///
/// Returns: `{cache_dir}/{name}/{version}/`
pub fn package_path(name: &str, version: &str) -> Result<PathBuf> {
    let cache = cache_dir()?;
    Ok(cache.join(name.to_lowercase()).join(version))
}

/// Check if a package version is cached.
///
/// Returns true if the package directory exists and contains at least one file.
pub fn is_cached(name: &str, version: &str) -> Result<bool> {
    let path = package_path(name, version)?;

    if !path.exists() {
        return Ok(false);
    }

    // Check if there are any files in the directory
    let entries = std::fs::read_dir(&path).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read cache directory {}: {}", path.display(), e),
        ))
    })?;

    Ok(entries.count() > 0)
}

/// Ensure the cache directory for a package exists.
pub fn ensure_package_cache_dir(name: &str, version: &str) -> Result<PathBuf> {
    let path = package_path(name, version)?;

    if !path.exists() {
        std::fs::create_dir_all(&path).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create cache directory {}: {}", path.display(), e),
            ))
        })?;
    }

    Ok(path)
}

/// Build the S_ADO environment variable from a lockfile.
///
/// By default (strict mode), only locked packages and BASE are included.
/// This ensures scripts fail fast if they use unlocked packages.
///
/// With `allow_global = true`, also includes SITE, PERSONAL, PLUS, OLDPLACE
/// for convenience during development.
///
/// Strict format: `{pkg1_cache};{pkg2_cache};...;BASE`
/// Global format: `{pkg1_cache};{pkg2_cache};...;BASE;SITE;PERSONAL;PLUS;OLDPLACE`
pub fn build_s_ado(lockfile: &Lockfile, allow_global: bool) -> Result<String> {
    let mut paths = Vec::new();

    // Sort packages alphabetically for deterministic S_ADO order
    let mut sorted_packages: Vec<_> = lockfile.packages.iter().collect();
    sorted_packages.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (name, entry) in sorted_packages {
        let pkg_path = package_path(name, &entry.version)?;
        paths.push(pkg_path.display().to_string());
    }

    // Always include Stata's built-in commands
    paths.push("BASE".to_string());

    // Optionally include global package locations
    if allow_global {
        paths.push("SITE".to_string());
        paths.push("PERSONAL".to_string());
        paths.push("PLUS".to_string());
        paths.push("OLDPLACE".to_string());
    }

    Ok(paths.join(";"))
}

/// Build the S_ADO environment variable from a lockfile, filtered by groups.
///
/// Only includes packages whose group is in the specified list.
/// Uses strict mode by default (only locked packages + BASE).
pub fn build_s_ado_for_groups(
    lockfile: &Lockfile,
    groups: &[&str],
    allow_global: bool,
) -> Result<String> {
    let mut paths = Vec::new();

    // Sort packages alphabetically for deterministic S_ADO order
    let mut sorted_packages: Vec<_> = lockfile.packages.iter().collect();
    sorted_packages.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (name, entry) in sorted_packages {
        if groups.contains(&entry.group.as_str()) {
            let pkg_path = package_path(name, &entry.version)?;
            paths.push(pkg_path.display().to_string());
        }
    }

    // Always include Stata's built-in commands
    paths.push("BASE".to_string());

    // Optionally include global package locations
    if allow_global {
        paths.push("SITE".to_string());
        paths.push("PERSONAL".to_string());
        paths.push("PLUS".to_string());
        paths.push("OLDPLACE".to_string());
    }

    Ok(paths.join(";"))
}

/// List all cached packages.
///
/// Returns a list of (name, version, path) tuples for each cached package.
pub fn list_cached_packages() -> Result<Vec<(String, String, PathBuf)>> {
    let cache = cache_dir()?;
    let mut packages = Vec::new();

    if !cache.exists() {
        return Ok(packages);
    }

    // Iterate over package directories
    let entries = std::fs::read_dir(&cache).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read cache directory {}: {}", cache.display(), e),
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read entry: {}", e),
            ))
        })?;

        let pkg_name = entry.file_name().to_string_lossy().to_string();
        let pkg_path = entry.path();

        if !pkg_path.is_dir() {
            continue;
        }

        // Iterate over version directories
        let versions = std::fs::read_dir(&pkg_path).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to read package directory {}: {}",
                    pkg_path.display(),
                    e
                ),
            ))
        })?;

        for version_entry in versions {
            let version_entry = version_entry.map_err(|e| {
                Error::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read entry: {}", e),
                ))
            })?;

            let version = version_entry.file_name().to_string_lossy().to_string();
            let version_path = version_entry.path();

            if version_path.is_dir() {
                packages.push((pkg_name.clone(), version, version_path));
            }
        }
    }

    // Sort by package name, then version
    packages.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    Ok(packages)
}

/// Get the total size of the package cache in bytes.
pub fn cache_size_bytes() -> Result<u64> {
    let cache = cache_dir()?;

    if !cache.exists() {
        return Ok(0);
    }

    calculate_dir_size(&cache)
}

/// Recursively calculate directory size.
fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0;

    if path.is_file() {
        return Ok(std::fs::metadata(path).map(|m| m.len()).unwrap_or(0));
    }

    let entries = std::fs::read_dir(path).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read directory {}: {}", path.display(), e),
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read entry: {}", e),
            ))
        })?;

        let entry_path = entry.path();
        if entry_path.is_file() {
            size += std::fs::metadata(&entry_path).map(|m| m.len()).unwrap_or(0);
        } else if entry_path.is_dir() {
            size += calculate_dir_size(&entry_path)?;
        }
    }

    Ok(size)
}

/// Remove a specific package version from the cache.
pub fn remove_cached_package(name: &str, version: &str) -> Result<()> {
    let path = package_path(name, version)?;

    if path.exists() {
        std::fs::remove_dir_all(&path).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to remove cached package {}: {}", path.display(), e),
            ))
        })?;

        // Try to remove the parent directory if empty
        if let Some(parent) = path.parent() {
            if parent.exists() {
                if let Ok(entries) = std::fs::read_dir(parent) {
                    if entries.count() == 0 {
                        let _ = std::fs::remove_dir(parent);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Clean the entire package cache.
pub fn clean_cache() -> Result<usize> {
    let cache = cache_dir()?;

    if !cache.exists() {
        return Ok(0);
    }

    let packages = list_cached_packages()?;
    let count = packages.len();

    std::fs::remove_dir_all(&cache).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to clean cache {}: {}", cache.display(), e),
        ))
    })?;

    Ok(count)
}

/// Clean unused packages from the cache.
///
/// A package is considered unused if it's not referenced by any lockfile
/// in the provided list of lockfile paths.
pub fn clean_unused_packages(lockfiles: &[&Lockfile]) -> Result<usize> {
    let cached = list_cached_packages()?;
    let mut removed = 0;

    // Build set of (name, version) pairs that are in use
    let mut in_use: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for lockfile in lockfiles {
        for (name, entry) in &lockfile.packages {
            in_use.insert((name.to_lowercase(), entry.version.clone()));
        }
    }

    // Remove packages not in use
    for (name, version, _path) in cached {
        if !in_use.contains(&(name.to_lowercase(), version.clone())) {
            remove_cached_package(&name, &version)?;
            removed += 1;
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Build a path fragment using the platform separator.
    /// e.g., `pkg_path("estout", "2024.03.15")` → `"estout/2024.03.15"` (Unix)
    ///   or `"estout\\2024.03.15"` (Windows)
    fn pkg_path_str(name: &str, version: &str) -> String {
        PathBuf::from(name).join(version).display().to_string()
    }

    // Helper to set up a test cache directory
    fn with_test_cache<F, R>(f: F) -> R
    where
        F: FnOnce(&TempDir) -> R,
    {
        let temp = TempDir::new().unwrap();
        // Override cache location for testing.
        // On Unix, cache_dir() reads XDG_CACHE_HOME.
        // On Windows, cache_dir() reads LOCALAPPDATA.
        // Set both so tests are isolated on all platforms.
        let prev_xdg = std::env::var("XDG_CACHE_HOME").ok();
        let prev_localappdata = std::env::var("LOCALAPPDATA").ok();
        std::env::set_var("XDG_CACHE_HOME", temp.path());
        std::env::set_var("LOCALAPPDATA", temp.path());
        let result = f(&temp);
        // Restore previous values
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        match prev_localappdata {
            Some(v) => std::env::set_var("LOCALAPPDATA", v),
            None => std::env::remove_var("LOCALAPPDATA"),
        }
        result
    }

    #[test]
    #[serial]
    fn test_cache_dir_xdg_compliant() {
        with_test_cache(|temp| {
            let cache = cache_dir().unwrap();
            assert!(cache.starts_with(temp.path()));
            // Windows: LOCALAPPDATA/stacy/cache/packages
            // Unix:    XDG_CACHE_HOME/stacy/packages
            assert!(cache.ends_with("packages"));
            assert!(cache.components().any(|c| c.as_os_str() == "stacy"));
        });
    }

    #[test]
    #[serial]
    fn test_package_path_structure() {
        with_test_cache(|_temp| {
            let path = package_path("estout", "2024.03.15").unwrap();
            assert!(path.ends_with("packages/estout/2024.03.15"));
        });
    }

    #[test]
    #[serial]
    fn test_package_path_lowercase() {
        with_test_cache(|_temp| {
            let path = package_path("ESTOUT", "2024.03.15").unwrap();
            assert!(path.ends_with("packages/estout/2024.03.15"));
        });
    }

    #[test]
    #[serial]
    fn test_is_cached_false_nonexistent() {
        with_test_cache(|_temp| {
            assert!(!is_cached("nonexistent", "1.0.0").unwrap());
        });
    }

    #[test]
    #[serial]
    fn test_is_cached_true_with_files() {
        with_test_cache(|_temp| {
            let pkg_path = ensure_package_cache_dir("testpkg", "1.0.0").unwrap();
            std::fs::write(pkg_path.join("testpkg.ado"), "test content").unwrap();

            assert!(is_cached("testpkg", "1.0.0").unwrap());
        });
    }

    #[test]
    #[serial]
    fn test_build_s_ado_empty_lockfile_strict() {
        with_test_cache(|_temp| {
            let lockfile = Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages: HashMap::new(),
            };

            // Strict mode (default): only BASE
            let s_ado = build_s_ado(&lockfile, false).unwrap();
            assert_eq!(s_ado, "BASE");
        });
    }

    #[test]
    #[serial]
    fn test_build_s_ado_empty_lockfile_allow_global() {
        with_test_cache(|_temp| {
            let lockfile = Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages: HashMap::new(),
            };

            // Allow global: includes all standard paths
            let s_ado = build_s_ado(&lockfile, true).unwrap();
            assert_eq!(s_ado, "BASE;SITE;PERSONAL;PLUS;OLDPLACE");
        });
    }

    #[test]
    #[serial]
    fn test_build_s_ado_multiple_packages_strict() {
        with_test_cache(|_temp| {
            use crate::project::{PackageEntry, PackageSource};

            let mut packages = HashMap::new();
            packages.insert(
                "estout".to_string(),
                PackageEntry {
                    version: "2024.03.15".to_string(),
                    source: PackageSource::SSC {
                        name: "estout".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );
            packages.insert(
                "reghdfe".to_string(),
                PackageEntry {
                    version: "6.12.0".to_string(),
                    source: PackageSource::SSC {
                        name: "reghdfe".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );

            let lockfile = Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages,
            };

            // Strict mode: packages + BASE only
            let s_ado = build_s_ado(&lockfile, false).unwrap();

            // Should contain paths to both packages
            assert!(s_ado.contains(&pkg_path_str("estout", "2024.03.15")));
            assert!(s_ado.contains(&pkg_path_str("reghdfe", "6.12.0")));
            // Should end with just BASE (strict mode)
            assert!(s_ado.ends_with(";BASE"));
            // Should NOT contain PLUS, PERSONAL, etc.
            assert!(!s_ado.contains("PLUS"));
            assert!(!s_ado.contains("PERSONAL"));
        });
    }

    #[test]
    #[serial]
    fn test_build_s_ado_multiple_packages_allow_global() {
        with_test_cache(|_temp| {
            use crate::project::{PackageEntry, PackageSource};

            let mut packages = HashMap::new();
            packages.insert(
                "estout".to_string(),
                PackageEntry {
                    version: "2024.03.15".to_string(),
                    source: PackageSource::SSC {
                        name: "estout".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );

            let lockfile = Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages,
            };

            // Allow global: packages + all standard paths
            let s_ado = build_s_ado(&lockfile, true).unwrap();

            assert!(s_ado.contains(&pkg_path_str("estout", "2024.03.15")));
            assert!(s_ado.ends_with(";BASE;SITE;PERSONAL;PLUS;OLDPLACE"));
        });
    }

    #[test]
    #[serial]
    fn test_list_cached_packages() {
        with_test_cache(|_temp| {
            // Create some cached packages
            let pkg1 = ensure_package_cache_dir("estout", "2024.03.15").unwrap();
            std::fs::write(pkg1.join("estout.ado"), "content").unwrap();

            let pkg2 = ensure_package_cache_dir("reghdfe", "6.12.0").unwrap();
            std::fs::write(pkg2.join("reghdfe.ado"), "content").unwrap();

            let packages = list_cached_packages().unwrap();
            assert_eq!(packages.len(), 2);

            let names: Vec<_> = packages.iter().map(|(n, _, _)| n.as_str()).collect();
            assert!(names.contains(&"estout"));
            assert!(names.contains(&"reghdfe"));
        });
    }

    #[test]
    #[serial]
    fn test_remove_cached_package() {
        with_test_cache(|_temp| {
            // Create a cached package
            let pkg_path = ensure_package_cache_dir("testpkg", "1.0.0").unwrap();
            std::fs::write(pkg_path.join("testpkg.ado"), "content").unwrap();

            assert!(is_cached("testpkg", "1.0.0").unwrap());

            // Remove it
            remove_cached_package("testpkg", "1.0.0").unwrap();

            assert!(!is_cached("testpkg", "1.0.0").unwrap());
        });
    }

    #[test]
    #[serial]
    fn test_cache_size_bytes() {
        with_test_cache(|_temp| {
            // Empty cache
            let size = cache_size_bytes().unwrap();
            assert_eq!(size, 0);

            // Add some content
            let pkg_path = ensure_package_cache_dir("testpkg", "1.0.0").unwrap();
            std::fs::write(pkg_path.join("test.ado"), "hello world").unwrap();

            let size = cache_size_bytes().unwrap();
            assert!(size > 0);
        });
    }

    #[test]
    #[serial]
    fn test_is_cached_empty_dir_returns_false() {
        with_test_cache(|_temp| {
            // Create empty package directory
            let pkg_path = ensure_package_cache_dir("emptypkg", "1.0.0").unwrap();
            assert!(pkg_path.exists());

            // Empty directory should return false (no files)
            assert!(!is_cached("emptypkg", "1.0.0").unwrap());
        });
    }

    #[test]
    #[serial]
    fn test_multiple_versions_same_package() {
        with_test_cache(|_temp| {
            // Create two versions of the same package
            let v1 = ensure_package_cache_dir("mypkg", "1.0.0").unwrap();
            std::fs::write(v1.join("mypkg.ado"), "v1 content").unwrap();

            let v2 = ensure_package_cache_dir("mypkg", "2.0.0").unwrap();
            std::fs::write(v2.join("mypkg.ado"), "v2 content").unwrap();

            // Both should be cached
            assert!(is_cached("mypkg", "1.0.0").unwrap());
            assert!(is_cached("mypkg", "2.0.0").unwrap());

            // Different paths
            let path1 = package_path("mypkg", "1.0.0").unwrap();
            let path2 = package_path("mypkg", "2.0.0").unwrap();
            assert_ne!(path1, path2);

            // List should show both
            let packages = list_cached_packages().unwrap();
            assert_eq!(packages.len(), 2);
        });
    }

    #[test]
    #[serial]
    fn test_build_s_ado_for_groups_production_only() {
        with_test_cache(|_temp| {
            use crate::project::{PackageEntry, PackageSource};

            let mut packages = HashMap::new();
            packages.insert(
                "estout".to_string(),
                PackageEntry {
                    version: "2024.03.15".to_string(),
                    source: PackageSource::SSC {
                        name: "estout".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );
            packages.insert(
                "testpkg".to_string(),
                PackageEntry {
                    version: "1.0.0".to_string(),
                    source: PackageSource::SSC {
                        name: "testpkg".to_string(),
                    },
                    checksum: None,
                    group: "dev".to_string(),
                },
            );

            let lockfile = Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages,
            };

            // Filter to production only (strict mode)
            let s_ado = build_s_ado_for_groups(&lockfile, &["production"], false).unwrap();

            assert!(s_ado.contains(&pkg_path_str("estout", "2024.03.15")));
            assert!(!s_ado.contains("testpkg")); // dev package excluded
            assert!(s_ado.ends_with(";BASE")); // strict mode - just BASE
            assert!(!s_ado.contains("PLUS"));
        });
    }

    #[test]
    #[serial]
    fn test_build_s_ado_for_groups_multiple_groups() {
        with_test_cache(|_temp| {
            use crate::project::{PackageEntry, PackageSource};

            let mut packages = HashMap::new();
            packages.insert(
                "prod_pkg".to_string(),
                PackageEntry {
                    version: "1.0.0".to_string(),
                    source: PackageSource::SSC {
                        name: "prod_pkg".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );
            packages.insert(
                "dev_pkg".to_string(),
                PackageEntry {
                    version: "1.0.0".to_string(),
                    source: PackageSource::SSC {
                        name: "dev_pkg".to_string(),
                    },
                    checksum: None,
                    group: "dev".to_string(),
                },
            );
            packages.insert(
                "test_pkg".to_string(),
                PackageEntry {
                    version: "1.0.0".to_string(),
                    source: PackageSource::SSC {
                        name: "test_pkg".to_string(),
                    },
                    checksum: None,
                    group: "test".to_string(),
                },
            );

            let lockfile = Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages,
            };

            // Filter to production and dev (allow global mode)
            let s_ado = build_s_ado_for_groups(&lockfile, &["production", "dev"], true).unwrap();

            assert!(s_ado.contains("prod_pkg"));
            assert!(s_ado.contains("dev_pkg"));
            assert!(!s_ado.contains("test_pkg")); // test excluded
            assert!(s_ado.ends_with(";BASE;SITE;PERSONAL;PLUS;OLDPLACE"));
        });
    }

    #[test]
    #[serial]
    fn test_clean_cache_removes_all() {
        with_test_cache(|_temp| {
            // Create multiple packages
            let pkg1 = ensure_package_cache_dir("pkg1", "1.0.0").unwrap();
            std::fs::write(pkg1.join("pkg1.ado"), "content").unwrap();

            let pkg2 = ensure_package_cache_dir("pkg2", "1.0.0").unwrap();
            std::fs::write(pkg2.join("pkg2.ado"), "content").unwrap();

            assert_eq!(list_cached_packages().unwrap().len(), 2);

            // Clean all
            let removed = clean_cache().unwrap();
            assert_eq!(removed, 2);

            // Cache should be empty
            assert_eq!(list_cached_packages().unwrap().len(), 0);
        });
    }

    #[test]
    #[serial]
    fn test_clean_unused_packages() {
        with_test_cache(|_temp| {
            use crate::project::{PackageEntry, PackageSource};

            // Create packages in cache
            let pkg1 = ensure_package_cache_dir("used_pkg", "1.0.0").unwrap();
            std::fs::write(pkg1.join("used_pkg.ado"), "content").unwrap();

            let pkg2 = ensure_package_cache_dir("unused_pkg", "1.0.0").unwrap();
            std::fs::write(pkg2.join("unused_pkg.ado"), "content").unwrap();

            // Create lockfile referencing only one package
            let mut packages = HashMap::new();
            packages.insert(
                "used_pkg".to_string(),
                PackageEntry {
                    version: "1.0.0".to_string(),
                    source: PackageSource::SSC {
                        name: "used_pkg".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );

            let lockfile = Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages,
            };

            // Clean unused
            let removed = clean_unused_packages(&[&lockfile]).unwrap();
            assert_eq!(removed, 1);

            // Only used package should remain
            assert!(is_cached("used_pkg", "1.0.0").unwrap());
            assert!(!is_cached("unused_pkg", "1.0.0").unwrap());
        });
    }

    #[test]
    #[serial]
    fn test_remove_package_cleans_empty_parent() {
        with_test_cache(|_temp| {
            // Create a single version of a package
            let pkg_path = ensure_package_cache_dir("lonely", "1.0.0").unwrap();
            std::fs::write(pkg_path.join("lonely.ado"), "content").unwrap();

            let parent = pkg_path.parent().unwrap();
            assert!(parent.exists());

            // Remove the package
            remove_cached_package("lonely", "1.0.0").unwrap();

            // Parent directory should be cleaned up too
            assert!(!parent.exists());
        });
    }

    #[test]
    #[serial]
    fn test_remove_package_preserves_other_versions() {
        with_test_cache(|_temp| {
            // Create two versions
            let v1 = ensure_package_cache_dir("mypkg", "1.0.0").unwrap();
            std::fs::write(v1.join("mypkg.ado"), "v1").unwrap();

            let v2 = ensure_package_cache_dir("mypkg", "2.0.0").unwrap();
            std::fs::write(v2.join("mypkg.ado"), "v2").unwrap();

            // Remove only v1
            remove_cached_package("mypkg", "1.0.0").unwrap();

            // v2 should still exist
            assert!(!is_cached("mypkg", "1.0.0").unwrap());
            assert!(is_cached("mypkg", "2.0.0").unwrap());

            // Parent directory should still exist (has v2)
            let parent = package_path("mypkg", "2.0.0")
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf();
            assert!(parent.exists());
        });
    }

    #[test]
    #[serial]
    fn test_package_path_with_special_version() {
        with_test_cache(|_temp| {
            // Versions with dots and dashes
            let path1 = package_path("pkg", "1.0.0-beta").unwrap();
            assert!(path1.ends_with("packages/pkg/1.0.0-beta"));

            let path2 = package_path("pkg", "2024.03.15").unwrap();
            assert!(path2.ends_with("packages/pkg/2024.03.15"));

            // Version with only numbers
            let path3 = package_path("pkg", "20240315").unwrap();
            assert!(path3.ends_with("packages/pkg/20240315"));
        });
    }

    #[test]
    #[serial]
    fn test_build_s_ado_deterministic_order() {
        with_test_cache(|_temp| {
            use crate::project::{PackageEntry, PackageSource};

            let mut packages = HashMap::new();
            packages.insert(
                "zebra".to_string(),
                PackageEntry {
                    version: "1.0.0".to_string(),
                    source: PackageSource::SSC {
                        name: "zebra".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );
            packages.insert(
                "alpha".to_string(),
                PackageEntry {
                    version: "2.0.0".to_string(),
                    source: PackageSource::SSC {
                        name: "alpha".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );
            packages.insert(
                "middle".to_string(),
                PackageEntry {
                    version: "3.0.0".to_string(),
                    source: PackageSource::SSC {
                        name: "middle".to_string(),
                    },
                    checksum: None,
                    group: "production".to_string(),
                },
            );

            let lockfile = Lockfile {
                version: "1".to_string(),
                stacy_version: None,
                packages,
            };

            // Call multiple times — all outputs must be identical
            let first = build_s_ado(&lockfile, false).unwrap();
            for _ in 0..10 {
                assert_eq!(build_s_ado(&lockfile, false).unwrap(), first);
            }

            // Packages must appear in alphabetical order (alpha, middle, zebra)
            let alpha_pos = first.find("alpha").unwrap();
            let middle_pos = first.find("middle").unwrap();
            let zebra_pos = first.find("zebra").unwrap();
            assert!(
                alpha_pos < middle_pos && middle_pos < zebra_pos,
                "Packages should be in alphabetical order: {}",
                first
            );
        });
    }

    #[test]
    #[serial]
    fn test_list_cached_packages_sorted() {
        with_test_cache(|_temp| {
            // Create packages in non-alphabetical order
            let c = ensure_package_cache_dir("cpkg", "1.0.0").unwrap();
            std::fs::write(c.join("c.ado"), "c").unwrap();

            let a = ensure_package_cache_dir("apkg", "1.0.0").unwrap();
            std::fs::write(a.join("a.ado"), "a").unwrap();

            let b = ensure_package_cache_dir("bpkg", "1.0.0").unwrap();
            std::fs::write(b.join("b.ado"), "b").unwrap();

            let packages = list_cached_packages().unwrap();
            let names: Vec<_> = packages.iter().map(|(n, _, _)| n.as_str()).collect();

            // Should be sorted alphabetically
            assert_eq!(names, vec!["apkg", "bpkg", "cpkg"]);
        });
    }

    #[test]
    #[serial]
    fn test_cache_size_multiple_files() {
        with_test_cache(|_temp| {
            let pkg_path = ensure_package_cache_dir("bigpkg", "1.0.0").unwrap();

            // Write multiple files
            std::fs::write(pkg_path.join("main.ado"), "x".repeat(1000)).unwrap();
            std::fs::write(pkg_path.join("helper.ado"), "y".repeat(500)).unwrap();
            std::fs::write(pkg_path.join("docs.sthlp"), "z".repeat(200)).unwrap();

            let size = cache_size_bytes().unwrap();
            // Should be at least the sum of file sizes
            assert!(size >= 1700);
        });
    }

    #[test]
    #[serial]
    fn test_ensure_package_cache_dir_idempotent() {
        with_test_cache(|_temp| {
            let path1 = ensure_package_cache_dir("pkg", "1.0.0").unwrap();
            std::fs::write(path1.join("file.ado"), "content").unwrap();

            // Call again - should not fail or remove existing content
            let path2 = ensure_package_cache_dir("pkg", "1.0.0").unwrap();

            assert_eq!(path1, path2);
            assert!(path2.join("file.ado").exists());
        });
    }

    #[test]
    #[serial]
    fn test_remove_nonexistent_package_succeeds() {
        with_test_cache(|_temp| {
            // Should not error when removing a package that doesn't exist
            let result = remove_cached_package("nonexistent", "1.0.0");
            assert!(result.is_ok());
        });
    }
}
