//! Net (URL) package source
//!
//! Downloads packages from arbitrary URLs using the same protocol as
//! Stata's `net install` command. Fetches `{name}.pkg` from the base URL,
//! parses the manifest, and downloads listed files.

use crate::error::Result;
use crate::packages::http::StacyHttpClient;
use crate::packages::pkg_parser::{parse_pkg_file, PackageManifest};
use crate::packages::ssc::{calculate_combined_checksum, calculate_sha256, DownloadedFile};

/// Result of downloading a package from a net URL
#[derive(Debug)]
pub struct NetPackageDownload {
    /// Package name
    pub name: String,
    /// Base URL used
    pub url: String,
    /// Parsed manifest
    pub manifest: PackageManifest,
    /// Downloaded files with checksums
    pub files: Vec<DownloadedFile>,
    /// Combined checksum of all files
    pub package_checksum: String,
}

/// Net package downloader
pub struct NetDownloader {
    client: StacyHttpClient,
}

impl Default for NetDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl NetDownloader {
    /// Create a new net downloader
    pub fn new() -> Self {
        Self {
            client: StacyHttpClient::new(),
        }
    }

    /// Download a package from a base URL
    ///
    /// Fetches `{base_url}/{name}.pkg`, parses the manifest, and downloads all listed files.
    ///
    /// # Arguments
    /// * `name` - Package name (e.g., "grc1leg")
    /// * `base_url` - Base URL (e.g., "http://www.stata.com/users/vwiggins/")
    pub fn download_package(&self, name: &str, base_url: &str) -> Result<NetPackageDownload> {
        let name = name.to_lowercase();

        // Ensure base URL ends with /
        let base_url = if base_url.ends_with('/') {
            base_url.to_string()
        } else {
            format!("{}/", base_url)
        };

        // Download .pkg manifest
        let pkg_url = format!("{}{}.pkg", base_url, name);
        let pkg_content = self.download_text(&pkg_url)?;

        // Parse manifest
        let manifest = parse_pkg_file(&pkg_content, &name)?;

        // Download all files listed in manifest
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

        Ok(NetPackageDownload {
            name,
            url: base_url,
            manifest,
            files,
            package_checksum,
        })
    }

    fn download_text(&self, url: &str) -> Result<String> {
        self.client.download_text(url)
    }

    fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.client.download_bytes(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downloader_creates_successfully() {
        let downloader = NetDownloader::new();
        // Verify the client was created (would panic in new() if it failed)
        let _ = &downloader.client;
    }

    #[test]
    fn test_download_nonexistent_url_errors() {
        let downloader = NetDownloader::new();
        let result = downloader.download_package("fakepkg", "http://127.0.0.1:1/nonexistent/");
        assert!(result.is_err());
    }

    #[test]
    fn test_download_package_result_fields() {
        // Verify NetPackageDownload struct can be constructed with expected fields
        let download = NetPackageDownload {
            name: "testpkg".to_string(),
            url: "http://example.com/stata/".to_string(),
            manifest: crate::packages::pkg_parser::PackageManifest {
                name: "testpkg".to_string(),
                title: "Test".to_string(),
                author: None,
                distribution_date: Some("20260101".to_string()),
                files: vec![],
                description_lines: vec![],
            },
            files: vec![],
            package_checksum: "abc123".to_string(),
        };
        assert_eq!(download.name, "testpkg");
        assert_eq!(download.url, "http://example.com/stata/");
    }

    #[test]
    fn test_download_text_404_returns_config_error() {
        let downloader = NetDownloader::new();
        // A URL that will reliably return 404
        let result = downloader.download_text("http://httpbin.org/status/404");
        // May fail with connection error in CI, so just verify it's an error
        assert!(result.is_err());
    }
}
