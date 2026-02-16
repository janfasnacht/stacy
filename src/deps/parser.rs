//! Parse Stata scripts for dependency statements
//!
//! Extracts `do`, `run`, and `include` statements from .do files.

use crate::error::{Error, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Type of dependency statement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// `do "file.do"` - execute script
    Do,
    /// `run "file.do"` - execute script quietly
    Run,
    /// `include "file.do"` - include script inline
    Include,
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Do => write!(f, "do"),
            DependencyType::Run => write!(f, "run"),
            DependencyType::Include => write!(f, "include"),
        }
    }
}

/// A dependency found in a Stata script
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Path to the dependency (as written in the script)
    pub path: PathBuf,
    /// Type of dependency (do/run/include)
    pub dep_type: DependencyType,
    /// Line number where the dependency was found (1-indexed)
    pub line_number: usize,
    /// The raw statement text
    pub raw_statement: String,
}

impl Dependency {
    /// Resolve the dependency path relative to a base directory
    pub fn resolve(&self, base_dir: &Path) -> PathBuf {
        if self.path.is_absolute() {
            self.path.clone()
        } else {
            base_dir.join(&self.path)
        }
    }
}

// Regex patterns for dependency statements
// Matches: do "file.do", do `"file.do"', do file.do
// Also handles: run, include
static DO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)^\s*do\s+(?:`"([^"]+)"'|"([^"]+)"|'([^']+)'|(\S+))"#).unwrap()
});

static RUN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)^\s*run\s+(?:`"([^"]+)"'|"([^"]+)"|'([^']+)'|(\S+))"#).unwrap()
});

static INCLUDE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)^\s*include\s+(?:`"([^"]+)"'|"([^"]+)"|'([^']+)'|(\S+))"#).unwrap()
});

/// Parse a Stata script file for dependencies
///
/// # Arguments
/// * `script` - Path to the .do file to parse
///
/// # Returns
/// A list of dependencies found in the script
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use stata_cli::deps::parser::parse_dependencies;
///
/// let deps = parse_dependencies(Path::new("analysis.do"))?;
/// for dep in deps {
///     println!("Line {}: {} {}", dep.line_number, dep.dep_type, dep.path.display());
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_dependencies(script: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(script).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read {}: {}", script.display(), e),
        ))
    })?;

    parse_dependencies_from_content(&content)
}

/// Parse dependencies from script content (for testing)
pub fn parse_dependencies_from_content(content: &str) -> Result<Vec<Dependency>> {
    let mut dependencies = Vec::new();

    for (line_number, line) in content.lines().enumerate() {
        let line_number = line_number + 1; // 1-indexed

        // Skip comments
        let trimmed = line.trim();
        if trimmed.starts_with('*') || trimmed.starts_with("//") {
            continue;
        }

        // Remove inline comments for parsing
        let line_without_comment = if let Some(pos) = line.find("//") {
            &line[..pos]
        } else {
            line
        };

        // Check each pattern
        if let Some(dep) = try_parse_pattern(line_without_comment, &DO_PATTERN, DependencyType::Do)
        {
            dependencies.push(Dependency {
                line_number,
                raw_statement: line.trim().to_string(),
                ..dep
            });
        } else if let Some(dep) =
            try_parse_pattern(line_without_comment, &RUN_PATTERN, DependencyType::Run)
        {
            dependencies.push(Dependency {
                line_number,
                raw_statement: line.trim().to_string(),
                ..dep
            });
        } else if let Some(dep) = try_parse_pattern(
            line_without_comment,
            &INCLUDE_PATTERN,
            DependencyType::Include,
        ) {
            dependencies.push(Dependency {
                line_number,
                raw_statement: line.trim().to_string(),
                ..dep
            });
        }
    }

    Ok(dependencies)
}

fn try_parse_pattern(line: &str, pattern: &Regex, dep_type: DependencyType) -> Option<Dependency> {
    pattern.captures(line).map(|caps| {
        // Extract path from whichever capture group matched
        let path_str = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .or_else(|| caps.get(4))
            .map(|m| m.as_str())
            .unwrap_or("");

        // Clean up the path
        let path = normalize_path(path_str);

        Dependency {
            path,
            dep_type,
            line_number: 0, // Will be set by caller
            raw_statement: String::new(),
        }
    })
}

/// Normalize a path string from Stata syntax
fn normalize_path(path_str: &str) -> PathBuf {
    let path_str = path_str.trim();

    // Add .do extension if not present and doesn't have another extension
    let path = PathBuf::from(path_str);
    if path.extension().is_none() {
        PathBuf::from(format!("{}.do", path_str))
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_do_quoted() {
        let content = r#"do "analysis.do""#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dep_type, DependencyType::Do);
        assert_eq!(deps[0].path, PathBuf::from("analysis.do"));
    }

    #[test]
    fn test_parse_do_unquoted() {
        let content = "do analysis";
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].path, PathBuf::from("analysis.do"));
    }

    #[test]
    fn test_parse_do_compound_quotes() {
        let content = r#"do `"path with spaces/file.do"'"#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].path, PathBuf::from("path with spaces/file.do"));
    }

    #[test]
    fn test_parse_run() {
        let content = r#"run "helper.do""#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dep_type, DependencyType::Run);
    }

    #[test]
    fn test_parse_include() {
        let content = r#"include "common.do""#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dep_type, DependencyType::Include);
    }

    #[test]
    fn test_skip_comments() {
        let content = r#"
* do "commented.do"
// do "also_commented.do"
do "real.do"
"#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].path, PathBuf::from("real.do"));
    }

    #[test]
    fn test_case_insensitive() {
        let content = r#"
DO "upper.do"
Do "mixed.do"
"#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_line_numbers() {
        let content = r#"
// comment
do "first.do"
// another comment
do "second.do"
"#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].line_number, 3);
        assert_eq!(deps[1].line_number, 5);
    }

    #[test]
    fn test_multiple_types() {
        let content = r#"
do "a.do"
run "b.do"
include "c.do"
"#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].dep_type, DependencyType::Do);
        assert_eq!(deps[1].dep_type, DependencyType::Run);
        assert_eq!(deps[2].dep_type, DependencyType::Include);
    }

    #[test]
    fn test_path_with_directory() {
        let content = r#"do "utils/helper.do""#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps[0].path, PathBuf::from("utils/helper.do"));
    }

    #[test]
    fn test_inline_comment_removed() {
        let content = r#"do "real.do" // this is a comment"#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].path, PathBuf::from("real.do"));
    }

    #[test]
    fn test_empty_file() {
        let content = "";
        let deps = parse_dependencies_from_content(content).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_no_dependencies() {
        let content = r#"
display "Hello"
regress y x
"#;
        let deps = parse_dependencies_from_content(content).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_resolve_relative_path() {
        let dep = Dependency {
            path: PathBuf::from("utils/helper.do"),
            dep_type: DependencyType::Do,
            line_number: 1,
            raw_statement: String::new(),
        };
        let resolved = dep.resolve(Path::new("/project/scripts"));
        assert_eq!(resolved, PathBuf::from("/project/scripts/utils/helper.do"));
    }

    #[test]
    fn test_resolve_absolute_path() {
        let dep = Dependency {
            path: PathBuf::from("/absolute/path/script.do"),
            dep_type: DependencyType::Do,
            line_number: 1,
            raw_statement: String::new(),
        };
        let resolved = dep.resolve(Path::new("/project/scripts"));
        assert_eq!(resolved, PathBuf::from("/absolute/path/script.do"));
    }
}
