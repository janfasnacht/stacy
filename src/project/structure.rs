//! Project structure creation for `stacy init`
//!
//! Creates a minimal stacy project:
//! - stacy.toml (configuration with commented defaults)
//! - .gitignore (ignore Stata-generated files)
//!
//! Other files (stacy.lock, ado/) are created on demand by `stacy install`.

use crate::error::{Error, Result};
use std::path::Path;

/// Create minimal project structure (stacy.toml + .gitignore only).
///
/// # Arguments
/// * `root` - Path to the project root directory
/// * `force` - If true, overwrite existing files
///
/// # Returns
/// A list of created files for reporting
pub fn create_project_structure(root: &Path, force: bool) -> Result<Vec<String>> {
    let mut created = Vec::new();

    // Ensure root directory exists
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|e| {
            Error::Config(format!(
                "Failed to create project directory {}: {}",
                root.display(),
                e
            ))
        })?;
        created.push(root.display().to_string());
    }

    // Create stacy.toml
    let config_path = root.join("stacy.toml");
    if !config_path.exists() || force {
        std::fs::write(&config_path, generate_config_template()).map_err(|e| {
            Error::Config(format!(
                "Failed to create stacy.toml at {}: {}",
                config_path.display(),
                e
            ))
        })?;
        created.push("stacy.toml".to_string());
    }

    // Create .gitignore
    let gitignore_path = root.join(".gitignore");
    if !gitignore_path.exists() || force {
        std::fs::write(&gitignore_path, generate_gitignore()).map_err(|e| {
            Error::Config(format!(
                "Failed to create .gitignore at {}: {}",
                gitignore_path.display(),
                e
            ))
        })?;
        created.push(".gitignore".to_string());
    }

    Ok(created)
}

/// Ensure a directory exists, creating it if needed.
/// Used by `stacy run` and `stacy install` to create directories on demand.
pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| {
            Error::Config(format!(
                "Failed to create directory {}: {}",
                path.display(),
                e
            ))
        })?;
    }
    Ok(())
}

/// Project metadata for interactive init
#[derive(Debug, Default)]
pub struct ProjectMetadata {
    pub name: Option<String>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub packages: Vec<PackageToInstall>,
}

/// Package to install during init
#[derive(Debug)]
pub struct PackageToInstall {
    pub name: String,
    pub source: PackageSource,
}

/// Package source type
#[derive(Debug)]
pub enum PackageSource {
    Ssc,
    GitHub {
        user: String,
        repo: String,
        git_ref: Option<String>,
    },
}

/// Generate stacy.toml template with commented defaults.
fn generate_config_template() -> &'static str {
    r#"# stacy project configuration
# See: https://github.com/janfasnacht/stacy

[project]
# name = "my-analysis"
# authors = ["Your Name <you@example.com>"]
# description = "Analysis project"
# url = "https://github.com/user/repo"

[run]
# show_progress = true
# progress_interval_seconds = 10

# Package dependencies (installed to global cache at ~/.cache/stacy/packages/)
# [packages.dependencies]
# estout = "ssc"
# reghdfe = "github:sergiocorreia/reghdfe"

# Note: Stata binary path is NOT set here (it's machine-specific).
# Configure it in ~/.config/stacy/config.toml or use $STATA_BINARY env var.
"#
}

/// Generate stacy.toml with provided metadata.
pub fn generate_config_with_metadata(metadata: &ProjectMetadata) -> String {
    let mut config = String::from(
        "# stacy project configuration\n# See: https://github.com/janfasnacht/stacy\n\n[project]\n",
    );

    if let Some(ref name) = metadata.name {
        config.push_str(&format!("name = \"{}\"\n", name));
    }

    if !metadata.authors.is_empty() {
        let authors: Vec<String> = metadata
            .authors
            .iter()
            .map(|a| format!("\"{}\"", a))
            .collect();
        config.push_str(&format!("authors = [{}]\n", authors.join(", ")));
    }

    if let Some(ref desc) = metadata.description {
        config.push_str(&format!("description = \"{}\"\n", desc));
    }

    if let Some(ref url) = metadata.url {
        config.push_str(&format!("url = \"{}\"\n", url));
    }

    config.push_str("\n[run]\nshow_progress = true\n");

    config
}

/// Create project structure with custom metadata.
pub fn create_project_structure_with_metadata(
    root: &Path,
    metadata: &ProjectMetadata,
    force: bool,
) -> Result<Vec<String>> {
    let mut created = Vec::new();

    // Ensure root directory exists
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|e| {
            Error::Config(format!(
                "Failed to create project directory {}: {}",
                root.display(),
                e
            ))
        })?;
        created.push(root.display().to_string());
    }

    // Create stacy.toml with metadata
    let config_path = root.join("stacy.toml");
    if !config_path.exists() || force {
        let content = generate_config_with_metadata(metadata);
        std::fs::write(&config_path, content).map_err(|e| {
            Error::Config(format!(
                "Failed to create stacy.toml at {}: {}",
                config_path.display(),
                e
            ))
        })?;
        created.push("stacy.toml".to_string());
    }

    // Create .gitignore
    let gitignore_path = root.join(".gitignore");
    if !gitignore_path.exists() || force {
        std::fs::write(&gitignore_path, generate_gitignore()).map_err(|e| {
            Error::Config(format!(
                "Failed to create .gitignore at {}: {}",
                gitignore_path.display(),
                e
            ))
        })?;
        created.push(".gitignore".to_string());
    }

    Ok(created)
}

/// Generate .gitignore for stacy projects.
fn generate_gitignore() -> &'static str {
    r#"# Stata generated files
*.log
*.smcl

# stacy internal files (cache, etc.)
.stacy/

# OS files
.DS_Store
Thumbs.db
"#
}

/// Check if a directory already has project markers.
pub fn has_project_markers(root: &Path) -> bool {
    root.join("stacy.toml").exists() || root.join("stacy.lock").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_structure_minimal() {
        let temp = TempDir::new().unwrap();
        let created = create_project_structure(temp.path(), false).unwrap();

        // Should only create stacy.toml and .gitignore
        assert!(created.contains(&"stacy.toml".to_string()));
        assert!(created.contains(&".gitignore".to_string()));
        assert_eq!(created.len(), 2);

        // Verify files exist
        assert!(temp.path().join("stacy.toml").is_file());
        assert!(temp.path().join(".gitignore").is_file());

        // Verify other files are NOT created
        assert!(!temp.path().join("ado").exists());
        assert!(!temp.path().join("stacy.lock").exists());
    }

    #[test]
    fn test_create_structure_idempotent() {
        let temp = TempDir::new().unwrap();

        // Create once
        let created1 = create_project_structure(temp.path(), false).unwrap();
        assert!(!created1.is_empty());

        // Create again without force - should not recreate files
        let created2 = create_project_structure(temp.path(), false).unwrap();
        assert!(created2.is_empty());
    }

    #[test]
    fn test_create_structure_force() {
        let temp = TempDir::new().unwrap();

        // Create once
        create_project_structure(temp.path(), false).unwrap();

        // Modify stacy.toml
        fs::write(temp.path().join("stacy.toml"), "modified").unwrap();

        // Create again with force - should overwrite
        let created = create_project_structure(temp.path(), true).unwrap();
        assert!(created.contains(&"stacy.toml".to_string()));

        // Verify content was overwritten
        let content = fs::read_to_string(temp.path().join("stacy.toml")).unwrap();
        assert!(content.contains("[project]"));
    }

    #[test]
    fn test_config_template_is_valid_toml() {
        let template = generate_config_template();
        let _: toml::Value = toml::from_str(template).unwrap();
    }

    #[test]
    fn test_has_project_markers() {
        let temp = TempDir::new().unwrap();

        // No markers initially
        assert!(!has_project_markers(temp.path()));

        // Add stacy.toml
        fs::write(temp.path().join("stacy.toml"), "").unwrap();
        assert!(has_project_markers(temp.path()));
    }

    #[test]
    fn test_has_project_markers_lock_file() {
        let temp = TempDir::new().unwrap();

        // No markers initially
        assert!(!has_project_markers(temp.path()));

        // Add lockfile
        fs::write(temp.path().join("stacy.lock"), "version = \"1\"").unwrap();
        assert!(has_project_markers(temp.path()));
    }

    #[test]
    fn test_create_in_new_directory() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("new").join("project");

        let created = create_project_structure(&nested, false).unwrap();
        assert!(!created.is_empty());
        assert!(nested.join("stacy.toml").exists());
    }

    #[test]
    fn test_ensure_dir_exists() {
        let temp = TempDir::new().unwrap();
        let logs_dir = temp.path().join("logs");

        assert!(!logs_dir.exists());
        ensure_dir_exists(&logs_dir).unwrap();
        assert!(logs_dir.exists());

        // Should be idempotent
        ensure_dir_exists(&logs_dir).unwrap();
        assert!(logs_dir.exists());
    }
}
