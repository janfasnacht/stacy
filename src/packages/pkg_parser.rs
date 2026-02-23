//! Parse Stata .pkg manifest files
//!
//! The .pkg format describes a Stata package:
//! - `d` lines: package description
//! - `D` lines: package description (alternative)
//! - `f` lines: file to install
//! - `F` lines: file to install (alternative)
//! - `h` lines: help file

use crate::error::{Error, Result};

/// File type in a package
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    /// Stata ado file (.ado)
    Ado,
    /// Stata help file (.sthlp or .hlp)
    Help,
    /// Mata library (.mlib)
    MataLib,
    /// Mata source (.mata)
    Mata,
    /// Dialog file (.dlg)
    Dialog,
    /// Scheme file (.scheme)
    Scheme,
    /// Style file (.style)
    Style,
    /// Other file type
    Other(String),
}

impl FileType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "ado" => FileType::Ado,
            "sthlp" | "hlp" => FileType::Help,
            "mlib" => FileType::MataLib,
            "mata" => FileType::Mata,
            "dlg" => FileType::Dialog,
            "scheme" => FileType::Scheme,
            "style" => FileType::Style,
            other => FileType::Other(other.to_string()),
        }
    }
}

/// A file listed in a package manifest
#[derive(Debug, Clone)]
pub struct PackageFile {
    /// File name (e.g., "rdrobust.ado")
    pub name: String,
    /// File type derived from extension
    pub file_type: FileType,
}

/// Parsed package manifest
#[derive(Debug, Clone)]
pub struct PackageManifest {
    /// Package name
    pub name: String,
    /// Package title/description
    pub title: String,
    /// Package author (if specified)
    pub author: Option<String>,
    /// Distribution date (if specified)
    pub distribution_date: Option<String>,
    /// Files included in the package
    pub files: Vec<PackageFile>,
    /// Raw description lines
    pub description_lines: Vec<String>,
}

impl PackageManifest {
    /// Get all ado files
    pub fn ado_files(&self) -> Vec<&PackageFile> {
        self.files
            .iter()
            .filter(|f| matches!(f.file_type, FileType::Ado))
            .collect()
    }

    /// Get all help files
    pub fn help_files(&self) -> Vec<&PackageFile> {
        self.files
            .iter()
            .filter(|f| matches!(f.file_type, FileType::Help))
            .collect()
    }
}

/// Parse a .pkg file content into a manifest
///
/// # Arguments
/// * `content` - The raw content of the .pkg file
/// * `package_name` - The package name (for fallback title)
///
/// # Returns
/// A parsed PackageManifest
///
/// # Example
/// ```
/// use stacy::packages::pkg_parser::parse_pkg_file;
///
/// let content = r#"d 'EXAMPLE': example package
/// d Distribution-Date: 20240101
/// f example.ado
/// f example.sthlp
/// "#;
///
/// let manifest = parse_pkg_file(content, "example").unwrap();
/// assert_eq!(manifest.files.len(), 2);
/// ```
pub fn parse_pkg_file(content: &str, package_name: &str) -> Result<PackageManifest> {
    let mut title = String::new();
    let mut author = None;
    let mut distribution_date = None;
    let mut files = Vec::new();
    let mut description_lines = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Parse line by first character
        let first_char = line.chars().next().unwrap_or(' ');

        match first_char {
            'd' | 'D' => {
                // Description line
                let rest = line[1..].trim();

                // Check for special metadata
                if rest.starts_with("Distribution-Date:") {
                    distribution_date = Some(
                        rest.trim_start_matches("Distribution-Date:")
                            .trim()
                            .to_string(),
                    );
                } else if rest.starts_with("Author:") || rest.starts_with("Authors:") {
                    author = Some(
                        rest.trim_start_matches("Author:")
                            .trim_start_matches("Authors:")
                            .trim()
                            .to_string(),
                    );
                } else if !rest.is_empty() {
                    // First non-empty description line is the title
                    if title.is_empty() {
                        // Handle format: 'PKGNAME': description
                        if let Some(stripped) = rest.strip_prefix('\'') {
                            if let Some(end_quote) = stripped.find('\'') {
                                let after = stripped[end_quote + 1..].trim();
                                if let Some(after_colon) = after.strip_prefix(':') {
                                    title = after_colon.trim().to_string();
                                } else {
                                    title = rest.to_string();
                                }
                            } else {
                                title = rest.to_string();
                            }
                        } else {
                            title = rest.to_string();
                        }
                    }
                    description_lines.push(rest.to_string());
                }
            }
            'f' | 'F' => {
                // File line
                let filename = line[1..].trim();
                if !filename.is_empty() {
                    let file_type = filename
                        .rsplit('.')
                        .next()
                        .map(FileType::from_extension)
                        .unwrap_or(FileType::Other(String::new()));

                    files.push(PackageFile {
                        name: filename.to_string(),
                        file_type,
                    });
                }
            }
            'h' | 'H' => {
                // Help file line (older format)
                let filename = line[1..].trim();
                if !filename.is_empty() {
                    // Add .sthlp extension if not present
                    let filename = if !filename.contains('.') {
                        format!("{}.sthlp", filename)
                    } else {
                        filename.to_string()
                    };

                    files.push(PackageFile {
                        name: filename,
                        file_type: FileType::Help,
                    });
                }
            }
            '*' => {
                // Comment line, ignore
            }
            'v' | 'V' => {
                // Version line (v 3), ignore
            }
            'p' | 'P' => {
                // Package name line, ignore (we already have it)
            }
            _ => {
                // Unknown line type, ignore
            }
        }
    }

    // Use package name as fallback title
    if title.is_empty() {
        title = package_name.to_string();
    }

    // Validate we have at least one file
    if files.is_empty() {
        return Err(Error::Config(format!(
            "Package {} has no files listed in manifest",
            package_name
        )));
    }

    Ok(PackageManifest {
        name: package_name.to_string(),
        title,
        author,
        distribution_date,
        files,
        description_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pkg() {
        let content = r#"
d 'EXAMPLE': module to do something useful
d
d Distribution-Date: 20240115
d Author: Jane Doe
d
f example.ado
f example.sthlp
"#;
        let manifest = parse_pkg_file(content, "example").unwrap();

        assert_eq!(manifest.name, "example");
        assert_eq!(manifest.title, "module to do something useful");
        assert_eq!(manifest.author, Some("Jane Doe".to_string()));
        assert_eq!(manifest.distribution_date, Some("20240115".to_string()));
        assert_eq!(manifest.files.len(), 2);
    }

    #[test]
    fn test_parse_pkg_file_types() {
        let content = r#"
d example
f example.ado
f example.sthlp
f example.mata
f example.mlib
f example.dlg
"#;
        let manifest = parse_pkg_file(content, "example").unwrap();

        assert_eq!(manifest.files.len(), 5);
        assert!(matches!(manifest.files[0].file_type, FileType::Ado));
        assert!(matches!(manifest.files[1].file_type, FileType::Help));
        assert!(matches!(manifest.files[2].file_type, FileType::Mata));
        assert!(matches!(manifest.files[3].file_type, FileType::MataLib));
        assert!(matches!(manifest.files[4].file_type, FileType::Dialog));
    }

    #[test]
    fn test_parse_pkg_uppercase() {
        let content = r#"
D EXAMPLE: a package
F example.ado
F example.sthlp
"#;
        let manifest = parse_pkg_file(content, "example").unwrap();
        assert_eq!(manifest.files.len(), 2);
    }

    #[test]
    fn test_parse_pkg_h_line() {
        let content = r#"
d example
f example.ado
h example
"#;
        let manifest = parse_pkg_file(content, "example").unwrap();
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.files[1].name, "example.sthlp");
    }

    #[test]
    fn test_parse_pkg_no_files_error() {
        let content = r#"
d example package with no files
"#;
        let result = parse_pkg_file(content, "example");
        assert!(result.is_err());
    }

    #[test]
    fn test_ado_files_filter() {
        let content = r#"
d example
f one.ado
f two.ado
f one.sthlp
"#;
        let manifest = parse_pkg_file(content, "example").unwrap();
        let ado_files = manifest.ado_files();
        assert_eq!(ado_files.len(), 2);
    }

    #[test]
    fn test_help_files_filter() {
        let content = r#"
d example
f one.ado
f one.sthlp
f two.hlp
"#;
        let manifest = parse_pkg_file(content, "example").unwrap();
        let help_files = manifest.help_files();
        assert_eq!(help_files.len(), 2);
    }

    #[test]
    fn test_fallback_title() {
        let content = r#"
d
f example.ado
"#;
        let manifest = parse_pkg_file(content, "mypackage").unwrap();
        assert_eq!(manifest.title, "mypackage");
    }
}
