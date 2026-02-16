//! SHA256 hashing utilities for cache key generation
//!
//! Provides file and dependency tree hashing for change detection.

use crate::deps::tree::{build_tree, DependencyTree};
use crate::error::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Compute SHA256 hash of a file's contents
pub fn hash_file(path: &Path) -> Result<String> {
    let content = std::fs::read(path).map_err(|e| {
        crate::error::Error::Config(format!(
            "Failed to read file for hashing {}: {}",
            path.display(),
            e
        ))
    })?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

/// Compute SHA256 hash of a string
pub fn hash_string(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Hash a script and all its dependencies, returning a map of paths to hashes
pub fn hash_dependency_tree(script: &Path) -> Result<DependencyHashes> {
    let tree = build_tree(script)?;
    let mut hashes = DependencyHashes::new();

    // Hash the root script
    if tree.exists {
        hashes.script_hash = hash_file(script)?;
    }

    // Recursively hash all dependencies
    hash_tree_recursive(&tree, &mut hashes)?;

    Ok(hashes)
}

/// Container for all hashes in a dependency tree
#[derive(Debug, Clone, Default)]
pub struct DependencyHashes {
    /// Hash of the main script
    pub script_hash: String,
    /// Hashes of all dependency files (path -> hash)
    pub dependency_hashes: HashMap<PathBuf, String>,
}

impl DependencyHashes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute a combined hash of all files for quick comparison
    pub fn combined_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Add script hash
        hasher.update(self.script_hash.as_bytes());

        // Add dependency hashes in sorted order for consistency
        let mut paths: Vec<_> = self.dependency_hashes.keys().collect();
        paths.sort();

        for path in paths {
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(self.dependency_hashes.get(path).unwrap().as_bytes());
        }

        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

fn hash_tree_recursive(tree: &DependencyTree, hashes: &mut DependencyHashes) -> Result<()> {
    use std::collections::hash_map::Entry;

    for child in &tree.children {
        // Skip circular references and missing files
        if child.is_circular || !child.exists {
            continue;
        }

        // Hash this dependency if we haven't already
        let canonical = child
            .path
            .canonicalize()
            .unwrap_or_else(|_| child.path.clone());

        if let Entry::Vacant(e) = hashes.dependency_hashes.entry(canonical) {
            let hash = hash_file(&child.path)?;
            e.insert(hash);
        }

        // Recurse into children
        hash_tree_recursive(child, hashes)?;
    }

    Ok(())
}

/// Hash the lockfile if it exists
pub fn hash_lockfile(project_root: &Path) -> Result<Option<String>> {
    let lockfile_path = project_root.join("stacy.lock");

    if lockfile_path.exists() {
        Ok(Some(hash_file(&lockfile_path)?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_hash_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let hash = hash_file(&file_path).unwrap();

        // SHA256 hash should be 64 hex characters
        assert_eq!(hash.len(), 64);

        // Same content should produce same hash
        let hash2 = hash_file(&file_path).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_file_different_content() {
        let temp = TempDir::new().unwrap();

        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");

        fs::write(&file1, "content 1").unwrap();
        fs::write(&file2, "content 2").unwrap();

        let hash1 = hash_file(&file1).unwrap();
        let hash2 = hash_file(&file2).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_string() {
        let hash1 = hash_string("hello");
        let hash2 = hash_string("hello");
        let hash3 = hash_string("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_hash_dependency_tree_single_file() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("main.do");
        fs::write(&script, "display \"hello\"").unwrap();

        let hashes = hash_dependency_tree(&script).unwrap();

        assert!(!hashes.script_hash.is_empty());
        assert!(hashes.dependency_hashes.is_empty());
    }

    #[test]
    fn test_hash_dependency_tree_with_deps() {
        let temp = TempDir::new().unwrap();

        // Create helper file
        let helper = temp.path().join("helper.do");
        fs::write(&helper, "display \"helper\"").unwrap();

        // Create main file that includes helper
        let main = temp.path().join("main.do");
        fs::write(&main, "do \"helper.do\"\ndisplay \"main\"").unwrap();

        let hashes = hash_dependency_tree(&main).unwrap();

        assert!(!hashes.script_hash.is_empty());
        assert_eq!(hashes.dependency_hashes.len(), 1);
    }

    #[test]
    fn test_hash_dependency_tree_nested() {
        let temp = TempDir::new().unwrap();

        // Create chain: main -> helper -> utils
        let utils = temp.path().join("utils.do");
        fs::write(&utils, "display \"utils\"").unwrap();

        let helper = temp.path().join("helper.do");
        fs::write(&helper, "do \"utils.do\"\ndisplay \"helper\"").unwrap();

        let main = temp.path().join("main.do");
        fs::write(&main, "do \"helper.do\"\ndisplay \"main\"").unwrap();

        let hashes = hash_dependency_tree(&main).unwrap();

        assert!(!hashes.script_hash.is_empty());
        assert_eq!(hashes.dependency_hashes.len(), 2);
    }

    #[test]
    fn test_combined_hash() {
        let mut hashes = DependencyHashes::new();
        hashes.script_hash = "abc123".to_string();
        hashes
            .dependency_hashes
            .insert(PathBuf::from("helper.do"), "def456".to_string());

        let combined = hashes.combined_hash();
        assert_eq!(combined.len(), 64);

        // Same input should produce same output
        let combined2 = hashes.combined_hash();
        assert_eq!(combined, combined2);
    }

    #[test]
    fn test_combined_hash_order_independent() {
        let mut hashes1 = DependencyHashes::new();
        hashes1.script_hash = "abc".to_string();
        hashes1
            .dependency_hashes
            .insert(PathBuf::from("a.do"), "aaa".to_string());
        hashes1
            .dependency_hashes
            .insert(PathBuf::from("b.do"), "bbb".to_string());

        let mut hashes2 = DependencyHashes::new();
        hashes2.script_hash = "abc".to_string();
        // Insert in different order
        hashes2
            .dependency_hashes
            .insert(PathBuf::from("b.do"), "bbb".to_string());
        hashes2
            .dependency_hashes
            .insert(PathBuf::from("a.do"), "aaa".to_string());

        assert_eq!(hashes1.combined_hash(), hashes2.combined_hash());
    }

    #[test]
    fn test_hash_lockfile_exists() {
        let temp = TempDir::new().unwrap();
        let lockfile = temp.path().join("stacy.lock");
        fs::write(&lockfile, "[packages]").unwrap();

        let hash = hash_lockfile(temp.path()).unwrap();
        assert!(hash.is_some());
        assert_eq!(hash.unwrap().len(), 64);
    }

    #[test]
    fn test_hash_lockfile_not_exists() {
        let temp = TempDir::new().unwrap();
        let hash = hash_lockfile(temp.path()).unwrap();
        assert!(hash.is_none());
    }

    #[test]
    fn test_hash_file_not_found() {
        let result = hash_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }
}
