//! Local directory package source
//!
//! Scans a local directory for Stata package files (.ado, .sthlp, etc.),
//! computes checksums, and returns them for caching and tracking.

use crate::error::{Error, Result};
use crate::packages::ssc::{calculate_combined_checksum, calculate_sha256, DownloadedFile};

/// Stata file extensions recognized when scanning a local directory
const STATA_EXTENSIONS: &[&str] = &["ado", "sthlp", "hlp", "do", "mata", "mlib", "dlg", "pkg"];

/// Result of scanning a local directory for package files
#[derive(Debug)]
pub struct LocalPackageDownload {
    /// Package name
    pub name: String,
    /// Path to source directory
    pub path: String,
    /// Downloaded files with checksums
    pub files: Vec<DownloadedFile>,
    /// Combined checksum of all files
    pub package_checksum: String,
}

/// Scan a local directory for Stata package files
///
/// # Arguments
/// * `name` - Package name (for error messages)
/// * `dir` - Directory to scan
///
/// # Returns
/// A `LocalPackageDownload` with all found Stata files
pub fn scan_local_directory(name: &str, dir: &std::path::Path) -> Result<LocalPackageDownload> {
    if !dir.exists() {
        return Err(Error::Config(format!(
            "Local source directory not found: {}",
            dir.display()
        )));
    }

    if !dir.is_dir() {
        return Err(Error::Config(format!(
            "Local source path is not a directory: {}",
            dir.display()
        )));
    }

    let entries = std::fs::read_dir(dir).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read directory {}: {}", dir.display(), e),
        ))
    })?;

    let mut files = Vec::new();
    let mut checksums = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read directory entry: {}", e),
            ))
        })?;

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Check if the file has a recognized Stata extension
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !STATA_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let content = std::fs::read(&path).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read {}: {}", path.display(), e),
            ))
        })?;

        let checksum = calculate_sha256(&content);
        checksums.push(checksum.clone());

        files.push(DownloadedFile {
            name: filename,
            content,
            checksum,
        });
    }

    // Require at least one .ado file
    let has_ado = files.iter().any(|f| f.name.ends_with(".ado"));
    if !has_ado {
        return Err(Error::Config(format!(
            "No .ado files found in local directory: {}",
            dir.display()
        )));
    }

    // The directory must actually hold the package that was asked for (#100).
    // Without this, `stacy add badname --source local:<dir>` installed whatever
    // the directory happened to contain and reported success.
    let wanted = format!("{}.ado", name.to_lowercase());
    if !files
        .iter()
        .any(|f| f.name.to_lowercase() == wanted.as_str())
    {
        let found: Vec<&str> = files
            .iter()
            .filter(|f| f.name.ends_with(".ado"))
            .map(|f| f.name.as_str())
            .collect();
        return Err(Error::Config(format!(
            "Package '{}' not found in local directory: {}\n  \
             expected {}, found: {}\n  \
             hint: use the name of the .ado file, or point --source at the directory that holds it",
            name,
            dir.display(),
            wanted,
            found.join(", ")
        )));
    }

    let package_checksum = calculate_combined_checksum(&checksums);

    Ok(LocalPackageDownload {
        name: name.to_string(),
        path: dir.to_string_lossy().to_string(),
        files,
        package_checksum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_local_directory() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("mypkg.ado"), "program define mypkg\nend").unwrap();
        fs::write(temp.path().join("mypkg.sthlp"), "{title:help}").unwrap();
        fs::write(temp.path().join("readme.txt"), "not a stata file").unwrap();

        let result = scan_local_directory("mypkg", temp.path()).unwrap();
        assert_eq!(result.name, "mypkg");
        assert_eq!(result.files.len(), 2);
        assert!(!result.package_checksum.is_empty());
    }

    #[test]
    fn test_scan_empty_directory_errors() {
        let temp = TempDir::new().unwrap();
        let result = scan_local_directory("mypkg", temp.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No .ado files"));
    }

    #[test]
    fn test_scan_nonexistent_directory_errors() {
        let result = scan_local_directory("mypkg", std::path::Path::new("/nonexistent/path"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_scan_rejects_name_not_in_directory() {
        // #100: the requested name has to be the package that's actually there.
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("othername.ado"),
            "program define othername\nend",
        )
        .unwrap();

        let err = scan_local_directory("badname", temp.path())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("badname"),
            "error should name the request: {}",
            err
        );
        assert!(
            err.contains("othername.ado"),
            "error should list what the directory holds: {}",
            err
        );
    }

    #[test]
    fn test_scan_accepts_case_insensitive_name_match() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("MyPkg.ado"), "program define mypkg\nend").unwrap();

        let result = scan_local_directory("mypkg", temp.path()).unwrap();
        assert_eq!(result.files.len(), 1);
    }

    #[test]
    fn test_scan_accepts_extra_files_alongside_the_named_package() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("mypkg.ado"), "program define mypkg\nend").unwrap();
        fs::write(temp.path().join("helper.ado"), "program define helper\nend").unwrap();

        let result = scan_local_directory("mypkg", temp.path()).unwrap();
        assert_eq!(result.files.len(), 2);
    }

    #[test]
    fn test_scan_directory_without_ado_errors() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("mypkg.sthlp"), "help content").unwrap();
        fs::write(temp.path().join("mypkg.do"), "do content").unwrap();

        let result = scan_local_directory("mypkg", temp.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No .ado files"));
    }
}
