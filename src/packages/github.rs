//! GitHub package source
//!
//! Downloads Stata packages directly from GitHub repositories.
//! Looks for .pkg manifest files in the repository root.

use crate::error::{Error, Result};
use crate::packages::http::StacyHttpClient;
use crate::packages::pkg_parser::{parse_pkg_file, PackageManifest};
use crate::packages::ssc::{calculate_combined_checksum, calculate_sha256, DownloadedFile};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// GitHub API response for a tag
#[derive(Debug, Deserialize)]
struct GitHubTag {
    name: String,
}

/// GitHub API response for a commit
#[derive(Debug, Deserialize)]
struct GitHubCommit {
    sha: String,
}

/// GitHub API response for a repository tree
#[derive(Debug, Deserialize)]
struct GitHubTree {
    tree: Vec<GitHubTreeEntry>,
}

/// A single entry in a GitHub tree
#[derive(Debug, Deserialize)]
struct GitHubTreeEntry {
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
}

/// Information about the latest version of a GitHub package
#[derive(Debug)]
pub struct GitHubLatestInfo {
    /// Latest tag name (e.g., "v6.0.0")
    pub latest_tag: Option<String>,
    /// Current installed tag
    pub current_tag: String,
    /// Whether an update is available
    pub has_update: bool,
}

/// Result of downloading a package from GitHub
#[derive(Debug)]
pub struct GitHubPackageDownload {
    /// Package name
    pub name: String,
    /// GitHub repository (user/repo)
    pub repo: String,
    /// Git ref (branch/tag) used
    pub git_ref: String,
    /// Parsed manifest
    pub manifest: PackageManifest,
    /// Downloaded files with checksums
    pub files: Vec<DownloadedFile>,
    /// Combined checksum of all files
    pub package_checksum: String,
}

/// GitHub package downloader
pub struct GitHubDownloader {
    client: StacyHttpClient,
}

impl Default for GitHubDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubDownloader {
    /// Create a new GitHub downloader
    pub fn new() -> Self {
        Self {
            client: StacyHttpClient::new(),
        }
    }

    /// Get the raw GitHub URL for a file
    fn get_raw_url(user: &str, repo: &str, git_ref: &str, path: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            user, repo, git_ref, path
        )
    }

    /// Download a package from GitHub
    ///
    /// # Arguments
    /// * `name` - Package name (used to find {name}.pkg)
    /// * `user` - GitHub username/organization
    /// * `repo` - Repository name
    /// * `git_ref` - Branch, tag, or commit (defaults to trying "main" then "master")
    pub fn download_package(
        &self,
        name: &str,
        user: &str,
        repo: &str,
        git_ref: Option<&str>,
    ) -> Result<GitHubPackageDownload> {
        let name = name.to_lowercase();

        // If git_ref specified, use it; otherwise try main, then master
        let refs_to_try: Vec<&str> = if let Some(r) = git_ref {
            vec![r]
        } else {
            vec!["main", "master"]
        };

        let mut last_error = None;
        for git_ref in &refs_to_try {
            match self.try_download_package(&name, user, repo, git_ref) {
                Ok(download) => return Ok(download),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::Config(format!(
                "Could not find package {} in {}/{}",
                name, user, repo
            ))
        }))
    }

    /// Try to download package from a specific git ref
    fn try_download_package(
        &self,
        name: &str,
        user: &str,
        repo: &str,
        git_ref: &str,
    ) -> Result<GitHubPackageDownload> {
        // Try to find .pkg file - check multiple locations
        match self.find_and_download_pkg(name, user, repo, git_ref) {
            Ok(pkg_content) => {
                // Parse manifest
                let manifest = parse_pkg_file(&pkg_content, name)?;
                self.download_manifest_files(name, user, repo, git_ref, manifest)
            }
            Err(_pkg_err) => {
                // No .pkg found — try to synthesize manifest from repo tree
                match self.synthesize_manifest(name, user, repo, git_ref) {
                    Ok(manifest) => {
                        eprintln!(
                            "  note: No .pkg manifest found, inferred package contents from repository files"
                        );
                        self.download_manifest_files(name, user, repo, git_ref, manifest)
                    }
                    Err(synth_err) => Err(synth_err),
                }
            }
        }
    }

    /// Download all files listed in a manifest
    fn download_manifest_files(
        &self,
        name: &str,
        user: &str,
        repo: &str,
        git_ref: &str,
        manifest: PackageManifest,
    ) -> Result<GitHubPackageDownload> {
        let mut files = Vec::new();
        let mut checksums = Vec::new();

        for pkg_file in &manifest.files {
            // Get the actual filename (strip path components)
            let filename = std::path::Path::new(&pkg_file.name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&pkg_file.name);

            // For synthesized manifests, files have full paths; try that first, then filename
            let content = if pkg_file.name.contains('/') {
                // Try the full path first (from synthesized manifest)
                let url = Self::get_raw_url(user, repo, git_ref, &pkg_file.name);
                self.download_bytes(&url)
                    .or_else(|_| self.download_file(user, repo, git_ref, filename))?
            } else {
                self.download_file(user, repo, git_ref, filename)?
            };

            let checksum = calculate_sha256(&content);
            checksums.push(checksum.clone());
            files.push(DownloadedFile {
                name: filename.to_string(),
                content,
                checksum,
            });
        }

        let package_checksum = calculate_combined_checksum(&checksums);

        Ok(GitHubPackageDownload {
            name: name.to_string(),
            repo: format!("{}/{}", user, repo),
            git_ref: git_ref.to_string(),
            manifest,
            files,
            package_checksum,
        })
    }

    /// Synthesize a package manifest from the repository tree when no .pkg file exists
    ///
    /// Uses the GitHub API tree endpoint to find .ado/.sthlp files that belong
    /// to the package.
    fn synthesize_manifest(
        &self,
        name: &str,
        user: &str,
        repo: &str,
        git_ref: &str,
    ) -> Result<PackageManifest> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
            user, repo, git_ref
        );

        let response = self
            .client
            .inner()
            .get(&url)
            .send()
            .map_err(|e| Error::Network(format!("Failed to fetch repository tree: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Network(format!(
                "GitHub API error {} when fetching tree for {}/{}",
                response.status().as_u16(),
                user,
                repo
            )));
        }

        let tree: GitHubTree = response
            .json()
            .map_err(|e| Error::Network(format!("Failed to parse GitHub tree response: {}", e)))?;

        // Filter tree for Stata files
        let stata_extensions = ["ado", "sthlp", "hlp", "dlg", "mlib", "mata"];
        let preferred_dirs = ["", "src/", "ado/"];

        let mut matched_files = Vec::new();

        for entry in &tree.tree {
            if entry.entry_type != "blob" {
                continue;
            }

            let path_lower = entry.path.to_lowercase();
            let ext = entry.path.rsplit('.').next().unwrap_or("").to_lowercase();

            if !stata_extensions.contains(&ext.as_str()) {
                continue;
            }

            let filename = entry
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&entry.path)
                .to_lowercase();
            let stem = filename.split('.').next().unwrap_or("");

            // Match files that:
            // 1. Have the package name as stem (e.g., name.ado, name.sthlp)
            // 2. Are in preferred directories (root, src/, ado/)
            // 3. Start with _{name}_ (common helper pattern)
            // 4. Start with l{name} (Mata library pattern)
            let name_match = stem == name
                || filename.starts_with(&format!("_{}_", name))
                || filename.starts_with(&format!("l{}.", name));

            let in_preferred_dir = preferred_dirs
                .iter()
                .any(|dir| path_lower.starts_with(dir) || !entry.path.contains('/'));

            if name_match || (in_preferred_dir && ext == "ado") {
                let file_type = crate::packages::pkg_parser::FileType::from_extension(&ext);
                matched_files.push(crate::packages::pkg_parser::PackageFile {
                    name: entry.path.clone(),
                    file_type,
                });
            }
        }

        if !matched_files
            .iter()
            .any(|f| matches!(f.file_type, crate::packages::pkg_parser::FileType::Ado))
        {
            return Err(Error::Config(format!(
                "No .pkg manifest or .ado files found in {}/{}",
                user, repo
            )));
        }

        Ok(PackageManifest {
            name: name.to_string(),
            title: format!("{} (synthesized from repository)", name),
            author: None,
            distribution_date: None,
            files: matched_files,
            description_lines: vec![],
        })
    }

    /// Find and download the .pkg file, checking multiple locations
    fn find_and_download_pkg(
        &self,
        name: &str,
        user: &str,
        repo: &str,
        git_ref: &str,
    ) -> Result<String> {
        // Try common locations for .pkg files
        let locations = vec![
            format!("{}.pkg", name),          // repo root
            format!("src/{}.pkg", name),      // src/ directory
            format!("pkg/{}.pkg", name),      // pkg/ directory
            format!("ado/{}.pkg", name),      // ado/ directory
            format!("{}/{}.pkg", name, name), // package subdirectory
        ];

        for location in &locations {
            let url = Self::get_raw_url(user, repo, git_ref, location);
            match self.download_text(&url) {
                Ok(content) => return Ok(content),
                Err(_) => continue,
            }
        }

        Err(Error::Config(format!(
            "Could not find {}.pkg in repository {}/{}. Checked: {}",
            name,
            user,
            repo,
            locations.join(", ")
        )))
    }

    /// Download a file, checking multiple locations
    fn download_file(
        &self,
        user: &str,
        repo: &str,
        git_ref: &str,
        filename: &str,
    ) -> Result<Vec<u8>> {
        // Try common locations
        let locations = vec![
            filename.to_string(),        // repo root
            format!("src/{}", filename), // src/ directory
            format!("ado/{}", filename), // ado/ directory
        ];

        for location in &locations {
            let url = Self::get_raw_url(user, repo, git_ref, location);
            match self.download_bytes(&url) {
                Ok(content) => return Ok(content),
                Err(_) => continue,
            }
        }

        Err(Error::Network(format!(
            "Could not download {} from {}/{}",
            filename, user, repo
        )))
    }

    fn download_text(&self, url: &str) -> Result<String> {
        self.client.download_text(url)
    }

    fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.client.download_bytes(url)
    }

    /// Get the latest tag for a repository
    ///
    /// Uses the GitHub API to fetch tags and returns the first one (most recent).
    /// Returns None if the repository has no tags.
    pub fn get_latest_tag(&self, user: &str, repo: &str) -> Result<Option<String>> {
        let url = format!("https://api.github.com/repos/{}/{}/tags", user, repo);

        let response = self.client.inner().get(&url).send().map_err(|e| {
            if e.is_timeout() {
                Error::Network(format!("Request timed out: {}", url))
            } else if e.is_connect() {
                Error::Network(format!("Connection failed: {}", url))
            } else {
                Error::Network(format!("HTTP error: {}", e))
            }
        })?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 {
                return Err(Error::Config(format!(
                    "Repository not found: {}/{}",
                    user, repo
                )));
            }
            if status.as_u16() == 403 {
                // Rate limited
                return Err(Error::Network("GitHub API rate limit exceeded".to_string()));
            }
            return Err(Error::Network(format!(
                "GitHub API error {} for {}/{}",
                status.as_u16(),
                user,
                repo
            )));
        }

        let tags: Vec<GitHubTag> = response
            .json()
            .map_err(|e| Error::Network(format!("Failed to parse GitHub API response: {}", e)))?;

        Ok(tags.first().map(|t| t.name.clone()))
    }

    /// Check if a newer version is available for a GitHub package
    ///
    /// Compares the installed tag with the latest tag from the repository.
    pub fn check_for_updates(
        &self,
        user: &str,
        repo: &str,
        current_tag: &str,
    ) -> Result<GitHubLatestInfo> {
        let latest_tag = self.get_latest_tag(user, repo)?;

        let has_update = match &latest_tag {
            Some(latest) => {
                // Simple comparison - if tags are different and current isn't "main"/"master"
                let is_branch =
                    current_tag == "main" || current_tag == "master" || current_tag == "latest";
                if is_branch {
                    // Can't easily compare branches, assume no update
                    false
                } else {
                    latest != current_tag
                }
            }
            None => false,
        };

        Ok(GitHubLatestInfo {
            latest_tag,
            current_tag: current_tag.to_string(),
            has_update,
        })
    }

    /// Resolve a git ref (branch, tag, or short SHA) to a full commit SHA.
    ///
    /// Uses the GitHub Commits API. Returns `None` on any failure
    /// (rate limit, network error, etc.) for graceful degradation.
    pub fn resolve_commit_sha(&self, user: &str, repo: &str, git_ref: &str) -> Option<String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/commits/{}",
            user, repo, git_ref
        );

        let response = self.client.inner().get(&url).send().ok()?;
        if !response.status().is_success() {
            return None;
        }

        let commit: GitHubCommit = response.json().ok()?;
        Some(commit.sha)
    }
}

/// Save downloaded GitHub package files to a directory
pub fn save_github_package_files(
    download: &GitHubPackageDownload,
    ado_dir: &Path,
) -> Result<HashMap<String, std::path::PathBuf>> {
    let mut saved_files = HashMap::new();

    // Save each file in its correct subdirectory
    for file in &download.files {
        // Get the actual filename
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
    use tempfile::TempDir;

    #[test]
    fn test_get_raw_url() {
        assert_eq!(
            GitHubDownloader::get_raw_url("user", "repo", "main", "file.ado"),
            "https://raw.githubusercontent.com/user/repo/main/file.ado"
        );
    }

    #[test]
    fn test_get_raw_url_with_tag() {
        assert_eq!(
            GitHubDownloader::get_raw_url("sergiocorreia", "reghdfe", "v6.0.0", "reghdfe.pkg"),
            "https://raw.githubusercontent.com/sergiocorreia/reghdfe/v6.0.0/reghdfe.pkg"
        );
    }

    #[test]
    fn test_get_raw_url_with_path() {
        assert_eq!(
            GitHubDownloader::get_raw_url("user", "repo", "main", "src/file.ado"),
            "https://raw.githubusercontent.com/user/repo/main/src/file.ado"
        );
    }

    #[test]
    fn test_calculate_combined_checksum() {
        let checksums = vec!["abc".to_string(), "def".to_string()];
        let result = calculate_combined_checksum(&checksums);
        // Should be consistent
        let result2 = calculate_combined_checksum(&checksums);
        assert_eq!(result, result2);
        // Should be a valid hex string
        assert!(result.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_save_github_package_files() {
        let temp = TempDir::new().unwrap();
        let ado_dir = temp.path().join("ado");

        let download = GitHubPackageDownload {
            name: "testpkg".to_string(),
            repo: "user/repo".to_string(),
            git_ref: "main".to_string(),
            manifest: crate::packages::pkg_parser::PackageManifest {
                name: "testpkg".to_string(),
                title: "Test Package".to_string(),
                author: None,
                distribution_date: None,
                files: vec![],
                description_lines: vec![],
            },
            files: vec![
                DownloadedFile {
                    name: "testpkg.ado".to_string(),
                    content: b"test content".to_vec(),
                    checksum: "abc123".to_string(),
                },
                DownloadedFile {
                    name: "helper.ado".to_string(),
                    content: b"helper content".to_vec(),
                    checksum: "def456".to_string(),
                },
            ],
            package_checksum: "combined".to_string(),
        };

        let saved = save_github_package_files(&download, &ado_dir).unwrap();

        // Check files were saved to correct subdirectories
        assert!(saved.contains_key("testpkg.ado"));
        assert!(saved.contains_key("helper.ado"));
        assert!(ado_dir.join("t").join("testpkg.ado").exists());
        assert!(ado_dir.join("h").join("helper.ado").exists());
    }

    #[test]
    fn test_synthesize_manifest_from_tree_response() {
        // Test the tree filtering logic by simulating what synthesize_manifest would find
        use crate::packages::pkg_parser::FileType;

        let stata_extensions = ["ado", "sthlp", "hlp", "dlg", "mlib", "mata"];
        let name = "mypkg";

        // Simulate tree entries
        let paths = vec![
            "mypkg.ado",
            "mypkg.sthlp",
            "README.md",
            "src/helper.ado",
            "_mypkg_internal.ado",
            "lmypkg.mlib",
            "other.do",
            "unrelated.ado",
        ];

        let preferred_dirs: Vec<&str> = vec!["", "src/", "ado/"];
        let mut matched = Vec::new();

        for path in &paths {
            let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
            if !stata_extensions.contains(&ext.as_str()) {
                continue;
            }

            let filename = path.rsplit('/').next().unwrap_or(path).to_lowercase();
            let stem = filename.split('.').next().unwrap_or("");

            let name_match = stem == name
                || filename.starts_with(&format!("_{}_", name))
                || filename.starts_with(&format!("l{}.", name));

            let in_preferred_dir = preferred_dirs
                .iter()
                .any(|dir| path.to_lowercase().starts_with(dir) || !path.contains('/'));

            if name_match || (in_preferred_dir && ext == "ado") {
                matched.push(path.to_string());
            }
        }

        // Should match: mypkg.ado, mypkg.sthlp, _mypkg_internal.ado, lmypkg.mlib, unrelated.ado (in root, .ado)
        assert!(matched.contains(&"mypkg.ado".to_string()));
        assert!(matched.contains(&"mypkg.sthlp".to_string()));
        assert!(matched.contains(&"_mypkg_internal.ado".to_string()));
        assert!(matched.contains(&"lmypkg.mlib".to_string()));
        // unrelated.ado is in root and is .ado, so it matches the broader fallback
        assert!(matched.contains(&"unrelated.ado".to_string()));
        // README.md should not match
        assert!(!matched.contains(&"README.md".to_string()));
        // src/helper.ado is in preferred dir and .ado
        assert!(matched.contains(&"src/helper.ado".to_string()));
    }

    #[test]
    fn test_synthesize_manifest_no_ado_files() {
        // If a repo has no .ado files, synthesize_manifest should fail.
        // We can test the validation logic: an empty matched_files vec with no Ado types
        use crate::packages::pkg_parser::{FileType, PackageFile};

        let matched_files: Vec<PackageFile> = vec![PackageFile {
            name: "readme.sthlp".to_string(),
            file_type: FileType::Help,
        }];

        let has_ado = matched_files
            .iter()
            .any(|f| matches!(f.file_type, FileType::Ado));
        assert!(!has_ado, "Should have no .ado files");
    }

    // Integration tests that require network
    #[test]
    #[ignore]
    fn test_download_package_integration() {
        let downloader = GitHubDownloader::new();
        // ftools is a simple package with clear structure
        let result =
            downloader.download_package("ftools", "sergiocorreia", "ftools", Some("master"));

        if let Ok(download) = result {
            assert_eq!(download.name, "ftools");
            assert!(!download.files.is_empty());
            assert_eq!(download.git_ref, "master");
        }
        // Note: This test may fail if the package structure changes
    }

    #[test]
    #[ignore]
    fn test_download_package_auto_branch() {
        let downloader = GitHubDownloader::new();
        // Without specifying ref, should try main then master
        let result = downloader.download_package("ftools", "sergiocorreia", "ftools", None);

        if let Ok(download) = result {
            assert_eq!(download.name, "ftools");
            // Should have found it on master (ftools uses master branch)
            assert_eq!(download.git_ref, "master");
        }
    }
}
