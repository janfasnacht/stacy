//! stacy.toml configuration file parsing and validation
//!
//! Handles loading, parsing, and validating project configuration from `stacy.toml`.
//! All fields have sensible defaults, so an empty or missing config file works.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Project configuration loaded from stacy.toml
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Project-level settings
    pub project: ProjectSection,
    /// Execution settings (for `stacy run`)
    pub run: RunSection,
    /// Package management settings
    pub packages: PackagesSection,
    /// Task definitions (for `stacy task`)
    pub scripts: ScriptsSection,
}

/// Project-level settings (committed to version control)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectSection {
    /// Project name (for display purposes)
    pub name: Option<String>,
    /// Project authors/contacts (e.g., `["Jane Doe <jane@example.com>"]`)
    pub authors: Option<Vec<String>>,
    /// Project description
    pub description: Option<String>,
    /// Project URL (e.g., GitHub repository)
    pub url: Option<String>,
}

/// Execution settings for `stacy run`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct RunSection {
    /// Directory for log files (relative to project root)
    pub log_dir: PathBuf,
    /// Show progress indicator during execution
    pub show_progress: bool,
    /// Interval in seconds for progress updates
    pub progress_interval_seconds: u64,
    /// Maximum log file size in MB before warning
    pub max_log_size_mb: u64,
}

impl Default for RunSection {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("logs"),
            show_progress: true,
            progress_interval_seconds: 10,
            max_log_size_mb: 50,
        }
    }
}

/// Package specification in stacy.toml
///
/// Supports two formats:
/// - Simple: just the source string, e.g., `estout = "ssc"` or `ftools = "github:sergiocorreia/ftools"`
/// - Detailed: object with source and optional version, e.g., `{ source = "ssc", version = "1.0.0" }`
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum PackageSpec {
    /// Simple format: just source string (e.g., "ssc" or "github:user/repo")
    Simple(String),
    /// Detailed format: object with source and optional version
    Detailed {
        source: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
}

impl PackageSpec {
    /// Get the source string
    pub fn source(&self) -> &str {
        match self {
            PackageSpec::Simple(s) => s,
            PackageSpec::Detailed { source, .. } => source,
        }
    }

    /// Get the version if specified
    pub fn version(&self) -> Option<&str> {
        match self {
            PackageSpec::Simple(_) => None,
            PackageSpec::Detailed { version, .. } => version.as_deref(),
        }
    }

    /// Create a simple spec from source string
    pub fn simple(source: impl Into<String>) -> Self {
        PackageSpec::Simple(source.into())
    }
}

/// Dependency group for categorizing packages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencyGroup {
    /// Production dependencies (installed by default)
    Production,
    /// Development dependencies (--with dev)
    Dev,
    /// Test dependencies (--with test)
    Test,
}

impl DependencyGroup {
    /// Get the display name for this group
    pub fn as_str(&self) -> &'static str {
        match self {
            DependencyGroup::Production => "production",
            DependencyGroup::Dev => "dev",
            DependencyGroup::Test => "test",
        }
    }
}

impl std::fmt::Display for DependencyGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Package management settings
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct PackagesSection {
    /// Production dependencies: package_name -> source spec
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub dependencies: HashMap<String, PackageSpec>,
    /// Development dependencies: package_name -> source spec
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub dev: HashMap<String, PackageSpec>,
    /// Test dependencies: package_name -> source spec
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub test: HashMap<String, PackageSpec>,
}

impl PackagesSection {
    /// Check if a package exists in any dependency group
    pub fn has_package(&self, name: &str) -> bool {
        self.dependencies.contains_key(name)
            || self.dev.contains_key(name)
            || self.test.contains_key(name)
    }

    /// Check if a package is a dev dependency
    pub fn is_dev_package(&self, name: &str) -> bool {
        self.dev.contains_key(name)
    }

    /// Check if a package is a test dependency
    pub fn is_test_package(&self, name: &str) -> bool {
        self.test.contains_key(name)
    }

    /// Get the dependency group for a package
    pub fn get_package_group(&self, name: &str) -> Option<DependencyGroup> {
        if self.dependencies.contains_key(name) {
            Some(DependencyGroup::Production)
        } else if self.dev.contains_key(name) {
            Some(DependencyGroup::Dev)
        } else if self.test.contains_key(name) {
            Some(DependencyGroup::Test)
        } else {
            None
        }
    }

    /// Add a dependency to a specific group
    pub fn add_dependency(&mut self, name: String, spec: PackageSpec, group: DependencyGroup) {
        match group {
            DependencyGroup::Production => {
                self.dependencies.insert(name, spec);
            }
            DependencyGroup::Dev => {
                self.dev.insert(name, spec);
            }
            DependencyGroup::Test => {
                self.test.insert(name, spec);
            }
        }
    }

    /// Remove a dependency from any group
    /// Returns the removed spec if found
    pub fn remove_dependency(&mut self, name: &str) -> Option<PackageSpec> {
        self.dependencies
            .remove(name)
            .or_else(|| self.dev.remove(name))
            .or_else(|| self.test.remove(name))
    }

    /// Get all packages with their group
    pub fn all_packages(&self) -> impl Iterator<Item = (&String, &PackageSpec, DependencyGroup)> {
        self.dependencies
            .iter()
            .map(|(k, v)| (k, v, DependencyGroup::Production))
            .chain(self.dev.iter().map(|(k, v)| (k, v, DependencyGroup::Dev)))
            .chain(self.test.iter().map(|(k, v)| (k, v, DependencyGroup::Test)))
    }

    /// Get packages filtered by groups
    pub fn packages_by_groups(
        &self,
        groups: &[DependencyGroup],
    ) -> impl Iterator<Item = (&String, &PackageSpec, DependencyGroup)> {
        let groups_set: std::collections::HashSet<_> = groups.iter().copied().collect();
        self.all_packages()
            .filter(move |(_, _, group)| groups_set.contains(group))
    }
}

/// Task/script definitions for `stacy task` command
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ScriptsSection {
    /// Task definitions keyed by task name
    #[serde(flatten)]
    pub tasks: HashMap<String, TaskDef>,
}

/// Task definition - supports multiple formats via untagged enum
///
/// # Examples
///
/// Simple format (just a script path):
/// ```toml
/// clean = "src/01_clean.do"
/// ```
///
/// Sequential format (array of task names to run in order):
/// ```toml
/// all = ["clean", "analyze", "report"]
/// ```
///
/// Complex format (object with options):
/// ```toml
/// analyze = { script = "src/02_analyze.do", description = "Run main analysis" }
/// outputs = { parallel = ["tables", "figures"] }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TaskDef {
    /// Simple: just a script path - `clean = "src/01_clean.do"`
    Simple(PathBuf),
    /// Sequential: array of task names - `all = ["clean", "analyze"]`
    Sequential(Vec<String>),
    /// Complex: object form for parallel execution or script with options
    Complex(ComplexTask),
}

/// Complex task definition with additional options
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ComplexTask {
    /// List of tasks to run in parallel
    #[serde(default)]
    pub parallel: Option<Vec<String>>,
    /// Script to run (alternative to parallel)
    #[serde(default)]
    pub script: Option<PathBuf>,
    /// Arguments to pass to the script
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// Human-readable description of the task
    #[serde(default)]
    pub description: Option<String>,
}

/// Load configuration from stacy.toml in the project root.
///
/// Returns `None` if the config file doesn't exist.
/// Returns an error if the file exists but is invalid TOML.
///
/// # Arguments
/// * `project_root` - Path to the project root directory
///
/// # Examples
/// ```ignore
/// let config = load_config(Path::new("/path/to/project"))?;
/// if let Some(config) = config {
///     println!("Log dir: {}", config.run.log_dir.display());
/// }
/// ```
pub fn load_config(project_root: &Path) -> Result<Option<Config>> {
    let config_path = project_root.join("stacy.toml");

    if !config_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| {
        Error::Config(format!(
            "Failed to read stacy.toml at {}: {}",
            config_path.display(),
            e
        ))
    })?;

    let config: Config = toml::from_str(&content).map_err(|e| {
        Error::Config(format!(
            "Failed to parse stacy.toml: {}",
            format_toml_error(&e)
        ))
    })?;

    // Validate the loaded config
    validate_config(&config, project_root)?;

    Ok(Some(config))
}

/// Format TOML parse error with line/column information
fn format_toml_error(err: &toml::de::Error) -> String {
    if let Some(span) = err.span() {
        format!("at position {}-{}: {}", span.start, span.end, err.message())
    } else {
        err.message().to_string()
    }
}

/// Validate configuration values.
///
/// Checks that specified paths exist and are valid.
/// Note: log_dir and ado_dir are not validated for existence - they will be created at runtime.
fn validate_config(_config: &Config, _project_root: &Path) -> Result<()> {
    // Note: We don't validate log_dir and ado_dir paths here because:
    // 1. They are relative paths that will be created at runtime
    // 2. The project might be shared and paths may not exist on all systems yet
    // 3. stacy init and stacy run will create these directories as needed
    //
    // Stata binary is NOT in project config - it's in user config (~/.config/stacy/config.toml)
    // or set via STATA_BINARY environment variable.

    Ok(())
}

/// Write configuration to stacy.toml in the project root.
///
/// # Arguments
/// * `config` - The configuration to write
/// * `project_root` - Path to the project root directory
pub fn write_config(config: &Config, project_root: &Path) -> Result<()> {
    let config_path = project_root.join("stacy.toml");
    let content = toml::to_string_pretty(config)
        .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(&config_path, content).map_err(|e| {
        Error::Config(format!(
            "Failed to write stacy.toml at {}: {}",
            config_path.display(),
            e
        ))
    })?;

    Ok(())
}

/// Generate a template stacy.toml with commented defaults.
///
/// This is used by `stacy init` to create a discoverable config file.
pub fn generate_config_template() -> &'static str {
    r#"# stacy project configuration
# See: https://github.com/janfasnacht/stacy

[project]
# name = "my-analysis"
# authors = ["Your Name <you@example.com>"]
# description = "Analysis project"
# url = "https://github.com/user/repo"

[run]
log_dir = "logs"
show_progress = true
# progress_interval_seconds = 10
# max_log_size_mb = 50

# Package dependencies (installed to global cache at ~/.cache/stacy/packages/)
# [packages.dependencies]
# estout = "ssc"
# reghdfe = "github:sergiocorreia/reghdfe"

# Task definitions for `stacy task` command
# [scripts]
# clean = "src/01_clean.do"
# analyze = "src/02_analyze.do"
# all = ["clean", "analyze"]
# outputs = { parallel = ["tables", "figures"] }

# Note: Stata binary path is NOT set here (it's machine-specific).
# Configure it in ~/.config/stacy/config.toml or use $STATA_BINARY env var.
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.run.log_dir, PathBuf::from("logs"));
        assert!(config.run.show_progress);
        assert_eq!(config.run.progress_interval_seconds, 10);
        assert_eq!(config.run.max_log_size_mb, 50);
        assert!(config.packages.dependencies.is_empty());
        assert!(config.project.name.is_none());
        assert!(config.project.authors.is_none());
        assert!(config.project.description.is_none());
        assert!(config.project.url.is_none());
    }

    #[test]
    fn test_load_missing_config() {
        let temp = TempDir::new().unwrap();
        let result = load_config(temp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_empty_config() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("stacy.toml"), "").unwrap();

        let result = load_config(temp.path()).unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        // Should use defaults
        assert_eq!(config.run.log_dir, PathBuf::from("logs"));
        assert!(config.run.show_progress);
    }

    #[test]
    fn test_load_full_config() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[project]
name = "my-analysis"

[run]
log_dir = "output/logs"
show_progress = false
progress_interval_seconds = 5
max_log_size_mb = 100

[packages.dependencies]
estout = "ssc"
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.project.name, Some("my-analysis".to_string()));
        assert_eq!(config.run.log_dir, PathBuf::from("output/logs"));
        assert!(!config.run.show_progress);
        assert_eq!(config.run.progress_interval_seconds, 5);
        assert_eq!(config.run.max_log_size_mb, 100);
        assert!(config.packages.has_package("estout"));
    }

    #[test]
    fn test_load_partial_config() {
        let temp = TempDir::new().unwrap();
        // Only specify some fields, others should use defaults
        let config_content = r#"
[run]
show_progress = false
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        // Specified value
        assert!(!config.run.show_progress);
        // Default values
        assert_eq!(config.run.log_dir, PathBuf::from("logs"));
        assert_eq!(config.run.progress_interval_seconds, 10);
        assert!(config.packages.dependencies.is_empty());
    }

    #[test]
    fn test_load_invalid_toml() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("stacy.toml"), "invalid { toml }}}").unwrap();

        let result = load_config(temp.path());
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(err_msg.contains("Failed to parse stacy.toml"));
    }

    #[test]
    fn test_load_config_with_authors() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[project]
name = "test-project"
authors = ["Alice <alice@example.com>", "Bob <bob@example.com>"]
description = "Test project"
url = "https://github.com/test/project"
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap().unwrap();
        assert_eq!(result.project.name, Some("test-project".to_string()));
        assert_eq!(
            result.project.authors,
            Some(vec![
                "Alice <alice@example.com>".to_string(),
                "Bob <bob@example.com>".to_string()
            ])
        );
        assert_eq!(result.project.description, Some("Test project".to_string()));
        assert_eq!(
            result.project.url,
            Some("https://github.com/test/project".to_string())
        );
    }

    #[test]
    fn test_write_and_read_config() {
        let temp = TempDir::new().unwrap();

        let mut config = Config::default();
        config.project.name = Some("test-project".to_string());
        config.run.log_dir = PathBuf::from("custom/logs");

        write_config(&config, temp.path()).unwrap();

        let loaded = load_config(temp.path()).unwrap().unwrap();
        assert_eq!(loaded.project.name, Some("test-project".to_string()));
        assert_eq!(loaded.run.log_dir, PathBuf::from("custom/logs"));
    }

    #[test]
    fn test_config_template_is_valid_toml() {
        let template = generate_config_template();
        // Should parse without error (comments are ignored)
        let _config: Config = toml::from_str(template).unwrap();
    }

    #[test]
    fn test_load_config_with_simple_scripts() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[scripts]
clean = "src/01_clean.do"
analyze = "src/02_analyze.do"
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap().unwrap();
        assert_eq!(result.scripts.tasks.len(), 2);

        match &result.scripts.tasks["clean"] {
            TaskDef::Simple(path) => assert_eq!(path, &PathBuf::from("src/01_clean.do")),
            _ => panic!("Expected Simple task"),
        }
    }

    #[test]
    fn test_load_config_with_sequential_tasks() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[scripts]
clean = "src/01_clean.do"
analyze = "src/02_analyze.do"
all = ["clean", "analyze"]
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap().unwrap();
        assert_eq!(result.scripts.tasks.len(), 3);

        match &result.scripts.tasks["all"] {
            TaskDef::Sequential(tasks) => {
                assert_eq!(tasks.len(), 2);
                assert_eq!(tasks[0], "clean");
                assert_eq!(tasks[1], "analyze");
            }
            _ => panic!("Expected Sequential task"),
        }
    }

    #[test]
    fn test_load_config_with_parallel_tasks() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[scripts]
tables = "src/03_tables.do"
figures = "src/04_figures.do"
outputs = { parallel = ["tables", "figures"] }
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap().unwrap();
        assert_eq!(result.scripts.tasks.len(), 3);

        match &result.scripts.tasks["outputs"] {
            TaskDef::Complex(complex) => {
                let parallel = complex.parallel.as_ref().unwrap();
                assert_eq!(parallel.len(), 2);
                assert_eq!(parallel[0], "tables");
                assert_eq!(parallel[1], "figures");
            }
            _ => panic!("Expected Complex task with parallel"),
        }
    }

    #[test]
    fn test_load_config_with_complex_script_task() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[scripts]
analyze = { script = "src/02_analyze.do", description = "Run main analysis" }
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap().unwrap();

        match &result.scripts.tasks["analyze"] {
            TaskDef::Complex(complex) => {
                assert_eq!(complex.script, Some(PathBuf::from("src/02_analyze.do")));
                assert_eq!(complex.description, Some("Run main analysis".to_string()));
            }
            _ => panic!("Expected Complex task"),
        }
    }

    #[test]
    fn test_load_config_with_mixed_tasks() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[scripts]
clean = "src/01_clean.do"
analyze = { script = "src/02_analyze.do", description = "Main analysis" }
tables = "src/03_tables.do"
figures = "src/04_figures.do"
outputs = { parallel = ["tables", "figures"] }
pipeline = ["clean", "analyze", "outputs"]
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap().unwrap();
        assert_eq!(result.scripts.tasks.len(), 6);

        // Simple
        assert!(matches!(&result.scripts.tasks["clean"], TaskDef::Simple(_)));
        // Complex with script
        assert!(matches!(
            &result.scripts.tasks["analyze"],
            TaskDef::Complex(_)
        ));
        // Complex with parallel
        assert!(matches!(
            &result.scripts.tasks["outputs"],
            TaskDef::Complex(_)
        ));
        // Sequential
        assert!(matches!(
            &result.scripts.tasks["pipeline"],
            TaskDef::Sequential(_)
        ));
    }

    #[test]
    fn test_package_spec_simple() {
        // Use PackageSpec::simple() helper
        let spec = PackageSpec::simple("ssc");
        assert_eq!(spec.source(), "ssc");
        assert!(spec.version().is_none());
    }

    #[test]
    fn test_package_spec_detailed() {
        let spec = PackageSpec::Detailed {
            source: "ssc".to_string(),
            version: Some("1.0.0".to_string()),
        };
        assert_eq!(spec.source(), "ssc");
        assert_eq!(spec.version(), Some("1.0.0"));
    }

    #[test]
    fn test_package_spec_github() {
        let spec = PackageSpec::simple("github:user/repo");
        assert_eq!(spec.source(), "github:user/repo");
    }

    #[test]
    fn test_packages_section_has_package() {
        let mut section = PackagesSection::default();
        section.add_dependency(
            "estout".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Production,
        );
        section.add_dependency(
            "mdesc".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Dev,
        );
        section.add_dependency(
            "testpkg".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Test,
        );

        assert!(section.has_package("estout"));
        assert!(section.has_package("mdesc"));
        assert!(section.has_package("testpkg"));
        assert!(!section.has_package("reghdfe"));
    }

    #[test]
    fn test_packages_section_is_dev_package() {
        let mut section = PackagesSection::default();
        section.add_dependency(
            "estout".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Production,
        );
        section.add_dependency(
            "mdesc".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Dev,
        );

        assert!(!section.is_dev_package("estout"));
        assert!(section.is_dev_package("mdesc"));
    }

    #[test]
    fn test_packages_section_is_test_package() {
        let mut section = PackagesSection::default();
        section.add_dependency(
            "estout".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Production,
        );
        section.add_dependency(
            "testpkg".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Test,
        );

        assert!(!section.is_test_package("estout"));
        assert!(section.is_test_package("testpkg"));
    }

    #[test]
    fn test_packages_section_get_package_group() {
        let mut section = PackagesSection::default();
        section.add_dependency(
            "estout".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Production,
        );
        section.add_dependency(
            "mdesc".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Dev,
        );
        section.add_dependency(
            "testpkg".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Test,
        );

        assert_eq!(
            section.get_package_group("estout"),
            Some(DependencyGroup::Production)
        );
        assert_eq!(
            section.get_package_group("mdesc"),
            Some(DependencyGroup::Dev)
        );
        assert_eq!(
            section.get_package_group("testpkg"),
            Some(DependencyGroup::Test)
        );
        assert_eq!(section.get_package_group("nonexistent"), None);
    }

    #[test]
    fn test_packages_section_remove_dependency() {
        let mut section = PackagesSection::default();
        section.add_dependency(
            "estout".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Production,
        );
        section.add_dependency(
            "mdesc".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Dev,
        );
        section.add_dependency(
            "testpkg".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Test,
        );

        assert!(section.remove_dependency("estout").is_some());
        assert!(!section.has_package("estout"));

        assert!(section.remove_dependency("mdesc").is_some());
        assert!(!section.has_package("mdesc"));

        assert!(section.remove_dependency("testpkg").is_some());
        assert!(!section.has_package("testpkg"));

        assert!(section.remove_dependency("nonexistent").is_none());
    }

    #[test]
    fn test_packages_section_packages_by_groups() {
        let mut section = PackagesSection::default();
        section.add_dependency(
            "estout".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Production,
        );
        section.add_dependency(
            "mdesc".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Dev,
        );
        section.add_dependency(
            "testpkg".to_string(),
            PackageSpec::simple("ssc"),
            DependencyGroup::Test,
        );

        // Get only production packages
        let prod: Vec<_> = section
            .packages_by_groups(&[DependencyGroup::Production])
            .collect();
        assert_eq!(prod.len(), 1);
        assert_eq!(prod[0].0, "estout");

        // Get production and dev packages
        let prod_dev: Vec<_> = section
            .packages_by_groups(&[DependencyGroup::Production, DependencyGroup::Dev])
            .collect();
        assert_eq!(prod_dev.len(), 2);

        // Get all groups
        let all: Vec<_> = section
            .packages_by_groups(&[
                DependencyGroup::Production,
                DependencyGroup::Dev,
                DependencyGroup::Test,
            ])
            .collect();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_load_config_with_dependencies() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[packages.dependencies]
estout = "ssc"
ftools = "github:sergiocorreia/ftools"

[packages.dev]
mdesc = "ssc"
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap().unwrap();
        assert!(result.packages.has_package("estout"));
        assert!(result.packages.has_package("ftools"));
        assert!(result.packages.has_package("mdesc"));
        assert!(result.packages.is_dev_package("mdesc"));
        assert!(!result.packages.is_dev_package("estout"));
    }

    #[test]
    fn test_load_config_with_test_dependencies() {
        let temp = TempDir::new().unwrap();
        let config_content = r#"
[packages.dependencies]
estout = "ssc"

[packages.dev]
mdesc = "ssc"

[packages.test]
assert = "ssc"
"#;
        fs::write(temp.path().join("stacy.toml"), config_content).unwrap();

        let result = load_config(temp.path()).unwrap().unwrap();
        assert!(result.packages.has_package("estout"));
        assert!(result.packages.has_package("mdesc"));
        assert!(result.packages.has_package("assert"));
        assert!(result.packages.is_test_package("assert"));
        assert!(!result.packages.is_test_package("estout"));
        assert!(!result.packages.is_test_package("mdesc"));
    }
}
