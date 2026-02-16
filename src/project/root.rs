//! Project root detection
//!
//! Walks up the directory tree looking for project markers:
//! 1. `stacy.toml` (explicit config - highest precedence)
//! 2. `stacy.lock` (lockfile marker)

use crate::error::Result;
use std::path::{Path, PathBuf};

/// Markers that indicate a project root, in precedence order.
/// The first marker found determines the project root.
///
/// Note: `ado/` was removed as a marker since packages are now stored
/// in a global cache at `~/.cache/stacy/packages/`.
const PROJECT_MARKERS: &[&str] = &["stacy.toml", "stacy.lock"];

/// Information about a detected project root
#[derive(Debug, Clone)]
pub struct ProjectRoot {
    /// The path to the project root directory
    pub path: PathBuf,
    /// The marker that was found (stacy.toml, stacy.lock, or ado)
    pub marker: String,
}

/// Find project root by walking up from the given directory.
///
/// Checks each directory for project markers in precedence order:
/// 1. `stacy.toml` - explicit project configuration
/// 2. `stacy.lock` - package lockfile
///
/// Returns the first directory containing any marker, or None if no project found.
///
/// # Arguments
/// * `start_dir` - The directory to start searching from
///
/// # Examples
/// ```ignore
/// use std::path::Path;
/// use stacy::project::root::find_project_root;
///
/// let root = find_project_root(Path::new("/home/user/project/subdir"))?;
/// if let Some(root) = root {
///     println!("Project root: {}", root.path.display());
/// }
/// ```
pub fn find_project_root(start_dir: &Path) -> Result<Option<ProjectRoot>> {
    // Canonicalize the start directory to resolve symlinks and get absolute path
    let mut current = start_dir
        .canonicalize()
        .unwrap_or_else(|_| start_dir.to_path_buf());

    loop {
        // Check for project markers in precedence order
        if let Some(marker) = find_marker_in_dir(&current) {
            return Ok(Some(ProjectRoot {
                path: current,
                marker: marker.to_string(),
            }));
        }

        // Move up to parent directory
        match current.parent() {
            Some(parent) => {
                // Check if we've reached the root (parent is same as current)
                if parent == current {
                    break;
                }
                current = parent.to_path_buf();
            }
            None => break,
        }
    }

    Ok(None)
}

/// Find project root starting from the current working directory.
///
/// Convenience wrapper around `find_project_root` that uses `std::env::current_dir()`.
pub fn find_project_root_from_cwd() -> Result<Option<ProjectRoot>> {
    let cwd = std::env::current_dir()?;
    find_project_root(&cwd)
}

/// Check if a directory contains any project marker.
/// Returns the first marker found (in precedence order), or None.
fn find_marker_in_dir(dir: &Path) -> Option<&'static str> {
    for marker in PROJECT_MARKERS {
        let marker_path = dir.join(marker);
        if marker_path.exists() {
            return Some(marker);
        }
    }
    None
}

/// Check if a directory is a project root (has any marker).
pub fn is_project_root(dir: &Path) -> bool {
    find_marker_in_dir(dir).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_root_with_stacy_toml() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("stacy.toml"), "# config").unwrap();

        let result = find_project_root(temp.path()).unwrap();
        assert!(result.is_some());
        let root = result.unwrap();
        assert_eq!(root.path, temp.path().canonicalize().unwrap());
        assert_eq!(root.marker, "stacy.toml");
    }

    #[test]
    fn test_find_root_with_stacy_lock() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("stacy.lock"), "version = \"1\"").unwrap();

        let result = find_project_root(temp.path()).unwrap();
        assert!(result.is_some());
        let root = result.unwrap();
        assert_eq!(root.marker, "stacy.lock");
    }

    #[test]
    fn test_find_root_precedence() {
        // stacy.toml should take precedence over stacy.lock
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("stacy.toml"), "").unwrap();
        fs::write(temp.path().join("stacy.lock"), "").unwrap();

        let result = find_project_root(temp.path()).unwrap();
        assert!(result.is_some());
        let root = result.unwrap();
        assert_eq!(root.marker, "stacy.toml");
    }

    #[test]
    fn test_find_root_walk_up() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("sub1").join("sub2").join("sub3");
        fs::create_dir_all(&nested).unwrap();
        fs::write(temp.path().join("stacy.toml"), "").unwrap();

        let result = find_project_root(&nested).unwrap();
        assert!(result.is_some());
        let root = result.unwrap();
        assert_eq!(root.path, temp.path().canonicalize().unwrap());
    }

    #[test]
    fn test_no_project_found() {
        let temp = TempDir::new().unwrap();
        // No markers - should return None

        let result = find_project_root(temp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_nested_projects_inner_wins() {
        // Inner project should be found first (stops at first marker)
        let temp = TempDir::new().unwrap();
        let inner = temp.path().join("inner");
        fs::create_dir(&inner).unwrap();

        // Outer project marker
        fs::write(temp.path().join("stacy.toml"), "# outer").unwrap();
        // Inner project marker
        fs::write(inner.join("stacy.toml"), "# inner").unwrap();

        let result = find_project_root(&inner).unwrap();
        assert!(result.is_some());
        let root = result.unwrap();
        assert_eq!(root.path, inner.canonicalize().unwrap());
    }

    #[test]
    fn test_is_project_root() {
        let temp = TempDir::new().unwrap();
        assert!(!is_project_root(temp.path()));

        fs::write(temp.path().join("stacy.toml"), "").unwrap();
        assert!(is_project_root(temp.path()));
    }
}
