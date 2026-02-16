//! User-level configuration (~/.config/stacy/config.toml)
//!
//! Machine-specific settings that should NOT be committed to version control.
//! This includes paths like stata_binary that vary between machines.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// User configuration loaded from ~/.config/stacy/config.toml
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct UserConfig {
    /// Stata binary path (machine-specific)
    pub stata_binary: Option<String>,
    /// Whether to check for updates on startup (default: true)
    pub update_check: Option<bool>,
}

/// Get the user config directory path.
///
/// Returns `~/.config/stacy/` on Unix and `%APPDATA%\stacy\` on Windows.
pub fn get_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("stacy"))
}

/// Get the user config file path.
///
/// Returns `~/.config/stacy/config.toml` on Unix.
pub fn get_config_path() -> Option<PathBuf> {
    get_config_dir().map(|p| p.join("config.toml"))
}

/// Load user configuration from ~/.config/stacy/config.toml
///
/// Returns `None` if the config file doesn't exist.
/// Returns an error if the file exists but is invalid TOML.
pub fn load_user_config() -> Result<Option<UserConfig>> {
    let config_path = match get_config_path() {
        Some(path) => path,
        None => return Ok(None),
    };

    if !config_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| {
        Error::Config(format!(
            "Failed to read user config at {}: {}",
            config_path.display(),
            e
        ))
    })?;

    let config: UserConfig = toml::from_str(&content).map_err(|e| {
        Error::Config(format!(
            "Failed to parse user config at {}: {}",
            config_path.display(),
            e
        ))
    })?;

    // Validate stata_binary if specified
    if let Some(ref binary) = config.stata_binary {
        let binary_path = Path::new(binary);
        if !binary_path.exists() {
            return Err(Error::Config(format!(
                "Stata binary specified in {} not found: {}\n\
                 Hint: Update the path or remove it to use auto-detection",
                config_path.display(),
                binary
            )));
        }
    }

    Ok(Some(config))
}

/// Save user configuration to ~/.config/stacy/config.toml
///
/// Creates the config directory if it doesn't exist.
pub fn save_user_config(config: &UserConfig) -> Result<()> {
    let config_dir = get_config_dir()
        .ok_or_else(|| Error::Config("Unable to determine user config directory".to_string()))?;

    // Create config directory if needed
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).map_err(|e| {
            Error::Config(format!(
                "Failed to create config directory {}: {}",
                config_dir.display(),
                e
            ))
        })?;
    }

    let config_path = config_dir.join("config.toml");
    let content = generate_user_config_content(config);

    std::fs::write(&config_path, content).map_err(|e| {
        Error::Config(format!(
            "Failed to write user config at {}: {}",
            config_path.display(),
            e
        ))
    })?;

    Ok(())
}

/// Generate user config file content with comments
fn generate_user_config_content(config: &UserConfig) -> String {
    let mut content = String::from(
        "# stacy user configuration (machine-specific, not committed to version control)\n\
         # See: https://github.com/janfasnacht/stacy\n\n",
    );

    if let Some(ref binary) = config.stata_binary {
        content.push_str(&format!("stata_binary = \"{}\"\n", binary));
    } else {
        content.push_str("# stata_binary = \"/path/to/stata-mp\"\n");
    }

    content.push('\n');
    content.push_str("# Check for updates on startup (set to false to disable)\n");
    if let Some(update_check) = config.update_check {
        content.push_str(&format!("update_check = {}\n", update_check));
    } else {
        content.push_str("# update_check = false\n");
    }

    content
}

/// Generate a template for the user config file
pub fn generate_user_config_template() -> &'static str {
    r#"# stacy user configuration (machine-specific, not committed to version control)
# See: https://github.com/janfasnacht/stacy

# Path to Stata binary (uncomment and set to your Stata installation)
# stata_binary = "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp"  # macOS
# stata_binary = "/usr/local/stata18/stata-mp"  # Linux
# stata_binary = "C:\\Program Files\\Stata18\\StataMP-64.exe"  # Windows

# Check for updates on startup (set to false to disable)
# update_check = false
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_user_config() {
        let config = UserConfig::default();
        assert!(config.stata_binary.is_none());
        assert!(config.update_check.is_none());
    }

    #[test]
    fn test_get_config_dir() {
        // Should return something on most systems
        let dir = get_config_dir();
        // Note: May be None in some test environments
        if let Some(d) = dir {
            assert!(d.ends_with("stacy"));
        }
    }

    #[test]
    fn test_generate_user_config_content_empty() {
        let config = UserConfig::default();
        let content = generate_user_config_content(&config);
        assert!(content.contains("# stata_binary"));
        assert!(content.contains("machine-specific"));
    }

    #[test]
    fn test_generate_user_config_content_with_binary() {
        let config = UserConfig {
            stata_binary: Some("/usr/local/stata/stata-mp".to_string()),
            update_check: None,
        };
        let content = generate_user_config_content(&config);
        assert!(content.contains("stata_binary = \"/usr/local/stata/stata-mp\""));
    }

    #[test]
    fn test_generate_user_config_content_with_update_check() {
        let config = UserConfig {
            stata_binary: None,
            update_check: Some(false),
        };
        let content = generate_user_config_content(&config);
        assert!(content.contains("update_check = false"));
    }

    #[test]
    fn test_parse_update_check_field() {
        let toml_str = r#"update_check = false"#;
        let config: UserConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.update_check, Some(false));
    }

    #[test]
    fn test_template_is_valid_toml() {
        let template = generate_user_config_template();
        // Should parse without error (comments are ignored)
        let _config: UserConfig = toml::from_str(template).unwrap();
    }
}
