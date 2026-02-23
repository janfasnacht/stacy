//! Build and display dependency trees
//!
//! Recursively analyzes Stata scripts to build a complete dependency tree,
//! detecting circular dependencies and missing files.

use super::parser::{parse_dependencies, DependencyType};
use crate::error::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A node in the dependency tree
#[derive(Debug, Clone)]
pub struct DependencyTree {
    /// Path to this script
    pub path: PathBuf,
    /// Type of dependency (None for root)
    pub dep_type: Option<DependencyType>,
    /// Child dependencies
    pub children: Vec<DependencyTree>,
    /// Whether this node creates a circular dependency
    pub is_circular: bool,
    /// Whether the file exists
    pub exists: bool,
    /// Line number in parent (None for root)
    pub line_number: Option<usize>,
}

impl DependencyTree {
    /// Create a new root node
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            dep_type: None,
            children: Vec::new(),
            is_circular: false,
            exists: true,
            line_number: None,
        }
    }

    /// Get total count of unique dependencies (excluding root)
    pub fn unique_count(&self) -> usize {
        let mut seen = HashSet::new();
        self.collect_unique(&mut seen);
        seen.len()
    }

    fn collect_unique(&self, seen: &mut HashSet<PathBuf>) {
        for child in &self.children {
            if !child.is_circular {
                seen.insert(child.path.clone());
                child.collect_unique(seen);
            }
        }
    }

    /// Check if tree has any circular dependencies
    pub fn has_circular(&self) -> bool {
        self.is_circular || self.children.iter().any(|c| c.has_circular())
    }

    /// Check if tree has any missing files
    pub fn has_missing(&self) -> bool {
        !self.exists || self.children.iter().any(|c| c.has_missing())
    }

    /// Get all circular dependency paths
    pub fn circular_paths(&self) -> Vec<&Path> {
        let mut paths = Vec::new();
        self.collect_circular(&mut paths);
        paths
    }

    fn collect_circular<'a>(&'a self, paths: &mut Vec<&'a Path>) {
        if self.is_circular {
            paths.push(&self.path);
        }
        for child in &self.children {
            child.collect_circular(paths);
        }
    }

    /// Get all missing file paths
    pub fn missing_paths(&self) -> Vec<&Path> {
        let mut paths = Vec::new();
        self.collect_missing(&mut paths);
        paths
    }

    fn collect_missing<'a>(&'a self, paths: &mut Vec<&'a Path>) {
        if !self.exists && !self.is_circular {
            paths.push(&self.path);
        }
        for child in &self.children {
            child.collect_missing(paths);
        }
    }

    /// Format tree as human-readable string
    pub fn format_tree(&self) -> String {
        let mut output = String::new();
        self.format_tree_recursive(&mut output, "", true);
        output
    }

    fn format_tree_recursive(&self, output: &mut String, prefix: &str, is_last: bool) {
        // Format current node
        let connector = if prefix.is_empty() {
            ""
        } else if is_last {
            "└── "
        } else {
            "├── "
        };

        let path_display = self.path.display().to_string();
        let suffix = if self.is_circular {
            " (circular)"
        } else if !self.exists {
            " (not found)"
        } else {
            ""
        };

        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&path_display);
        output.push_str(suffix);
        output.push('\n');

        // Format children
        let child_prefix = if prefix.is_empty() {
            String::new()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        for (i, child) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;
            child.format_tree_recursive(output, &child_prefix, is_last_child);
        }
    }

    /// Convert to flat list of dependencies
    pub fn flatten(&self) -> Vec<FlatDependency> {
        let mut deps = Vec::new();
        self.flatten_recursive(&mut deps, 0);
        deps
    }

    fn flatten_recursive(&self, deps: &mut Vec<FlatDependency>, depth: usize) {
        for child in &self.children {
            deps.push(FlatDependency {
                path: child.path.clone(),
                dep_type: child.dep_type,
                depth,
                is_circular: child.is_circular,
                exists: child.exists,
            });
            if !child.is_circular {
                child.flatten_recursive(deps, depth + 1);
            }
        }
    }
}

/// A flattened dependency entry
#[derive(Debug, Clone)]
pub struct FlatDependency {
    pub path: PathBuf,
    pub dep_type: Option<DependencyType>,
    pub depth: usize,
    pub is_circular: bool,
    pub exists: bool,
}

/// Build a complete dependency tree for a script
///
/// # Arguments
/// * `script` - Path to the root .do file
///
/// # Returns
/// A dependency tree with all nested dependencies resolved
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use stacy::deps::tree::build_tree;
///
/// let tree = build_tree(Path::new("main.do"))?;
/// println!("{}", tree.format_tree());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn build_tree(script: &Path) -> Result<DependencyTree> {
    let mut visited = HashSet::new();
    build_tree_recursive(script, None, None, &mut visited)
}

fn build_tree_recursive(
    script: &Path,
    dep_type: Option<DependencyType>,
    line_number: Option<usize>,
    visited: &mut HashSet<PathBuf>,
) -> Result<DependencyTree> {
    // Canonicalize path for consistent comparison
    let canonical = if script.exists() {
        script
            .canonicalize()
            .unwrap_or_else(|_| script.to_path_buf())
    } else {
        script.to_path_buf()
    };

    // Check for circular dependency
    if visited.contains(&canonical) {
        return Ok(DependencyTree {
            path: script.to_path_buf(),
            dep_type,
            children: Vec::new(),
            is_circular: true,
            exists: script.exists(),
            line_number,
        });
    }

    // Check if file exists
    if !script.exists() {
        return Ok(DependencyTree {
            path: script.to_path_buf(),
            dep_type,
            children: Vec::new(),
            is_circular: false,
            exists: false,
            line_number,
        });
    }

    // Mark as visited
    visited.insert(canonical.clone());

    // Parse dependencies
    let dependencies = parse_dependencies(script)?;

    // Get base directory for resolving relative paths
    let base_dir = script.parent().unwrap_or(Path::new("."));

    // Build children
    let mut children = Vec::new();
    for dep in dependencies {
        let resolved_path = dep.resolve(base_dir);
        let child = build_tree_recursive(
            &resolved_path,
            Some(dep.dep_type),
            Some(dep.line_number),
            visited,
        )?;
        children.push(child);
    }

    // Unmark as visited (allow same file to appear in different branches)
    visited.remove(&canonical);

    Ok(DependencyTree {
        path: script.to_path_buf(),
        dep_type,
        children,
        is_circular: false,
        exists: true,
        line_number,
    })
}

/// Analyze dependencies and return summary information
#[derive(Debug)]
pub struct DependencyAnalysis {
    pub tree: DependencyTree,
    pub unique_count: usize,
    pub has_circular: bool,
    pub has_missing: bool,
    pub circular_paths: Vec<PathBuf>,
    pub missing_paths: Vec<PathBuf>,
}

pub fn analyze_dependencies(script: &Path) -> Result<DependencyAnalysis> {
    let tree = build_tree(script)?;

    let circular_paths: Vec<PathBuf> = tree
        .circular_paths()
        .iter()
        .map(|p| p.to_path_buf())
        .collect();
    let missing_paths: Vec<PathBuf> = tree
        .missing_paths()
        .iter()
        .map(|p| p.to_path_buf())
        .collect();

    Ok(DependencyAnalysis {
        unique_count: tree.unique_count(),
        has_circular: tree.has_circular(),
        has_missing: tree.has_missing(),
        circular_paths,
        missing_paths,
        tree,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_single_file_no_deps() {
        let temp = TempDir::new().unwrap();
        let script = create_test_file(temp.path(), "main.do", "display \"hello\"");

        let tree = build_tree(&script).unwrap();
        assert!(tree.children.is_empty());
        assert_eq!(tree.unique_count(), 0);
        assert!(!tree.has_circular());
    }

    #[test]
    fn test_single_dependency() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "helper.do", "display \"helper\"");
        let main = create_test_file(temp.path(), "main.do", "do \"helper.do\"");

        let tree = build_tree(&main).unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.unique_count(), 1);
    }

    #[test]
    fn test_nested_dependencies() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "c.do", "display \"c\"");
        create_test_file(temp.path(), "b.do", "do \"c.do\"");
        let main = create_test_file(temp.path(), "a.do", "do \"b.do\"");

        let tree = build_tree(&main).unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].children.len(), 1);
        assert_eq!(tree.unique_count(), 2);
    }

    #[test]
    fn test_circular_dependency() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "a.do", "do \"b.do\"");
        create_test_file(temp.path(), "b.do", "do \"a.do\"");

        let tree = build_tree(&temp.path().join("a.do")).unwrap();
        assert!(tree.has_circular());
        assert_eq!(tree.circular_paths().len(), 1);
    }

    #[test]
    fn test_missing_file() {
        let temp = TempDir::new().unwrap();
        let main = create_test_file(temp.path(), "main.do", "do \"missing.do\"");

        let tree = build_tree(&main).unwrap();
        assert!(tree.has_missing());
        assert_eq!(tree.missing_paths().len(), 1);
    }

    #[test]
    fn test_diamond_dependency() {
        // a -> b -> d
        // a -> c -> d
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "d.do", "display \"d\"");
        create_test_file(temp.path(), "b.do", "do \"d.do\"");
        create_test_file(temp.path(), "c.do", "do \"d.do\"");
        let main = create_test_file(temp.path(), "a.do", "do \"b.do\"\ndo \"c.do\"");

        let tree = build_tree(&main).unwrap();
        // d.do appears twice but should count as 1 unique dependency
        // Actually in our implementation, d appears in both branches (not circular)
        // unique_count counts unique paths
        assert_eq!(tree.unique_count(), 3); // b, c, d
        assert!(!tree.has_circular());
    }

    #[test]
    fn test_format_tree() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "helper.do", "display \"helper\"");
        let main = create_test_file(temp.path(), "main.do", "do \"helper.do\"");

        let tree = build_tree(&main).unwrap();
        let output = tree.format_tree();
        assert!(output.contains("main.do"));
        assert!(output.contains("helper.do"));
    }

    #[test]
    fn test_flatten() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "c.do", "display \"c\"");
        create_test_file(temp.path(), "b.do", "do \"c.do\"");
        let main = create_test_file(temp.path(), "a.do", "do \"b.do\"");

        let tree = build_tree(&main).unwrap();
        let flat = tree.flatten();
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].depth, 0);
        assert_eq!(flat[1].depth, 1);
    }

    #[test]
    fn test_subdirectory_deps() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "utils/helper.do", "display \"helper\"");
        let main = create_test_file(temp.path(), "main.do", "do \"utils/helper.do\"");

        let tree = build_tree(&main).unwrap();
        assert_eq!(tree.children.len(), 1);
        assert!(tree.children[0].exists);
    }
}
