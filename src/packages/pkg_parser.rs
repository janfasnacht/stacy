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
    /// SSC package names declared on the `Requires:` line (supplementary
    /// dependency signal; may be empty)
    pub requires: Vec<String>,
    /// Minimum Stata version declared on the `Requires:` line (e.g. "11.2"),
    /// if stated
    pub stata_version: Option<String>,
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
    let mut requires = Vec::new();
    let mut stata_version = None;

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
                } else if let Some(req) = rest.strip_prefix("Requires:") {
                    for dep in parse_requires(req) {
                        if !requires.contains(&dep) {
                            requires.push(dep);
                        }
                    }
                    if stata_version.is_none() {
                        stata_version = parse_stata_version(req);
                    }
                    description_lines.push(rest.to_string());
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
                // File line. Some manifests list the same file twice (e.g.
                // reghdfe); dedupe here so download, install, and checksum
                // all see one entry per file — verification hashes the
                // unique files on disk (#68).
                let filename = line[1..].trim();
                if !filename.is_empty() && !files.iter().any(|f: &PackageFile| f.name == filename) {
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
        requires,
        stata_version,
    })
}

/// Extract the minimum Stata version from the text following `Requires:`.
///
/// Every SSC `Requires:` line opens with `Stata version <X[.Y]>`. We take the
/// token right after `Stata version` and keep it only if it looks like a
/// numeric version (`11`, `14.2`), ignoring any trailing prose.
fn parse_stata_version(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let after = lower.split("stata version").nth(1)?;
    let token = after.split_whitespace().next()?;
    // Keep clean numeric versions like "11" or "14.2".
    if !token.is_empty()
        && token.chars().all(|c| c.is_ascii_digit() || c == '.')
        && token.chars().any(|c| c.is_ascii_digit())
    {
        Some(token.to_string())
    } else {
        None
    }
}

/// Extract SSC package dependencies from the text following `Requires:`.
///
/// The line has a semi-structured free-text shape, e.g.
/// `Stata version 13 and avar, ftools and reghdfe from SSC (q.v.)`. Only the
/// packages named before `from SSC` are SSC dependencies; version-only lines
/// (`Stata version 14.2`) have no `from SSC` anchor and yield nothing. We take
/// the text up to the first `from SSC`, drop parentheticals, split on commas
/// and whitespace, and keep clean lowercase SSC-style tokens. This is a
/// high-precision supplement — it intentionally ignores rare conditional
/// clauses that trail a second `from SSC` rather than risk false positives.
fn parse_requires(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let head = match lower.find("from ssc") {
        Some(i) => &lower[..i],
        None => return Vec::new(),
    };

    // Drop parenthetical clauses like "(version 9.2 for colorpalette9)".
    let mut cleaned = String::with_capacity(head.len());
    let mut depth = 0i32;
    for c in head.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if depth == 0 => cleaned.push(c),
            _ => {}
        }
    }

    // Words that appear in the free text but are never package names.
    const STOPWORDS: &[&str] = &[
        "stata", "version", "and", "or", "the", "for", "also", "require", "requires", "older",
        "newer", "with", "from", "ssc",
    ];

    let mut deps = Vec::new();
    for token in cleaned.split(|c: char| c == ',' || c == '&' || c.is_whitespace()) {
        let token = token.trim();
        // SSC package names are clean lowercase identifiers.
        if token.len() < 2
            || !token.starts_with(|c: char| c.is_ascii_lowercase())
            || !token
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            || STOPWORDS.contains(&token)
        {
            continue;
        }
        let dep = token.to_string();
        if !deps.contains(&dep) {
            deps.push(dep);
        }
    }
    deps
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_duplicate_file_entries_deduped() {
        // reghdfe's manifest lists some files twice (#68)
        let pkg = "d 'REGHDFE': module to run linear models\n\
f reghdfe.ado\n\
f reghdfe5_parse.ado\n\
f reghdfe.mata\n\
f reghdfe5_parse.ado\n\
f reghdfe.mata\n";
        let manifest = parse_pkg_file(pkg, "reghdfe").unwrap();
        let names: Vec<_> = manifest.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["reghdfe.ado", "reghdfe5_parse.ado", "reghdfe.mata"]
        );
    }

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
    fn test_requires_single_dep() {
        let content = "d 'REGHDFE': linear models\n\
d Requires: Stata version 11.2 and ftools from SSC (q.v.)\n\
f reghdfe.ado\n";
        let manifest = parse_pkg_file(content, "reghdfe").unwrap();
        assert_eq!(manifest.requires, vec!["ftools"]);
    }

    #[test]
    fn test_requires_multiple_deps() {
        let content = "d 'ESI': event study\n\
d Requires: Stata version 13 and avar, ftools and reghdfe from SSC (q.v.)\n\
f esi.ado\n";
        let manifest = parse_pkg_file(content, "eventstudyinteract").unwrap();
        assert_eq!(manifest.requires, vec!["avar", "ftools", "reghdfe"]);
    }

    #[test]
    fn test_requires_version_only_has_no_deps() {
        // No `from SSC` anchor => Stata-version-only requirement, no package dep.
        let content = "d 'ESTOUT': tables\n\
d Requires: Stata version 8.2\n\
f estout.ado\n";
        let manifest = parse_pkg_file(content, "estout").unwrap();
        assert!(manifest.requires.is_empty());
    }

    #[test]
    fn test_requires_strips_parenthetical_version_pin() {
        let content = "d 'GRSTYLE': schemes\n\
d Requires: Stata version 14.2 and colrspace from SSC (q.v.); (version 9.2 for colorpalette9)\n\
f grstyle.ado\n";
        let manifest = parse_pkg_file(content, "grstyle").unwrap();
        // colorpalette9 lives in a parenthetical and after `from SSC`; only
        // colrspace is a real declared dep.
        assert_eq!(manifest.requires, vec!["colrspace"]);
    }

    #[test]
    fn test_requires_ignores_conditional_trailing_clause() {
        // ftools: a second `from SSC` trails a conditional clause with English
        // words ("older", "also require"). We keep only the first segment.
        let content = "d 'FTOOLS': fast tools\n\
d Requires: Stata version 9.2 and moremata from SSC; Stata 12 or older also require boottest from SSC\n\
f ftools.ado\n";
        let manifest = parse_pkg_file(content, "ftools").unwrap();
        assert_eq!(manifest.requires, vec!["moremata"]);
    }

    #[test]
    fn test_no_requires_line() {
        let content = "d example\nf example.ado\n";
        let manifest = parse_pkg_file(content, "example").unwrap();
        assert!(manifest.requires.is_empty());
        assert!(manifest.stata_version.is_none());
    }

    #[test]
    fn test_stata_version_captured() {
        let content = "d 'REGHDFE': linear models\n\
d Requires: Stata version 11.2 and ftools from SSC (q.v.)\n\
f reghdfe.ado\n";
        let manifest = parse_pkg_file(content, "reghdfe").unwrap();
        assert_eq!(manifest.stata_version.as_deref(), Some("11.2"));
    }

    #[test]
    fn test_stata_version_integer_and_version_only_line() {
        let content = "d 'ESTOUT': tables\n\
d Requires: Stata version 8\n\
f estout.ado\n";
        let manifest = parse_pkg_file(content, "estout").unwrap();
        assert_eq!(manifest.stata_version.as_deref(), Some("8"));
        assert!(manifest.requires.is_empty());
    }

    #[test]
    fn test_stata_version_ignores_trailing_prose() {
        // labutil: "Stata version 7.0 (version 8 for labvalcombine)."
        let content = "d 'LABUTIL': label utilities\n\
d Requires: Stata version 7.0 (version 8 for labvalcombine).\n\
f labutil.ado\n";
        let manifest = parse_pkg_file(content, "labutil").unwrap();
        assert_eq!(manifest.stata_version.as_deref(), Some("7.0"));
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
