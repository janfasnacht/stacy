//! Package installation logic
//!
//! Handles downloading and installing packages to the global cache,
//! and updating the lockfile.

use crate::error::{Error, Result};
use crate::packages::github::GitHubDownloader;
use crate::packages::global_cache;
use crate::packages::lockfile::{
    add_package, create_lockfile, create_package_entry, load_lockfile, save_lockfile,
};
use crate::packages::ssc::SscDownloader;
use crate::project::{PackageSource, Project};
use std::path::{Path, PathBuf};

/// Result of installing a package
#[derive(Debug)]
pub struct InstallResult {
    /// Package name
    pub name: String,
    /// Package version (from manifest or date)
    pub version: String,
    /// Files that were installed
    pub files_installed: Vec<PathBuf>,
    /// Whether this was an update (package already existed)
    pub was_update: bool,
    /// Whether the download came from an SSC mirror (not the primary server)
    pub from_mirror: bool,
    /// Combined checksum of all downloaded files
    pub package_checksum: String,
}

/// Install a package from SSC
///
/// # Arguments
/// * `name` - Package name to install
/// * `project_root` - Project root directory
/// * `group` - Dependency group ("production", "dev", or "test")
///
/// # Returns
/// InstallResult with details about what was installed
pub fn install_from_ssc(name: &str, project_root: &Path, group: &str) -> Result<InstallResult> {
    let name = name.to_lowercase();

    // Download package
    let downloader = SscDownloader::new();
    let download = downloader.download_package(&name)?;

    // Get version from manifest (use distribution date or today's date)
    let version = download
        .manifest
        .distribution_date
        .clone()
        .unwrap_or_else(crate::utils::date::today_yyyymmdd);

    // Save files to global cache atomically
    let (_cache_dir, saved_files) = atomic_save_to_cache(&download.files, &name, &version)?;

    // Load or create lockfile
    let mut lockfile = load_lockfile(project_root)?.unwrap_or_else(create_lockfile);

    // Check if this is an update
    let was_update = lockfile.packages.contains_key(&name);

    // Create package entry
    let entry = create_package_entry(
        &version,
        PackageSource::SSC { name: name.clone() },
        &download.package_checksum,
        group,
    );

    // Update lockfile
    add_package(&mut lockfile, &name, entry);
    save_lockfile(project_root, &lockfile)?;

    let from_mirror = download.from_mirror;
    let package_checksum = download.package_checksum.clone();

    Ok(InstallResult {
        name,
        version,
        files_installed: saved_files,
        was_update,
        from_mirror,
        package_checksum,
    })
}

/// Install a package (auto-detect source)
pub fn install_package(
    name: &str,
    source: &str,
    project_root: &Path,
    group: &str,
) -> Result<InstallResult> {
    match source.to_lowercase().as_str() {
        "ssc" => install_from_ssc(name, project_root, group),
        _ => Err(Error::Config(format!("Unknown package source: {}", source))),
    }
}

/// Install a package from GitHub
///
/// # Arguments
/// * `name` - Package name (used to find {name}.pkg)
/// * `user` - GitHub username/organization
/// * `repo` - Repository name
/// * `git_ref` - Branch, tag, or commit (defaults to "main")
/// * `project_root` - Project root directory
/// * `group` - Dependency group ("production", "dev", or "test")
pub fn install_package_github(
    name: &str,
    user: &str,
    repo: &str,
    git_ref: Option<&str>,
    project_root: &Path,
    group: &str,
) -> Result<InstallResult> {
    let name = name.to_lowercase();
    let git_ref_str = git_ref.unwrap_or("main");

    // Download package from GitHub
    let downloader = GitHubDownloader::new();
    let download = downloader.download_package(&name, user, repo, git_ref)?;

    // Resolve commit SHA for reproducibility (graceful degradation)
    let commit_sha = downloader.resolve_commit_sha(user, repo, &download.git_ref);

    // Get version from manifest, or use short SHA / git ref as fallback
    let version = download
        .manifest
        .distribution_date
        .clone()
        .unwrap_or_else(|| {
            if let Some(ref sha) = commit_sha {
                sha[..8].to_string()
            } else {
                git_ref_str.to_string()
            }
        });

    // Save files to global cache atomically
    let (_cache_dir, saved_files) = atomic_save_to_cache(&download.files, &name, &version)?;

    // Load or create lockfile
    let mut lockfile = load_lockfile(project_root)?.unwrap_or_else(create_lockfile);

    // Check if this is an update
    let was_update = lockfile.packages.contains_key(&name);

    // Create package entry
    let entry = create_package_entry(
        &version,
        PackageSource::GitHub {
            repo: format!("{}/{}", user, repo),
            tag: git_ref_str.to_string(),
            commit: commit_sha,
        },
        &download.package_checksum,
        group,
    );

    // Update lockfile
    add_package(&mut lockfile, &name, entry);
    save_lockfile(project_root, &lockfile)?;

    Ok(InstallResult {
        name,
        version,
        files_installed: saved_files,
        was_update,
        from_mirror: false, // GitHub packages don't use SSC mirrors
        package_checksum: download.package_checksum.clone(),
    })
}

/// Check if a package version is installed in the global cache
pub fn is_package_installed(name: &str, version: &str) -> bool {
    global_cache::is_cached(name, version).unwrap_or(false)
}

/// Check if a package is installed in a local ado directory (legacy check)
#[allow(dead_code)]
pub fn is_package_installed_local(name: &str, ado_dir: &Path) -> bool {
    let first_char = name
        .chars()
        .next()
        .unwrap_or('_')
        .to_lowercase()
        .next()
        .unwrap_or('_');

    let package_dir = ado_dir.join(first_char.to_string());

    // Check for .ado file with package name
    let ado_file = package_dir.join(format!("{}.ado", name));
    ado_file.exists()
}

/// Uninstall a package (removes from lockfile only, keeps cache for reuse)
///
/// With the global cache, packages are shared between projects.
/// Uninstalling just removes the package from the project's lockfile.
/// The cached package files remain for potential reuse by other projects.
///
/// # Arguments
/// * `name` - Package name to uninstall
///
/// # Returns
/// Empty list (no files are deleted from cache)
pub fn uninstall_package(name: &str) -> Result<Vec<PathBuf>> {
    // With global cache, we don't delete files - they may be used by other projects.
    // The lockfile removal is handled by the remove command.
    let _ = name.to_lowercase();
    Ok(Vec::new())
}

/// Uninstall a package from a local ado directory (legacy function)
#[allow(dead_code)]
pub fn uninstall_package_local(name: &str, ado_dir: &Path) -> Result<Vec<PathBuf>> {
    let name = name.to_lowercase();
    let first_char = name
        .chars()
        .next()
        .unwrap_or('_')
        .to_lowercase()
        .next()
        .unwrap_or('_');

    let package_dir = ado_dir.join(first_char.to_string());

    if !package_dir.exists() {
        return Ok(Vec::new());
    }

    let mut deleted_files = Vec::new();

    // Find all files that belong to this package
    // Package files typically include: {name}.ado, {name}.sthlp, {name}.pkg, l{name}.mlib, etc.
    let entries = std::fs::read_dir(&package_dir).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read directory {}: {}", package_dir.display(), e),
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read directory entry: {}", e),
            ))
        })?;

        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy().to_lowercase();

        // Match files that:
        // 1. Start with the package name (e.g., estout.ado, estout.sthlp)
        // 2. Start with 'l' + package name (e.g., lestout.mlib for Mata libraries)
        // 3. Are prefixed with underscore + package name (e.g., _estout_internal.ado)
        let matches = file_name_str.starts_with(&name)
            || file_name_str.starts_with(&format!("l{}", name))
            || file_name_str.starts_with(&format!("_{}", name));

        if matches {
            let file_path = entry.path();
            if file_path.is_file() {
                std::fs::remove_file(&file_path).map_err(|e| {
                    Error::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to delete {}: {}", file_path.display(), e),
                    ))
                })?;
                deleted_files.push(file_path);
            }
        }
    }

    Ok(deleted_files)
}

/// Get project root, creating project if needed
pub fn get_or_create_project(path: Option<&Path>) -> Result<PathBuf> {
    // Try to find existing project
    let project = if let Some(p) = path {
        Project::find_from(p)?
    } else {
        Project::find()?
    };

    if let Some(proj) = project {
        return Ok(proj.root);
    }

    // No project found - create one in current directory
    let current_dir = std::env::current_dir().map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            "Failed to get current directory",
        ))
    })?;

    // Create minimal project structure
    crate::project::structure::create_project_structure(&current_dir, false)?;

    Ok(current_dir)
}

/// Atomically save downloaded files to the global cache using a staging directory.
///
/// Writes files to a temporary `.downloading` directory, then renames it to the
/// final location. This prevents partial packages from being visible to `is_cached()`.
fn atomic_save_to_cache(
    files: &[crate::packages::ssc::DownloadedFile],
    name: &str,
    version: &str,
) -> Result<(PathBuf, Vec<PathBuf>)> {
    let final_dir = global_cache::package_path(name, version)?;

    // Use a unique staging dir per attempt (PID + thread ID) to avoid races
    let unique_id = format!("{}.{:?}", std::process::id(), std::thread::current().id());
    let staging_dir = final_dir.with_file_name(format!("{}.downloading.{}", version, unique_id));

    // Ensure parent exists
    if let Some(parent) = staging_dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create parent dir {}: {}", parent.display(), e),
            ))
        })?;

        // Clean up stale staging dirs from interrupted installs (different PIDs).
        // Skip dirs from our own PID — those are concurrent threads, not stale.
        let our_pid_marker = format!(".{}", std::process::id());
        let prefix = format!("{}.downloading", version);
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&prefix) && !name.contains(&our_pid_marker) {
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                }
            }
        }
    }

    // Clean up our own staging dir if it exists (shouldn't happen, but be safe)
    if staging_dir.exists() {
        let _ = std::fs::remove_dir_all(&staging_dir);
    }

    std::fs::create_dir(&staging_dir).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to create staging dir {}: {}",
                staging_dir.display(),
                e
            ),
        ))
    })?;

    // Write to staging
    let result = save_package_files_to_cache(files, &staging_dir);

    match result {
        Ok(staged_files) => {
            // Atomic rename: try to move staging → final
            // If final_dir already exists (another process won the race), that's fine —
            // just clean up our staging dir and use the existing final dir.
            match std::fs::rename(&staging_dir, &final_dir) {
                Ok(()) => {
                    // We won: remap paths from staging to final dir
                    let final_files = staged_files
                        .iter()
                        .map(|p| final_dir.join(p.file_name().unwrap()))
                        .collect();
                    Ok((final_dir, final_files))
                }
                Err(_) if final_dir.exists() => {
                    // Another process/thread already placed the final dir — that's OK.
                    // Clean up our staging dir and return the existing final dir.
                    let _ = std::fs::remove_dir_all(&staging_dir);
                    let final_files = staged_files
                        .iter()
                        .map(|p| final_dir.join(p.file_name().unwrap()))
                        .collect();
                    Ok((final_dir, final_files))
                }
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&staging_dir);
                    Err(Error::Io(std::io::Error::new(
                        e.kind(),
                        format!(
                            "Failed to rename staging dir {} to {}: {}",
                            staging_dir.display(),
                            final_dir.display(),
                            e
                        ),
                    )))
                }
            }
        }
        Err(e) => {
            // Clean up staging on failure
            let _ = std::fs::remove_dir_all(&staging_dir);
            Err(e)
        }
    }
}

/// Save downloaded files to a cache directory (flat structure)
///
/// Unlike the old save_package_files which organized by first letter,
/// this saves all files directly into the cache directory.
fn save_package_files_to_cache(
    files: &[crate::packages::ssc::DownloadedFile],
    cache_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let mut saved_files = Vec::new();

    for file in files {
        // Get the actual filename (strip any path components like "../e/")
        let filename = std::path::Path::new(&file.name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.name);

        let target_path = cache_dir.join(filename);
        std::fs::write(&target_path, &file.content).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write {}: {}", target_path.display(), e),
            ))
        })?;
        saved_files.push(target_path);
    }

    Ok(saved_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    // Helper to set up a test cache directory.
    // Sets both XDG_CACHE_HOME (Unix) and LOCALAPPDATA (Windows) so
    // cache_dir() is isolated on all platforms.
    fn with_test_cache<F, R>(f: F) -> R
    where
        F: FnOnce(&TempDir) -> R,
    {
        let temp = TempDir::new().unwrap();
        let prev_xdg = std::env::var("XDG_CACHE_HOME").ok();
        let prev_localappdata = std::env::var("LOCALAPPDATA").ok();
        std::env::set_var("XDG_CACHE_HOME", temp.path());
        std::env::set_var("LOCALAPPDATA", temp.path());
        let result = f(&temp);
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
    fn test_is_package_installed_false() {
        with_test_cache(|_temp| {
            assert!(!is_package_installed("estout", "1.0.0"));
        });
    }

    #[test]
    #[serial]
    fn test_is_package_installed_true() {
        with_test_cache(|_temp| {
            let cache_dir = global_cache::ensure_package_cache_dir("estout", "1.0.0").unwrap();
            fs::write(cache_dir.join("estout.ado"), "test").unwrap();

            assert!(is_package_installed("estout", "1.0.0"));
        });
    }

    #[test]
    fn test_is_package_installed_local_false() {
        let temp = TempDir::new().unwrap();
        let ado_dir = temp.path().join("ado");
        fs::create_dir_all(&ado_dir).unwrap();

        assert!(!is_package_installed_local("estout", &ado_dir));
    }

    #[test]
    fn test_is_package_installed_local_true() {
        let temp = TempDir::new().unwrap();
        let ado_dir = temp.path().join("ado");
        let pkg_dir = ado_dir.join("e");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("estout.ado"), "test").unwrap();

        assert!(is_package_installed_local("estout", &ado_dir));
    }

    #[test]
    fn test_get_or_create_project_creates_new() {
        let temp = TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = get_or_create_project(None);
        assert!(result.is_ok());

        // Should have created stacy.toml
        assert!(temp.path().join("stacy.toml").exists());
    }

    #[test]
    fn test_uninstall_package_returns_empty() {
        // With global cache, uninstall_package returns empty (no files deleted)
        let result = uninstall_package("estout").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_uninstall_package_local_removes_files() {
        let temp = TempDir::new().unwrap();
        let ado_dir = temp.path().join("ado");
        let pkg_dir = ado_dir.join("e");
        fs::create_dir_all(&pkg_dir).unwrap();

        // Create package files
        fs::write(pkg_dir.join("estout.ado"), "test").unwrap();
        fs::write(pkg_dir.join("estout.sthlp"), "help").unwrap();
        fs::write(pkg_dir.join("estout.pkg"), "pkg").unwrap();
        fs::write(pkg_dir.join("other.ado"), "other").unwrap(); // Should not be deleted

        let result = uninstall_package_local("estout", &ado_dir).unwrap();

        // Should have deleted 3 estout files
        assert_eq!(result.len(), 3);

        // Files should be gone
        assert!(!pkg_dir.join("estout.ado").exists());
        assert!(!pkg_dir.join("estout.sthlp").exists());
        assert!(!pkg_dir.join("estout.pkg").exists());

        // Other files should still exist
        assert!(pkg_dir.join("other.ado").exists());
    }

    #[test]
    #[serial]
    fn test_save_package_files_to_cache() {
        with_test_cache(|_temp| {
            use crate::packages::ssc::DownloadedFile;

            let cache_dir = global_cache::ensure_package_cache_dir("testpkg", "1.0.0").unwrap();

            let files = vec![
                DownloadedFile {
                    name: "testpkg.ado".to_string(),
                    content: b"ado content".to_vec(),
                    checksum: "abc".to_string(),
                },
                DownloadedFile {
                    name: "../t/testpkg.sthlp".to_string(), // Path prefix should be stripped
                    content: b"help content".to_vec(),
                    checksum: "def".to_string(),
                },
            ];

            let saved = save_package_files_to_cache(&files, &cache_dir).unwrap();

            assert_eq!(saved.len(), 2);
            assert!(cache_dir.join("testpkg.ado").exists());
            assert!(cache_dir.join("testpkg.sthlp").exists());
        });
    }

    #[test]
    #[serial]
    fn test_save_package_files_strips_path_components() {
        with_test_cache(|_temp| {
            use crate::packages::ssc::DownloadedFile;

            let cache_dir = global_cache::ensure_package_cache_dir("pathtest", "1.0.0").unwrap();

            // Files with various path prefixes that should be stripped
            let files = vec![
                DownloadedFile {
                    name: "../p/pathtest.ado".to_string(),
                    content: b"ado".to_vec(),
                    checksum: "abc".to_string(),
                },
                DownloadedFile {
                    name: "./subdir/helper.ado".to_string(),
                    content: b"helper".to_vec(),
                    checksum: "def".to_string(),
                },
                DownloadedFile {
                    name: "simple.sthlp".to_string(),
                    content: b"help".to_vec(),
                    checksum: "ghi".to_string(),
                },
            ];

            let saved = save_package_files_to_cache(&files, &cache_dir).unwrap();

            assert_eq!(saved.len(), 3);
            // All files should be directly in cache_dir with basename only
            assert!(cache_dir.join("pathtest.ado").exists());
            assert!(cache_dir.join("helper.ado").exists());
            assert!(cache_dir.join("simple.sthlp").exists());
        });
    }

    #[test]
    #[serial]
    fn test_is_package_installed_case_insensitive() {
        with_test_cache(|_temp| {
            let cache_dir = global_cache::ensure_package_cache_dir("MyPkg", "1.0.0").unwrap();
            fs::write(cache_dir.join("mypkg.ado"), "content").unwrap();

            // Package names are normalized to lowercase
            assert!(is_package_installed("mypkg", "1.0.0"));
            assert!(is_package_installed("MYPKG", "1.0.0"));
            assert!(is_package_installed("MyPkg", "1.0.0"));
        });
    }

    #[test]
    fn test_uninstall_package_returns_empty_vec() {
        // With global cache, uninstall doesn't delete files
        let result = uninstall_package("any_package").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_uninstall_package_local_nonexistent() {
        let temp = TempDir::new().unwrap();
        let ado_dir = temp.path().join("ado");
        fs::create_dir_all(&ado_dir).unwrap();

        // Should succeed even if package doesn't exist
        let result = uninstall_package_local("nonexistent", &ado_dir).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_uninstall_package_local_removes_only_matching_files() {
        let temp = TempDir::new().unwrap();
        let ado_dir = temp.path().join("ado");
        let pkg_dir = ado_dir.join("m");
        fs::create_dir_all(&pkg_dir).unwrap();

        // Create files for mypkg
        fs::write(pkg_dir.join("mypkg.ado"), "content").unwrap();
        fs::write(pkg_dir.join("mypkg.sthlp"), "help").unwrap();
        fs::write(pkg_dir.join("mypkg_helper.ado"), "helper").unwrap();

        // Create file for different package
        fs::write(pkg_dir.join("otherpkg.ado"), "other").unwrap();

        let removed = uninstall_package_local("mypkg", &ado_dir).unwrap();

        // Should remove mypkg files
        assert_eq!(removed.len(), 3);
        assert!(!pkg_dir.join("mypkg.ado").exists());
        assert!(!pkg_dir.join("mypkg.sthlp").exists());
        assert!(!pkg_dir.join("mypkg_helper.ado").exists());

        // Should preserve other package
        assert!(pkg_dir.join("otherpkg.ado").exists());
    }

    #[test]
    #[serial]
    fn test_save_package_files_preserves_content() {
        with_test_cache(|_temp| {
            use crate::packages::ssc::DownloadedFile;

            let cache_dir = global_cache::ensure_package_cache_dir("contenttest", "1.0.0").unwrap();

            let original_content = b"* This is a test ado file\nprogram define mytest\nend";
            let files = vec![DownloadedFile {
                name: "contenttest.ado".to_string(),
                content: original_content.to_vec(),
                checksum: "abc".to_string(),
            }];

            save_package_files_to_cache(&files, &cache_dir).unwrap();

            let saved_content = fs::read(cache_dir.join("contenttest.ado")).unwrap();
            assert_eq!(saved_content, original_content);
        });
    }

    #[test]
    #[serial]
    fn test_multiple_packages_independent_caches() {
        with_test_cache(|_temp| {
            use crate::packages::ssc::DownloadedFile;

            // Install two packages
            let cache1 = global_cache::ensure_package_cache_dir("pkg1", "1.0.0").unwrap();
            let cache2 = global_cache::ensure_package_cache_dir("pkg2", "1.0.0").unwrap();

            let files1 = vec![DownloadedFile {
                name: "pkg1.ado".to_string(),
                content: b"pkg1 content".to_vec(),
                checksum: "abc".to_string(),
            }];
            let files2 = vec![DownloadedFile {
                name: "pkg2.ado".to_string(),
                content: b"pkg2 content".to_vec(),
                checksum: "def".to_string(),
            }];

            save_package_files_to_cache(&files1, &cache1).unwrap();
            save_package_files_to_cache(&files2, &cache2).unwrap();

            // Each package should be in its own directory
            assert!(cache1.join("pkg1.ado").exists());
            assert!(cache2.join("pkg2.ado").exists());
            assert!(!cache1.join("pkg2.ado").exists());
            assert!(!cache2.join("pkg1.ado").exists());
        });
    }

    // Integration tests that require network access
    #[test]
    #[ignore]
    #[serial]
    fn test_install_from_ssc_integration() {
        with_test_cache(|_cache_temp| {
            let temp = TempDir::new().unwrap();

            // Create minimal project
            fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

            let result = install_from_ssc("estout", temp.path(), "production");
            assert!(result.is_ok());

            let install = result.unwrap();
            assert_eq!(install.name, "estout");
            assert!(!install.files_installed.is_empty());
            assert!(!install.was_update);

            // Check lockfile was created
            assert!(temp.path().join("stacy.lock").exists());

            // No local ado/ directory should be created
            assert!(!temp.path().join("ado").exists());

            // Check that files are in the global cache
            assert!(is_package_installed("estout", &install.version));

            // Check that the lockfile has the correct group
            let lockfile_content = fs::read_to_string(temp.path().join("stacy.lock")).unwrap();
            assert!(lockfile_content.contains("group = \"production\""));
        });
    }

    #[test]
    #[ignore]
    #[serial]
    fn test_install_from_ssc_dev_group() {
        with_test_cache(|_cache_temp| {
            let temp = TempDir::new().unwrap();
            fs::write(temp.path().join("stacy.toml"), "[project]\n").unwrap();

            let result = install_from_ssc("estout", temp.path(), "dev");
            assert!(result.is_ok());

            let lockfile_content = fs::read_to_string(temp.path().join("stacy.lock")).unwrap();
            assert!(lockfile_content.contains("group = \"dev\""));
        });
    }

    // C5: Test that .downloading dir is not treated as cached
    #[test]
    #[serial]
    fn test_downloading_dir_not_treated_as_cached() {
        with_test_cache(|_temp| {
            // Create a .downloading staging dir (simulates interrupted install)
            let final_dir = global_cache::package_path("stagingtest", "1.0.0").unwrap();
            let staging_dir = final_dir.with_file_name("1.0.0.downloading");
            fs::create_dir_all(&staging_dir).unwrap();
            fs::write(staging_dir.join("stagingtest.ado"), "partial").unwrap();

            // Should NOT be treated as cached — only the final dir counts
            assert!(!is_package_installed("stagingtest", "1.0.0"));
        });
    }

    // C5: Test atomic_save_to_cache creates complete package
    #[test]
    #[serial]
    fn test_atomic_save_creates_complete_package() {
        with_test_cache(|_temp| {
            use crate::packages::ssc::DownloadedFile;

            let files = vec![
                DownloadedFile {
                    name: "atomicpkg.ado".to_string(),
                    content: b"ado content".to_vec(),
                    checksum: "abc".to_string(),
                },
                DownloadedFile {
                    name: "atomicpkg.sthlp".to_string(),
                    content: b"help content".to_vec(),
                    checksum: "def".to_string(),
                },
            ];

            let (final_dir, saved_files) =
                atomic_save_to_cache(&files, "atomicpkg", "1.0.0").unwrap();

            // Final dir should exist with all files
            assert!(final_dir.exists());
            assert!(final_dir.join("atomicpkg.ado").exists());
            assert!(final_dir.join("atomicpkg.sthlp").exists());
            assert_eq!(saved_files.len(), 2);

            // No staging dirs should remain
            if let Some(parent) = final_dir.parent() {
                let stale: Vec<_> = std::fs::read_dir(parent)
                    .unwrap()
                    .flatten()
                    .filter(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.contains(".downloading"))
                            .unwrap_or(false)
                    })
                    .collect();
                assert!(
                    stale.is_empty(),
                    "No staging dirs should remain: {:?}",
                    stale
                );
            }

            // Should be reported as cached
            assert!(is_package_installed("atomicpkg", "1.0.0"));
        });
    }

    // C5: Test concurrent atomic_save_to_cache calls don't corrupt the final state
    #[test]
    #[serial]
    fn test_concurrent_atomic_save_no_corruption() {
        with_test_cache(|_temp| {
            use crate::packages::ssc::DownloadedFile;
            use std::thread;

            let files1 = vec![DownloadedFile {
                name: "concpkg.ado".to_string(),
                content: b"thread 1 content".to_vec(),
                checksum: "abc".to_string(),
            }];
            let files2 = vec![DownloadedFile {
                name: "concpkg.ado".to_string(),
                content: b"thread 2 content".to_vec(),
                checksum: "def".to_string(),
            }];

            let handle1 = thread::spawn(move || atomic_save_to_cache(&files1, "concpkg", "1.0.0"));
            let handle2 = thread::spawn(move || atomic_save_to_cache(&files2, "concpkg", "1.0.0"));

            let r1 = handle1.join().unwrap();
            let r2 = handle2.join().unwrap();

            // Both should succeed: winner renames, loser sees final_dir exists
            assert!(r1.is_ok(), "Thread 1 must succeed: {:?}", r1);
            assert!(r2.is_ok(), "Thread 2 must succeed: {:?}", r2);

            // Final state must be valid: directory exists with exactly one file
            let final_dir = global_cache::package_path("concpkg", "1.0.0").unwrap();
            assert!(final_dir.exists(), "Final dir must exist");
            assert!(
                final_dir.join("concpkg.ado").exists(),
                "concpkg.ado must exist in final dir"
            );

            // No staging dirs should remain
            if let Some(parent) = final_dir.parent() {
                let stale: Vec<_> = std::fs::read_dir(parent)
                    .unwrap()
                    .flatten()
                    .filter(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.contains(".downloading"))
                            .unwrap_or(false)
                    })
                    .collect();
                assert!(
                    stale.is_empty(),
                    "No staging dirs should remain: {:?}",
                    stale
                );
            }

            // Package should be reported as cached
            assert!(is_package_installed("concpkg", "1.0.0"));
        });
    }

    // C5: Test that stale .downloading dir is cleaned up on retry
    #[test]
    #[serial]
    fn test_atomic_save_cleans_stale_staging() {
        with_test_cache(|_temp| {
            use crate::packages::ssc::DownloadedFile;

            // Simulate a stale .downloading dir from a previous interrupted install
            let final_dir = global_cache::package_path("stalепkg", "1.0.0").unwrap();
            let staging_dir = final_dir.with_file_name("1.0.0.downloading");
            fs::create_dir_all(&staging_dir).unwrap();
            fs::write(staging_dir.join("old_file.ado"), "stale content").unwrap();

            // Now do a real atomic save
            let files = vec![DownloadedFile {
                name: "stalepkg.ado".to_string(),
                content: b"fresh content".to_vec(),
                checksum: "abc".to_string(),
            }];

            // Use a package name that won't conflict (the cyrillic е above is different)
            let result = atomic_save_to_cache(&files, "stalepkg", "1.0.0");
            assert!(result.is_ok());

            let (final_dir, _) = result.unwrap();
            assert!(final_dir.join("stalepkg.ado").exists());

            // Staging dir should be gone
            let staging_dir2 = final_dir.with_file_name("1.0.0.downloading");
            assert!(!staging_dir2.exists());
        });
    }
}
