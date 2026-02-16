//! Build cache for incremental builds
//!
//! Stores execution results based on script content hashes and dependency trees.
//! Cache is opt-in via `--cache` flag and stored in `.stacy/cache/build.json`.

pub mod detect;
pub mod hash;

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cache format version for backwards compatibility
const CACHE_VERSION: u32 = 1;

/// Directory name for stacy internal files
const STACY_DIR: &str = ".stacy";

/// Cache file path within .stacy directory
const CACHE_FILE: &str = "cache/build.json";

/// Build cache containing all cached execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCache {
    /// Cache format version
    pub version: u32,
    /// Map of script paths to their cached entries
    pub entries: HashMap<PathBuf, CacheEntry>,
}

impl Default for BuildCache {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
        }
    }
}

impl BuildCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Load cache from the project's .stacy directory
    pub fn load(project_root: &Path) -> Result<Self> {
        let cache_path = project_root.join(STACY_DIR).join(CACHE_FILE);

        if !cache_path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&cache_path).map_err(|e| {
            Error::Config(format!(
                "Failed to read cache file {}: {}",
                cache_path.display(),
                e
            ))
        })?;

        let cache: BuildCache = serde_json::from_str(&content).map_err(|e| {
            Error::Config(format!(
                "Failed to parse cache file {}: {}",
                cache_path.display(),
                e
            ))
        })?;

        // Check version compatibility
        if cache.version != CACHE_VERSION {
            // Version mismatch - return empty cache (will be rebuilt)
            return Ok(Self::new());
        }

        Ok(cache)
    }

    /// Save cache to the project's .stacy directory
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let stacy_dir = project_root.join(STACY_DIR);
        let cache_dir = stacy_dir.join("cache");
        let cache_path = stacy_dir.join(CACHE_FILE);

        // Ensure directories exist
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).map_err(|e| {
                Error::Config(format!(
                    "Failed to create cache directory {}: {}",
                    cache_dir.display(),
                    e
                ))
            })?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize cache: {}", e)))?;

        std::fs::write(&cache_path, content).map_err(|e| {
            Error::Config(format!(
                "Failed to write cache file {}: {}",
                cache_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Get a cached entry for a script
    pub fn get(&self, script: &Path) -> Option<&CacheEntry> {
        // Try to canonicalize the path for consistent lookup
        let key = script
            .canonicalize()
            .unwrap_or_else(|_| script.to_path_buf());
        self.entries.get(&key)
    }

    /// Insert or update a cache entry for a script
    pub fn insert(&mut self, script: &Path, entry: CacheEntry) {
        // Use canonicalized path for consistent storage
        let key = script
            .canonicalize()
            .unwrap_or_else(|_| script.to_path_buf());
        self.entries.insert(key, entry);
    }

    /// Remove a cached entry for a script
    pub fn remove(&mut self, script: &Path) -> Option<CacheEntry> {
        let key = script
            .canonicalize()
            .unwrap_or_else(|_| script.to_path_buf());
        self.entries.remove(&key)
    }

    /// Clear all entries from the cache
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove entries older than a given number of days
    pub fn remove_older_than(&mut self, days: u32) -> usize {
        let cutoff = SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(days as u64 * 24 * 60 * 60))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let initial_count = self.entries.len();
        self.entries.retain(|_, entry| entry.cached_at >= cutoff);
        initial_count - self.entries.len()
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get total size of cached entries in bytes (approximate)
    pub fn size_bytes(&self) -> usize {
        serde_json::to_string(self).map(|s| s.len()).unwrap_or(0)
    }

    /// Get the cache file path for a project
    pub fn cache_path(project_root: &Path) -> PathBuf {
        project_root.join(STACY_DIR).join(CACHE_FILE)
    }

    /// Delete the cache file from disk
    pub fn delete_file(project_root: &Path) -> Result<bool> {
        let cache_path = Self::cache_path(project_root);
        if cache_path.exists() {
            std::fs::remove_file(&cache_path).map_err(|e| {
                Error::Config(format!(
                    "Failed to delete cache file {}: {}",
                    cache_path.display(),
                    e
                ))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// A cached execution result for a single script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// SHA256 hash of the script content
    pub script_hash: String,
    /// SHA256 hashes of all dependency files
    pub dependency_hashes: HashMap<PathBuf, String>,
    /// SHA256 hash of the lockfile (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockfile_hash: Option<String>,
    /// SHA256 hash of the working directory path (if set via -C or --cd)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir_hash: Option<String>,
    /// Cached execution result
    pub result: CachedResult,
    /// When this entry was cached
    #[serde(with = "system_time_serde")]
    pub cached_at: SystemTime,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(
        script_hash: String,
        dependency_hashes: HashMap<PathBuf, String>,
        lockfile_hash: Option<String>,
        result: CachedResult,
    ) -> Self {
        Self {
            script_hash,
            dependency_hashes,
            lockfile_hash,
            working_dir_hash: None,
            result,
            cached_at: SystemTime::now(),
        }
    }

    /// Create a new cache entry with working directory
    pub fn with_working_dir(
        script_hash: String,
        dependency_hashes: HashMap<PathBuf, String>,
        lockfile_hash: Option<String>,
        working_dir_hash: Option<String>,
        result: CachedResult,
    ) -> Self {
        Self {
            script_hash,
            dependency_hashes,
            lockfile_hash,
            working_dir_hash,
            result,
            cached_at: SystemTime::now(),
        }
    }

    /// Get the age of this cache entry in seconds
    pub fn age_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.cached_at)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Cached execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    /// Exit code from execution
    pub exit_code: i32,
    /// Whether execution was successful
    pub success: bool,
    /// Execution duration in seconds
    pub duration_secs: f64,
    /// Any errors that occurred
    pub errors: Vec<CachedError>,
}

/// Cached error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedError {
    /// Error type (e.g., "StataCode", "ProcessKilled")
    pub error_type: String,
    /// Stata error code if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r_code: Option<u32>,
    /// Error message
    pub message: String,
    /// Line number if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
}

/// Serde module for SystemTime (serialize as Unix timestamp)
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_new() {
        let cache = BuildCache::new();
        assert_eq!(cache.version, CACHE_VERSION);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_insert_get() {
        let mut cache = BuildCache::new();
        let script = PathBuf::from("test.do");

        let entry = CacheEntry::new(
            "abc123".to_string(),
            HashMap::new(),
            None,
            CachedResult {
                exit_code: 0,
                success: true,
                duration_secs: 1.5,
                errors: vec![],
            },
        );

        cache.insert(&script, entry.clone());
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(&script);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().script_hash, "abc123");
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = BuildCache::new();
        let script = PathBuf::from("test.do");

        let entry = CacheEntry::new(
            "abc123".to_string(),
            HashMap::new(),
            None,
            CachedResult {
                exit_code: 0,
                success: true,
                duration_secs: 1.5,
                errors: vec![],
            },
        );

        cache.insert(&script, entry);
        assert_eq!(cache.len(), 1);

        cache.remove(&script);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = BuildCache::new();

        for i in 0..5 {
            let script = PathBuf::from(format!("test{}.do", i));
            let entry = CacheEntry::new(
                format!("hash{}", i),
                HashMap::new(),
                None,
                CachedResult {
                    exit_code: 0,
                    success: true,
                    duration_secs: 1.0,
                    errors: vec![],
                },
            );
            cache.insert(&script, entry);
        }

        assert_eq!(cache.len(), 5);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_save_load() {
        let temp = TempDir::new().unwrap();
        let mut cache = BuildCache::new();

        let script = PathBuf::from("test.do");
        let entry = CacheEntry::new(
            "abc123".to_string(),
            HashMap::new(),
            None,
            CachedResult {
                exit_code: 0,
                success: true,
                duration_secs: 1.5,
                errors: vec![],
            },
        );
        cache.insert(&script, entry);

        // Save cache
        cache.save(temp.path()).unwrap();

        // Verify file exists
        assert!(temp.path().join(".stacy/cache/build.json").exists());

        // Load cache
        let loaded = BuildCache::load(temp.path()).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_cache_load_nonexistent() {
        let temp = TempDir::new().unwrap();
        let cache = BuildCache::load(temp.path()).unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_entry_age() {
        let entry = CacheEntry::new(
            "abc123".to_string(),
            HashMap::new(),
            None,
            CachedResult {
                exit_code: 0,
                success: true,
                duration_secs: 1.0,
                errors: vec![],
            },
        );

        // Age should be very small (just created)
        assert!(entry.age_secs() < 5);
    }

    #[test]
    fn test_cached_result_with_errors() {
        let result = CachedResult {
            exit_code: 2,
            success: false,
            duration_secs: 0.5,
            errors: vec![CachedError {
                error_type: "StataCode".to_string(),
                r_code: Some(198),
                message: "invalid syntax".to_string(),
                line_number: Some(42),
            }],
        };

        assert!(!result.success);
        assert_eq!(result.exit_code, 2);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].r_code, Some(198));
    }

    #[test]
    fn test_cache_serialization() {
        let mut cache = BuildCache::new();
        let script = PathBuf::from("test.do");

        let mut dep_hashes = HashMap::new();
        dep_hashes.insert(PathBuf::from("helper.do"), "def456".to_string());

        let entry = CacheEntry::new(
            "abc123".to_string(),
            dep_hashes,
            Some("lockfile789".to_string()),
            CachedResult {
                exit_code: 0,
                success: true,
                duration_secs: 2.5,
                errors: vec![],
            },
        );
        cache.insert(&script, entry);

        // Serialize and deserialize
        let json = serde_json::to_string(&cache).unwrap();
        let deserialized: BuildCache = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 1);
    }
}
