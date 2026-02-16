//! Test discovery by convention
//!
//! Discovers test files using naming conventions:
//! - `test_*.do`, `*_test.do` anywhere in project
//! - All `.do` files in `tests/` or `test/` directories

use crate::error::Result;
use std::path::{Path, PathBuf};

/// Information about a discovered test
#[derive(Debug, Clone)]
pub struct TestFile {
    /// Full path to the test file
    pub path: PathBuf,
    /// Test name (derived from filename)
    pub name: String,
}

impl TestFile {
    /// Create a new TestFile from a path
    pub fn from_path(path: PathBuf) -> Self {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        Self { path, name }
    }
}

/// Discover tests in a project directory
pub fn discover_tests(project_root: &Path, filters: &[String]) -> Result<Vec<TestFile>> {
    let mut tests = Vec::new();

    // Strategy 1: Find test_*.do and *_test.do anywhere in project
    discover_by_naming_convention(project_root, &mut tests)?;

    // Strategy 2: Find all .do files in tests/ or test/ directories
    discover_in_test_directories(project_root, &mut tests)?;

    // Remove duplicates (same path)
    tests.sort_by(|a, b| a.path.cmp(&b.path));
    tests.dedup_by(|a, b| a.path == b.path);

    // Apply filters if any
    if !filters.is_empty() {
        tests.retain(|test| {
            filters
                .iter()
                .any(|f| test.name.contains(f) || test.path.to_string_lossy().contains(f))
        });
    }

    // Sort by name for consistent ordering
    tests.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(tests)
}

/// Find a specific test by name or path
pub fn find_test(project_root: &Path, name_or_path: &str) -> Result<Option<TestFile>> {
    // If it looks like a path (contains / or ends with .do), try to find it directly
    if name_or_path.contains('/') || name_or_path.contains('\\') || name_or_path.ends_with(".do") {
        let path = if Path::new(name_or_path).is_absolute() {
            PathBuf::from(name_or_path)
        } else {
            project_root.join(name_or_path)
        };

        if path.exists() {
            return Ok(Some(TestFile::from_path(path)));
        }
    }

    // Otherwise, discover all tests and find by name
    let tests = discover_tests(project_root, &[])?;

    // Exact match first
    if let Some(test) = tests.iter().find(|t| t.name == name_or_path) {
        return Ok(Some(test.clone()));
    }

    // Partial match
    if let Some(test) = tests.iter().find(|t| t.name.contains(name_or_path)) {
        return Ok(Some(test.clone()));
    }

    Ok(None)
}

/// Discover tests by naming convention (test_*.do, *_test.do)
fn discover_by_naming_convention(dir: &Path, tests: &mut Vec<TestFile>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Skip hidden directories and common non-test directories
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if !name.starts_with('.') && name != "node_modules" && name != "target" {
                discover_by_naming_convention(&path, tests)?;
            }
        } else if path.extension().map(|e| e == "do").unwrap_or(false) {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Check naming convention: test_*.do or *_test.do
            if stem.starts_with("test_") || stem.ends_with("_test") {
                tests.push(TestFile::from_path(path));
            }
        }
    }

    Ok(())
}

/// Discover all .do files in tests/ or test/ directories
fn discover_in_test_directories(project_root: &Path, tests: &mut Vec<TestFile>) -> Result<()> {
    for dir_name in &["tests", "test"] {
        let test_dir = project_root.join(dir_name);
        if test_dir.is_dir() {
            discover_all_do_files(&test_dir, tests)?;
        }
    }
    Ok(())
}

/// Recursively discover all .do files in a directory
fn discover_all_do_files(dir: &Path, tests: &mut Vec<TestFile>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if !name.starts_with('.') {
                discover_all_do_files(&path, tests)?;
            }
        } else if path.extension().map(|e| e == "do").unwrap_or(false) {
            tests.push(TestFile::from_path(path));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, "* test file").unwrap();
    }

    #[test]
    fn test_discover_by_naming_convention() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create test files with naming convention
        create_test_file(root, "test_foo.do");
        create_test_file(root, "bar_test.do");
        create_test_file(root, "regular.do"); // Should NOT be discovered
        create_test_file(root, "src/test_nested.do");

        let tests = discover_tests(root, &[]).unwrap();

        assert_eq!(tests.len(), 3);
        assert!(tests.iter().any(|t| t.name == "test_foo"));
        assert!(tests.iter().any(|t| t.name == "bar_test"));
        assert!(tests.iter().any(|t| t.name == "test_nested"));
    }

    #[test]
    fn test_discover_in_test_directories() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create files in tests/ directory
        create_test_file(root, "tests/foo.do");
        create_test_file(root, "tests/bar.do");
        create_test_file(root, "tests/nested/baz.do");

        // Create file in test/ directory
        create_test_file(root, "test/qux.do");

        let tests = discover_tests(root, &[]).unwrap();

        assert_eq!(tests.len(), 4);
        assert!(tests.iter().any(|t| t.name == "foo"));
        assert!(tests.iter().any(|t| t.name == "bar"));
        assert!(tests.iter().any(|t| t.name == "baz"));
        assert!(tests.iter().any(|t| t.name == "qux"));
    }

    #[test]
    fn test_discover_with_filter() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        create_test_file(root, "test_foo.do");
        create_test_file(root, "test_bar.do");
        create_test_file(root, "test_baz.do");

        let tests = discover_tests(root, &["foo".to_string()]).unwrap();

        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_foo");
    }

    #[test]
    fn test_discover_removes_duplicates() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // test_foo.do matches naming convention AND is in tests/
        create_test_file(root, "tests/test_foo.do");

        let tests = discover_tests(root, &[]).unwrap();

        // Should only appear once
        assert_eq!(tests.len(), 1);
    }

    #[test]
    fn test_find_test_by_name() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        create_test_file(root, "test_foo.do");
        create_test_file(root, "test_bar.do");

        let found = find_test(root, "test_foo").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test_foo");
    }

    #[test]
    fn test_find_test_by_path() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        create_test_file(root, "tests/my_test.do");

        let found = find_test(root, "tests/my_test.do").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "my_test");
    }

    #[test]
    fn test_find_test_not_found() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let found = find_test(root, "nonexistent").unwrap();
        assert!(found.is_none());
    }
}
