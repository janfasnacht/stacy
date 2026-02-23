//! SSC (Statistical Software Components) package source
//!
//! Downloads packages from the SSC archive hosted at Boston College.
//! Falls back to GitHub mirror if the primary server is unreachable.
//! SSC is Stata's primary community package repository.

use crate::error::{Error, Result};
use crate::packages::http::StacyHttpClient;
use crate::packages::pkg_parser::{parse_pkg_file, PackageManifest};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Base URL for SSC archive (primary)
///
/// Must use HTTP, not HTTPS. The SSC server at Boston College (fmwww.bc.edu)
/// does not support TLS — port 443 serves plain HTTP, causing TLS handshake
/// failures with any HTTPS client. Stata's own `ssc install` uses plain HTTP.
const SSC_BASE_URL: &str = "http://fmwww.bc.edu/repec/bocode";

/// GitHub mirror URL (fallback)
/// See: <https://github.com/labordynamicsinstitute/ssc-mirror>
const SSC_MIRROR_URL: &str =
    "https://raw.githubusercontent.com/labordynamicsinstitute/ssc-mirror/releases/fmwww.bc.edu/repec/bocode";

/// A downloaded file with its content and checksum
#[derive(Debug, Clone)]
pub struct DownloadedFile {
    /// File name
    pub name: String,
    /// File content as bytes
    pub content: Vec<u8>,
    /// SHA256 checksum of content
    pub checksum: String,
}

/// Result of downloading a package from SSC
#[derive(Debug)]
pub struct PackageDownload {
    /// Package name
    pub name: String,
    /// Parsed manifest
    pub manifest: PackageManifest,
    /// Downloaded files with checksums
    pub files: Vec<DownloadedFile>,
    /// Combined checksum of all files (for lockfile)
    pub package_checksum: String,
    /// Whether the download came from the GitHub mirror (not the primary SSC server)
    pub from_mirror: bool,
}

/// SSC package downloader
pub struct SscDownloader {
    client: StacyHttpClient,
}

impl Default for SscDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl SscDownloader {
    /// Create a new SSC downloader
    pub fn new() -> Self {
        Self {
            client: StacyHttpClient::new(),
        }
    }

    /// Get the SSC URL for a package (primary server)
    ///
    /// Packages are organized by first letter:
    /// - `rdrobust` -> `http://fmwww.bc.edu/repec/bocode/r/`
    pub fn get_package_url(name: &str) -> String {
        let first_char = name
            .chars()
            .next()
            .unwrap_or('_')
            .to_lowercase()
            .next()
            .unwrap_or('_');
        format!("{}/{}/", SSC_BASE_URL, first_char)
    }

    /// Get the GitHub mirror URL for a package (fallback)
    fn get_mirror_url(name: &str) -> String {
        let first_char = name
            .chars()
            .next()
            .unwrap_or('_')
            .to_lowercase()
            .next()
            .unwrap_or('_');
        format!("{}/{}/", SSC_MIRROR_URL, first_char)
    }

    /// Download a package from SSC (tries primary, then mirror)
    ///
    /// Error handling distinguishes three failure scenarios:
    /// 1. Primary 404 → check mirror to distinguish "package doesn't exist" from "mirror gap"
    /// 2. Primary connection error → try mirror as fallback
    /// 3. Both unreachable → suggest checking internet connection
    ///
    /// # Arguments
    /// * `name` - Package name (e.g., "rdrobust", "estout")
    ///
    /// # Returns
    /// A PackageDownload containing manifest and all files
    pub fn download_package(&self, name: &str) -> Result<PackageDownload> {
        let name = name.to_lowercase();

        // Try primary SSC server first
        match self.download_package_from_url(&name, Self::get_package_url(&name)) {
            Ok(mut download) => {
                download.from_mirror = false;
                Ok(download)
            }
            Err(primary_err) => {
                if is_connection_error(&primary_err) {
                    // Primary connection error → try mirror as fallback
                    eprintln!("Primary SSC server unreachable, trying GitHub mirror...");
                    match self.download_package_from_url(&name, Self::get_mirror_url(&name)) {
                        Ok(mut download) => {
                            download.from_mirror = true;
                            Ok(download)
                        }
                        Err(mirror_err) => {
                            if is_not_found_error(&mirror_err) {
                                Err(Error::Network(format!(
                                    "SSC server is unreachable and the GitHub mirror does not have '{}'. \
                                     This may be a recently published package not yet mirrored, \
                                     or the package name may be misspelled.",
                                    name
                                )))
                            } else if is_connection_error(&mirror_err) {
                                Err(Error::Network(
                                    "Both SSC and its GitHub mirror are unreachable. \
                                     Check your internet connection."
                                        .to_string(),
                                ))
                            } else {
                                Err(Error::Network(format!(
                                    "Both SSC servers failed. Primary: {}. Mirror: {}",
                                    primary_err, mirror_err
                                )))
                            }
                        }
                    }
                } else if is_not_found_error(&primary_err) {
                    // Primary 404 → check mirror to see if it's a mirror gap
                    match self.download_package_from_url(&name, Self::get_mirror_url(&name)) {
                        Ok(mut download) => {
                            eprintln!(
                                "  note: SSC primary is missing this package; installed from GitHub mirror"
                            );
                            download.from_mirror = true;
                            Ok(download)
                        }
                        Err(_) => Err(Error::Config(format!(
                            "Package '{}' not found on SSC. \
                                 Check spelling or verify the package exists at \
                                 https://ideas.repec.org/s/boc/bocode.html",
                            name
                        ))),
                    }
                } else {
                    Err(primary_err)
                }
            }
        }
    }

    /// Download package from a specific base URL
    fn download_package_from_url(&self, name: &str, base_url: String) -> Result<PackageDownload> {
        // Download .pkg manifest
        let pkg_url = format!("{}{}.pkg", base_url, name);
        let pkg_content = self.download_text(&pkg_url)?;

        // Parse manifest
        let manifest = parse_pkg_file(&pkg_content, name)?;

        // Download all files
        let mut files = Vec::new();
        let mut checksums = Vec::new();

        for pkg_file in &manifest.files {
            let file_url = format!("{}{}", base_url, pkg_file.name);
            let content = self.download_bytes(&file_url)?;
            let checksum = calculate_sha256(&content);

            checksums.push(checksum.clone());
            files.push(DownloadedFile {
                name: pkg_file.name.clone(),
                content,
                checksum,
            });
        }

        // Calculate combined package checksum
        let package_checksum = calculate_combined_checksum(&checksums);

        Ok(PackageDownload {
            name: name.to_string(),
            manifest,
            files,
            package_checksum,
            from_mirror: false, // Caller overrides after return
        })
    }

    /// Check if a package exists on SSC
    pub fn package_exists(&self, name: &str) -> Result<bool> {
        let name = name.to_lowercase();
        let base_url = Self::get_package_url(&name);
        let pkg_url = format!("{}{}.pkg", base_url, name);

        match self.client.inner().head(&pkg_url).send() {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                if e.is_timeout() || e.is_connect() {
                    Err(Error::Network(format!("Network error: {}", e)))
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// Get package manifest without downloading files
    pub fn get_manifest(&self, name: &str) -> Result<PackageManifest> {
        let name = name.to_lowercase();
        let base_url = Self::get_package_url(&name);
        let pkg_url = format!("{}{}.pkg", base_url, name);

        let pkg_content = self.download_text(&pkg_url)?;
        parse_pkg_file(&pkg_content, &name)
    }

    fn download_text(&self, url: &str) -> Result<String> {
        self.client.download_text(url)
    }

    fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.client.download_bytes(url)
    }
}

/// Calculate SHA256 checksum of data
pub fn calculate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Calculate combined checksum from multiple checksums
///
/// Sorts checksums before hashing so the result is independent of file order.
pub fn calculate_combined_checksum(checksums: &[String]) -> String {
    let mut sorted = checksums.to_vec();
    sorted.sort();
    let mut hasher = Sha256::new();
    for checksum in &sorted {
        hasher.update(checksum.as_bytes());
    }
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Check if an error is a connection/network error (vs. 404, etc.)
fn is_connection_error(err: &Error) -> bool {
    match err {
        Error::Network(msg) => {
            msg.contains("Connection failed")
                || msg.contains("timed out")
                || msg.contains("connect")
        }
        _ => false,
    }
}

/// Check if an error indicates a 404 / package not found
fn is_not_found_error(err: &Error) -> bool {
    match err {
        Error::Config(msg) => msg.to_lowercase().contains("not found"),
        _ => false,
    }
}

/// Save downloaded package files to a directory
///
/// # Arguments
/// * `download` - The downloaded package
/// * `ado_dir` - Target ado directory
///
/// # Returns
/// Map of filename to full path where it was saved
///
/// # Note
/// SSC packages can have files in different subdirectories (e.g., `../e/estfe.ado`).
/// Each file is placed in the correct subdirectory based on its actual filename.
pub fn save_package_files(
    download: &PackageDownload,
    ado_dir: &Path,
) -> Result<HashMap<String, std::path::PathBuf>> {
    let mut saved_files = HashMap::new();

    // Save each file in its correct subdirectory
    for file in &download.files {
        // Get the actual filename (strip any path components like "../e/")
        let filename = std::path::Path::new(&file.name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.name);

        // Determine subdirectory based on filename's first letter
        let subdir = filename
            .chars()
            .next()
            .unwrap_or('_')
            .to_lowercase()
            .next()
            .unwrap_or('_');
        let target_dir = ado_dir.join(subdir.to_string());

        // Create directory if needed
        if !target_dir.exists() {
            std::fs::create_dir_all(&target_dir).map_err(|e| {
                Error::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create {}: {}", target_dir.display(), e),
                ))
            })?;
        }

        let target_path = target_dir.join(filename);
        std::fs::write(&target_path, &file.content).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write {}: {}", target_path.display(), e),
            ))
        })?;
        saved_files.insert(filename.to_string(), target_path);
    }

    Ok(saved_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_package_url() {
        assert_eq!(
            SscDownloader::get_package_url("rdrobust"),
            "http://fmwww.bc.edu/repec/bocode/r/"
        );
        assert_eq!(
            SscDownloader::get_package_url("estout"),
            "http://fmwww.bc.edu/repec/bocode/e/"
        );
        assert_eq!(
            SscDownloader::get_package_url("UPPERCASE"),
            "http://fmwww.bc.edu/repec/bocode/u/"
        );
    }

    #[test]
    fn test_ssc_url_uses_http_not_https() {
        // SSC server (fmwww.bc.edu) does not support TLS.
        // Stata's own `ssc install` uses plain HTTP.
        // Using HTTPS causes TLS handshake failures and forces mirror fallback.
        assert!(
            SSC_BASE_URL.starts_with("http://"),
            "SSC_BASE_URL must use http://, not https:// — the server does not support TLS"
        );
        assert!(
            !SSC_BASE_URL.starts_with("https://"),
            "SSC_BASE_URL must NOT use https:// — fmwww.bc.edu does not support TLS"
        );
    }

    #[test]
    fn test_ssc_mirror_uses_https() {
        // The GitHub mirror (raw.githubusercontent.com) does support HTTPS
        assert!(
            SSC_MIRROR_URL.starts_with("https://"),
            "Mirror URL should use https:// (GitHub supports TLS)"
        );
    }

    #[test]
    fn test_calculate_sha256() {
        let data = b"hello world";
        let hash = calculate_sha256(data);
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    // Integration tests that require network access should be marked #[ignore]
    // Run with: cargo test -- --ignored

    #[test]
    #[ignore]
    fn test_download_package_integration() {
        let downloader = SscDownloader::new();
        let result = downloader.download_package("estout");
        assert!(result.is_ok());

        let download = result.unwrap();
        assert_eq!(download.name, "estout");
        assert!(!download.files.is_empty());
        assert!(!download.package_checksum.is_empty());
    }

    #[test]
    #[ignore]
    fn test_package_exists_integration() {
        let downloader = SscDownloader::new();

        // estout should exist
        assert!(downloader.package_exists("estout").unwrap());

        // random garbage should not exist
        assert!(!downloader
            .package_exists("definitely_not_a_real_package_xyz123")
            .unwrap());
    }

    #[test]
    #[ignore]
    fn test_get_manifest_integration() {
        let downloader = SscDownloader::new();
        let manifest = downloader.get_manifest("estout").unwrap();

        assert_eq!(manifest.name, "estout");
        assert!(!manifest.title.is_empty());
        assert!(!manifest.files.is_empty());
    }

    // Tests for mirror URL
    #[test]
    fn test_get_mirror_url() {
        assert_eq!(
            SscDownloader::get_mirror_url("rdrobust"),
            "https://raw.githubusercontent.com/labordynamicsinstitute/ssc-mirror/releases/fmwww.bc.edu/repec/bocode/r/"
        );
        assert_eq!(
            SscDownloader::get_mirror_url("estout"),
            "https://raw.githubusercontent.com/labordynamicsinstitute/ssc-mirror/releases/fmwww.bc.edu/repec/bocode/e/"
        );
    }

    // Tests for is_connection_error
    #[test]
    fn test_is_connection_error() {
        assert!(is_connection_error(&Error::Network(
            "Connection failed: some url".to_string()
        )));
        assert!(is_connection_error(&Error::Network(
            "Request timed out: some url".to_string()
        )));
        assert!(!is_connection_error(&Error::Network(
            "HTTP 404 for some url".to_string()
        )));
        assert!(!is_connection_error(&Error::Config(
            "some config error".to_string()
        )));
    }

    // Tests for is_not_found_error
    #[test]
    fn test_is_not_found_error() {
        assert!(is_not_found_error(&Error::Config(
            "Package not found: http://example.com/pkg.pkg".to_string()
        )));
        assert!(is_not_found_error(&Error::Config(
            "File not found: http://example.com/foo.ado".to_string()
        )));
        assert!(!is_not_found_error(&Error::Network(
            "Connection failed".to_string()
        )));
        assert!(!is_not_found_error(&Error::Config(
            "Invalid configuration".to_string()
        )));
    }

    // Tests for error message content
    #[test]
    fn test_error_message_package_not_found() {
        // Simulate what happens when both primary and mirror return 404
        let err = Error::Config(
            "Package 'notreal' not found on SSC. Check spelling or verify the package exists at https://ideas.repec.org/s/boc/bocode.html".to_string()
        );
        let msg = err.to_string();
        assert!(msg.contains("not found on SSC"));
        assert!(msg.contains("Check spelling"));
        assert!(msg.contains("ideas.repec.org"));
    }

    #[test]
    fn test_error_message_connection_error() {
        let err = Error::Network(
            "Both SSC and its GitHub mirror are unreachable. Check your internet connection."
                .to_string(),
        );
        let msg = err.to_string();
        assert!(msg.contains("unreachable"));
        assert!(msg.contains("internet connection"));
    }

    // Tests for save_package_files with cross-directory files
    #[test]
    fn test_save_package_files_cross_directory() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let ado_dir = temp.path().join("ado");

        // Simulate a package like reghdfe that has files in different directories
        let download = PackageDownload {
            name: "reghdfe".to_string(),
            manifest: crate::packages::pkg_parser::PackageManifest {
                name: "reghdfe".to_string(),
                title: "Test".to_string(),
                author: None,
                distribution_date: None,
                files: vec![],
                description_lines: vec![],
            },
            files: vec![
                DownloadedFile {
                    name: "reghdfe.ado".to_string(),
                    content: b"main file".to_vec(),
                    checksum: "abc".to_string(),
                },
                // This simulates "../e/estfe.ado" from the pkg manifest
                DownloadedFile {
                    name: "../e/estfe.ado".to_string(),
                    content: b"helper file".to_vec(),
                    checksum: "def".to_string(),
                },
            ],
            package_checksum: "combined".to_string(),
            from_mirror: false,
        };

        let saved = save_package_files(&download, &ado_dir).unwrap();

        // Check files were saved to correct subdirectories
        assert!(saved.contains_key("reghdfe.ado"));
        assert!(saved.contains_key("estfe.ado")); // Stripped path prefix
        assert!(ado_dir.join("r").join("reghdfe.ado").exists());
        assert!(ado_dir.join("e").join("estfe.ado").exists());

        // Verify content
        let content = std::fs::read(ado_dir.join("e").join("estfe.ado")).unwrap();
        assert_eq!(content, b"helper file");
    }

    // Test combined checksum is deterministic
    #[test]
    fn test_combined_checksum_deterministic() {
        let checksums1 = vec!["abc".to_string(), "def".to_string()];
        let checksums2 = vec!["abc".to_string(), "def".to_string()];

        let result1 = calculate_combined_checksum(&checksums1);
        let result2 = calculate_combined_checksum(&checksums2);

        assert_eq!(result1, result2);
    }

    // Test combined checksum is order-independent (H2 fix)
    #[test]
    fn test_combined_checksum_order_independent() {
        let checksums1 = vec!["abc".to_string(), "def".to_string()];
        let checksums2 = vec!["def".to_string(), "abc".to_string()];

        let result1 = calculate_combined_checksum(&checksums1);
        let result2 = calculate_combined_checksum(&checksums2);

        assert_eq!(result1, result2);
    }
}
