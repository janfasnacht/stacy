//! Schema parser for commands.toml
//!
//! Parses the single source of truth and provides typed access
//! to command definitions, arguments, and return values.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Root schema structure
///
/// Note: Some fields are parsed for schema completeness but not yet used
/// in code generation. They document the full contract.
#[derive(Debug, Deserialize)]
pub struct Schema {
    pub meta: Meta,
    pub commands: HashMap<String, Command>,
    /// Type mapping documentation (for reference)
    #[serde(default)]
    #[allow(dead_code)]
    pub type_mapping: Option<toml::Value>,
    /// Global exit codes
    #[serde(default)]
    pub exit_codes: HashMap<String, ExitCodeDef>,
}

/// Exit code definition
#[derive(Debug, Deserialize)]
pub struct ExitCodeDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub r_codes: Option<String>,
}

/// Schema metadata
#[derive(Debug, Deserialize)]
pub struct Meta {
    /// Schema version (for compatibility tracking)
    #[allow(dead_code)]
    pub version: String,
    /// Minimum Stata version required
    #[allow(dead_code)]
    pub stata_minimum: String,
    pub description: String,
}

/// Command definition
#[derive(Debug, Deserialize)]
pub struct Command {
    pub description: String,
    /// Command category (execution, utility, packages, etc.)
    pub category: String,
    pub stata_command: String,
    /// Extended description for documentation (optional)
    #[serde(default)]
    pub long_description: Option<String>,
    /// Related commands and docs for "See Also" section
    #[serde(default)]
    pub see_also: Option<Vec<String>>,
    #[serde(default)]
    pub args: HashMap<String, Argument>,
    #[serde(default)]
    pub returns: HashMap<String, ReturnValue>,
    /// Per-command exit codes documentation
    #[serde(default)]
    pub exit_codes: HashMap<String, String>,
    /// Examples for documentation
    #[serde(default)]
    pub examples: Vec<Example>,
}

/// Example usage for documentation
#[derive(Debug, Deserialize)]
pub struct Example {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub commands: Vec<String>,
    #[serde(default)]
    pub output: Option<String>,
}

/// Command argument definition
#[derive(Debug, Deserialize)]
pub struct Argument {
    #[serde(rename = "type")]
    pub arg_type: String,
    #[serde(default)]
    pub positional: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub required_unless: Option<String>,
    /// Short option flag (for CLI, not currently used in Stata)
    #[serde(default)]
    #[allow(dead_code)]
    pub short: Option<String>,
    /// Conflicting argument (for CLI validation)
    #[serde(default)]
    #[allow(dead_code)]
    pub conflicts_with: Option<String>,
    pub description: String,
    #[serde(default)]
    pub stata_option: Option<String>,
}

/// Return value definition
#[derive(Debug, Deserialize)]
pub struct ReturnValue {
    #[serde(rename = "type")]
    pub ret_type: String,
    /// JSON path for extraction (documents where to find value)
    #[allow(dead_code)]
    pub json_path: String,
    pub stata_type: String,
    #[serde(default)]
    pub array_handling: Option<String>,
    pub description: String,
}

impl Schema {
    /// Load schema from commands.toml file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read schema file: {}", path.display()))?;

        let schema: Schema =
            toml::from_str(&content).with_context(|| "Failed to parse schema TOML")?;

        Ok(schema)
    }

    /// Get commands sorted by name for consistent output
    pub fn commands_sorted(&self) -> Vec<(&String, &Command)> {
        let mut commands: Vec<_> = self.commands.iter().collect();
        commands.sort_by_key(|(name, _)| *name);
        commands
    }
}

impl Command {
    /// Get arguments sorted: positional first, then alphabetically
    pub fn args_sorted(&self) -> Vec<(&String, &Argument)> {
        let mut args: Vec<_> = self.args.iter().collect();
        args.sort_by(|(name_a, arg_a), (name_b, arg_b)| {
            // Positional args first
            match (arg_a.positional, arg_b.positional) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => name_a.cmp(name_b),
            }
        });
        args
    }

    /// Get return values sorted: scalars first, then locals
    pub fn returns_sorted(&self) -> Vec<(&String, &ReturnValue)> {
        let mut returns: Vec<_> = self.returns.iter().collect();
        returns.sort_by(|(name_a, ret_a), (name_b, ret_b)| {
            // Scalars before locals
            match (ret_a.stata_type.as_str(), ret_b.stata_type.as_str()) {
                ("scalar", "local") => std::cmp::Ordering::Less,
                ("local", "scalar") => std::cmp::Ordering::Greater,
                _ => name_a.cmp(name_b),
            }
        });
        returns
    }

    /// Get Stata syntax options (non-positional args with stata_option)
    pub fn stata_options(&self) -> Vec<(&String, &Argument)> {
        self.args_sorted()
            .into_iter()
            .filter(|(name, arg)| !arg.positional && *name != "json")
            .collect()
    }

    /// Get positional arguments
    pub fn positional_args(&self) -> Vec<(&String, &Argument)> {
        self.args_sorted()
            .into_iter()
            .filter(|(_, arg)| arg.positional)
            .collect()
    }
}

impl Argument {
    /// Generate Stata syntax fragment for this argument
    #[allow(dead_code)]
    pub fn to_stata_syntax(&self) -> Option<String> {
        if self.positional {
            return None;
        }

        self.stata_option.as_ref().map(|opt| {
            if self.arg_type == "bool" {
                opt.clone()
            } else {
                opt.clone()
            }
        })
    }
}

impl ReturnValue {
    /// Is this a scalar return value?
    pub fn is_scalar(&self) -> bool {
        self.stata_type == "scalar"
    }

    /// Is this a local (string) return value?
    pub fn is_local(&self) -> bool {
        self.stata_type == "local"
    }

    /// Get the JSON extraction function to use
    #[allow(dead_code)]
    pub fn json_extractor(&self) -> &'static str {
        match self.array_handling.as_deref() {
            Some("count") => "_stacy_count_array",
            _ => match self.ret_type.as_str() {
                "bool" => "_stacy_extract_bool",
                "int" | "float" => "_stacy_extract_number",
                "string" | "path" => "_stacy_extract_string",
                _ => "_stacy_extract_string",
            },
        }
    }

    /// Get the scalar name used internally during JSON parsing
    pub fn internal_scalar_name(&self, name: &str) -> String {
        format!("_stacy_json_{}", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_schema() {
        let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("schema/commands.toml");

        let schema = Schema::load(&schema_path).expect("Failed to load schema");

        assert_eq!(schema.meta.version, "0.1.0");
        assert!(schema.commands.contains_key("run"));
        assert!(schema.commands.contains_key("doctor"));
    }
}
