pub mod config;
pub mod root;
pub mod structure;
pub mod user_config;

// Re-export main types
pub use config::Config;

use crate::error::Result;
use std::path::PathBuf;

/// Represents a stacy project with its root directory, configuration, and lockfile.
#[derive(Debug)]
pub struct Project {
    /// Path to the project root directory
    pub root: PathBuf,
    /// Configuration loaded from stacy.toml (None if no config file)
    pub config: Option<Config>,
    /// Lockfile loaded from stacy.lock (None if no lockfile)
    pub lockfile: Option<Lockfile>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Lockfile {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stacy_version: Option<String>,
    pub packages: std::collections::HashMap<String, PackageEntry>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackageEntry {
    pub version: String,
    pub source: PackageSource,
    pub checksum: Option<String>,
    /// Dependency group: "production", "dev", or "test"
    #[serde(default = "default_group")]
    pub group: String,
}

fn default_group() -> String {
    "production".to_string()
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum PackageSource {
    SSC {
        name: String,
    },
    GitHub {
        repo: String,
        tag: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        commit: Option<String>,
    },
    Local {
        path: String,
    },
    Net {
        url: String,
    },
}

impl Project {
    /// Find project root by walking up from the current working directory.
    ///
    /// Looks for project markers in this order:
    /// 1. `stacy.toml` - explicit project configuration
    /// 2. `stacy.lock` - package lockfile
    /// 3. `ado/` - project-local packages directory
    ///
    /// Returns `None` if no project is found.
    pub fn find() -> Result<Option<Project>> {
        let project_root = root::find_project_root_from_cwd()?;

        match project_root {
            Some(root_info) => {
                // Load config from stacy.toml if it exists
                let config = config::load_config(&root_info.path)?;

                // Lockfile is loaded on-demand by commands that need it (sync, install)
                // rather than eagerly here, to avoid unnecessary file I/O

                Ok(Some(Project {
                    root: root_info.path,
                    config,
                    lockfile: None,
                }))
            }
            None => Ok(None),
        }
    }

    /// Find project root starting from a specific directory.
    pub fn find_from(start_dir: &std::path::Path) -> Result<Option<Project>> {
        let project_root = root::find_project_root(start_dir)?;

        match project_root {
            Some(root_info) => {
                // Load config from stacy.toml if it exists
                let config = config::load_config(&root_info.path)?;

                Ok(Some(Project {
                    root: root_info.path,
                    config,
                    lockfile: None,
                }))
            }
            None => Ok(None),
        }
    }

    /// Create new project structure.
    ///
    /// Creates the standard stacy project layout:
    /// - stacy.toml (configuration)
    /// - stacy.lock (empty lockfile)
    /// - ado/, logs/, tmp/ directories
    /// - .gitignore
    ///
    /// # Arguments
    /// * `path` - Path to create the project in
    pub fn init(path: &std::path::Path) -> Result<Project> {
        // Create the project structure
        structure::create_project_structure(path, false)?;

        // Load the newly created config
        let config = config::load_config(path)?;

        Ok(Project {
            root: path.to_path_buf(),
            config,
            lockfile: None,
        })
    }

    /// Create new project structure, overwriting existing files.
    pub fn init_force(path: &std::path::Path) -> Result<Project> {
        // Create the project structure with force
        structure::create_project_structure(path, true)?;

        // Load the newly created config
        let config = config::load_config(path)?;

        Ok(Project {
            root: path.to_path_buf(),
            config,
            lockfile: None,
        })
    }

    /// Get the project root path
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Check if the project has a configuration file
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }

    /// Check if the project has a lockfile
    pub fn has_lockfile(&self) -> bool {
        self.lockfile.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_source_with_commit_roundtrip() {
        let source = PackageSource::GitHub {
            repo: "sergiocorreia/reghdfe".to_string(),
            tag: "main".to_string(),
            commit: Some("abc123def456abc123def456abc123def456abc1".to_string()),
        };

        let toml_str = toml::to_string(&source).unwrap();
        assert!(toml_str.contains("commit"));
        let deserialized: PackageSource = toml::from_str(&toml_str).unwrap();

        match deserialized {
            PackageSource::GitHub { commit, .. } => {
                assert_eq!(
                    commit.as_deref(),
                    Some("abc123def456abc123def456abc123def456abc1")
                );
            }
            _ => panic!("Expected GitHub variant"),
        }
    }

    #[test]
    fn test_github_source_without_commit_backwards_compat() {
        // Old lockfile TOML without commit field
        let toml_str = r#"
            type = "GitHub"
            repo = "sergiocorreia/reghdfe"
            tag = "main"
        "#;

        let source: PackageSource = toml::from_str(toml_str).unwrap();
        match source {
            PackageSource::GitHub { commit, .. } => {
                assert_eq!(commit, None);
            }
            _ => panic!("Expected GitHub variant"),
        }
    }

    #[test]
    fn test_net_source_roundtrip() {
        let source = PackageSource::Net {
            url: "http://www.stata.com/users/vwiggins/".to_string(),
        };

        let toml_str = toml::to_string(&source).unwrap();
        assert!(toml_str.contains("Net"));
        assert!(toml_str.contains("http://www.stata.com/users/vwiggins/"));

        let deserialized: PackageSource = toml::from_str(&toml_str).unwrap();
        match deserialized {
            PackageSource::Net { url } => {
                assert_eq!(url, "http://www.stata.com/users/vwiggins/");
            }
            _ => panic!("Expected Net variant"),
        }
    }

    #[test]
    fn test_github_source_none_commit_omitted_in_serialization() {
        let source = PackageSource::GitHub {
            repo: "user/repo".to_string(),
            tag: "v1.0".to_string(),
            commit: None,
        };

        let toml_str = toml::to_string(&source).unwrap();
        assert!(!toml_str.contains("commit"));
    }
}
