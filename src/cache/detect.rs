//! Change detection for incremental builds
//!
//! Compares current script and dependency hashes against cached values
//! to determine if a rebuild is necessary.

use super::hash::{hash_dependency_tree, hash_lockfile};
use super::{BuildCache, CacheEntry};
use crate::error::Result;
use std::path::Path;

/// Reason why a rebuild is required
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebuildReason {
    /// No cached entry exists for this script
    NotCached,
    /// The script content has changed
    ScriptChanged,
    /// A dependency file has changed
    DependencyChanged(String),
    /// The lockfile has changed
    LockfileChanged,
    /// A dependency was added
    DependencyAdded(String),
    /// A dependency was removed
    DependencyRemoved(String),
    /// The working directory has changed
    WorkingDirChanged,
    /// Force rebuild was requested
    ForceRebuild,
}

impl std::fmt::Display for RebuildReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RebuildReason::NotCached => write!(f, "not in cache"),
            RebuildReason::ScriptChanged => write!(f, "script changed"),
            RebuildReason::DependencyChanged(dep) => write!(f, "dependency changed: {}", dep),
            RebuildReason::LockfileChanged => write!(f, "lockfile changed"),
            RebuildReason::DependencyAdded(dep) => write!(f, "dependency added: {}", dep),
            RebuildReason::DependencyRemoved(dep) => write!(f, "dependency removed: {}", dep),
            RebuildReason::WorkingDirChanged => write!(f, "working directory changed"),
            RebuildReason::ForceRebuild => write!(f, "forced rebuild"),
        }
    }
}

/// Result of change detection
#[derive(Debug)]
pub enum CacheStatus {
    /// Cache hit - no changes detected, can use cached result
    Hit(CacheEntry),
    /// Cache miss - rebuild required
    Miss(RebuildReason),
}

impl CacheStatus {
    /// Returns true if this is a cache hit
    pub fn is_hit(&self) -> bool {
        matches!(self, CacheStatus::Hit(_))
    }

    /// Returns true if this is a cache miss
    pub fn is_miss(&self) -> bool {
        matches!(self, CacheStatus::Miss(_))
    }

    /// Get the cached entry if this is a hit
    pub fn entry(&self) -> Option<&CacheEntry> {
        match self {
            CacheStatus::Hit(entry) => Some(entry),
            CacheStatus::Miss(_) => None,
        }
    }

    /// Get the rebuild reason if this is a miss
    pub fn reason(&self) -> Option<&RebuildReason> {
        match self {
            CacheStatus::Hit(_) => None,
            CacheStatus::Miss(reason) => Some(reason),
        }
    }
}

/// Check if a script needs to be rebuilt
///
/// Compares the current script and all its dependencies against cached hashes.
/// Returns a CacheStatus indicating whether we can use the cached result.
pub fn check_cache(
    cache: &BuildCache,
    script: &Path,
    project_root: Option<&Path>,
    force: bool,
) -> Result<CacheStatus> {
    check_cache_with_working_dir(cache, script, project_root, None, force)
}

/// Check if a script needs to be rebuilt, with working directory support
///
/// Like check_cache, but also considers the working directory. If the cached
/// entry was created with a different working directory, it's a cache miss.
pub fn check_cache_with_working_dir(
    cache: &BuildCache,
    script: &Path,
    project_root: Option<&Path>,
    working_dir: Option<&Path>,
    force: bool,
) -> Result<CacheStatus> {
    use super::hash::hash_string;

    // Force rebuild if requested
    if force {
        return Ok(CacheStatus::Miss(RebuildReason::ForceRebuild));
    }

    // Get cached entry
    let cached = match cache.get(script) {
        Some(entry) => entry,
        None => return Ok(CacheStatus::Miss(RebuildReason::NotCached)),
    };

    // Compute current hashes
    let current_hashes = hash_dependency_tree(script)?;

    // Check script hash
    if cached.script_hash != current_hashes.script_hash {
        return Ok(CacheStatus::Miss(RebuildReason::ScriptChanged));
    }

    // Check working directory hash
    let current_working_dir_hash = working_dir.map(|d| hash_string(&d.display().to_string()));
    if cached.working_dir_hash != current_working_dir_hash {
        return Ok(CacheStatus::Miss(RebuildReason::WorkingDirChanged));
    }

    // Check lockfile hash (if we have a project root)
    if let Some(root) = project_root {
        let current_lockfile_hash = hash_lockfile(root)?;

        match (&cached.lockfile_hash, &current_lockfile_hash) {
            (Some(cached_hash), Some(current_hash)) if cached_hash != current_hash => {
                return Ok(CacheStatus::Miss(RebuildReason::LockfileChanged));
            }
            (None, Some(_)) => {
                return Ok(CacheStatus::Miss(RebuildReason::LockfileChanged));
            }
            (Some(_), None) => {
                return Ok(CacheStatus::Miss(RebuildReason::LockfileChanged));
            }
            _ => {}
        }
    }

    // Check for added dependencies
    for path in current_hashes.dependency_hashes.keys() {
        if !cached.dependency_hashes.contains_key(path) {
            return Ok(CacheStatus::Miss(RebuildReason::DependencyAdded(
                path.display().to_string(),
            )));
        }
    }

    // Check for removed dependencies
    for path in cached.dependency_hashes.keys() {
        if !current_hashes.dependency_hashes.contains_key(path) {
            return Ok(CacheStatus::Miss(RebuildReason::DependencyRemoved(
                path.display().to_string(),
            )));
        }
    }

    // Check each dependency hash
    for (path, current_hash) in &current_hashes.dependency_hashes {
        if let Some(cached_hash) = cached.dependency_hashes.get(path) {
            if cached_hash != current_hash {
                return Ok(CacheStatus::Miss(RebuildReason::DependencyChanged(
                    path.display().to_string(),
                )));
            }
        }
    }

    // All checks passed - cache hit!
    Ok(CacheStatus::Hit(cached.clone()))
}

/// Convenience function to check if rebuild is needed (returns bool)
pub fn needs_rebuild(
    cache: &BuildCache,
    script: &Path,
    project_root: Option<&Path>,
    force: bool,
) -> Result<bool> {
    let status = check_cache(cache, script, project_root, force)?;
    Ok(status.is_miss())
}

/// Hash a working directory path for cache comparison
pub fn hash_working_dir(working_dir: Option<&Path>) -> Option<String> {
    use super::hash::hash_string;
    working_dir.map(|d| hash_string(&d.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{CacheEntry, CachedResult};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_cache_entry(script_hash: &str, dep_hashes: HashMap<PathBuf, String>) -> CacheEntry {
        CacheEntry::new(
            script_hash.to_string(),
            dep_hashes,
            None,
            CachedResult {
                exit_code: 0,
                success: true,
                duration_secs: 1.0,
                errors: vec![],
            },
        )
    }

    #[test]
    fn test_not_cached() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("test.do");
        fs::write(&script, "display 1").unwrap();

        let cache = BuildCache::new();
        let status = check_cache(&cache, &script, None, false).unwrap();

        assert!(status.is_miss());
        assert_eq!(status.reason(), Some(&RebuildReason::NotCached));
    }

    #[test]
    fn test_force_rebuild() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("test.do");
        fs::write(&script, "display 1").unwrap();

        let mut cache = BuildCache::new();
        let hash = crate::cache::hash::hash_file(&script).unwrap();
        cache.insert(&script, create_cache_entry(&hash, HashMap::new()));

        let status = check_cache(&cache, &script, None, true).unwrap();

        assert!(status.is_miss());
        assert_eq!(status.reason(), Some(&RebuildReason::ForceRebuild));
    }

    #[test]
    fn test_cache_hit() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("test.do");
        fs::write(&script, "display 1").unwrap();

        let mut cache = BuildCache::new();
        let hash = crate::cache::hash::hash_file(&script).unwrap();
        cache.insert(&script, create_cache_entry(&hash, HashMap::new()));

        let status = check_cache(&cache, &script, None, false).unwrap();

        assert!(status.is_hit());
        assert!(status.entry().is_some());
    }

    #[test]
    fn test_script_changed() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("test.do");
        fs::write(&script, "display 1").unwrap();

        let mut cache = BuildCache::new();
        // Cache with old hash
        cache.insert(&script, create_cache_entry("old_hash", HashMap::new()));

        // Script now has different content (different hash)
        let status = check_cache(&cache, &script, None, false).unwrap();

        assert!(status.is_miss());
        assert_eq!(status.reason(), Some(&RebuildReason::ScriptChanged));
    }

    #[test]
    fn test_dependency_changed() {
        let temp = TempDir::new().unwrap();

        // Create helper file
        let helper = temp.path().join("helper.do");
        fs::write(&helper, "display \"helper\"").unwrap();

        // Create main file
        let main = temp.path().join("main.do");
        fs::write(&main, "do \"helper.do\"\ndisplay \"main\"").unwrap();

        // Get current hashes
        let main_hash = crate::cache::hash::hash_file(&main).unwrap();
        let helper_canonical = helper.canonicalize().unwrap();

        // Create cache with old helper hash
        let mut dep_hashes = HashMap::new();
        dep_hashes.insert(helper_canonical, "old_helper_hash".to_string());

        let mut cache = BuildCache::new();
        cache.insert(&main, create_cache_entry(&main_hash, dep_hashes));

        // Check - helper hash is now different
        let status = check_cache(&cache, &main, None, false).unwrap();

        assert!(status.is_miss());
        match status.reason() {
            Some(RebuildReason::DependencyChanged(_)) => {}
            other => panic!("Expected DependencyChanged, got {:?}", other),
        }
    }

    #[test]
    fn test_dependency_added() {
        let temp = TempDir::new().unwrap();

        // Create main file initially without deps
        let main = temp.path().join("main.do");
        fs::write(&main, "display \"main\"").unwrap();

        let main_hash = crate::cache::hash::hash_file(&main).unwrap();

        // Cache with no dependencies
        let mut cache = BuildCache::new();
        cache.insert(&main, create_cache_entry(&main_hash, HashMap::new()));

        // Now add a dependency
        let helper = temp.path().join("helper.do");
        fs::write(&helper, "display \"helper\"").unwrap();
        fs::write(&main, "do \"helper.do\"\ndisplay \"main\"").unwrap();

        // Note: main_hash changed, so we'll get ScriptChanged first
        // Let's test with correct script hash but missing dep
        let new_main_hash = crate::cache::hash::hash_file(&main).unwrap();
        cache.insert(&main, create_cache_entry(&new_main_hash, HashMap::new()));

        let status = check_cache(&cache, &main, None, false).unwrap();

        assert!(status.is_miss());
        match status.reason() {
            Some(RebuildReason::DependencyAdded(_)) => {}
            other => panic!("Expected DependencyAdded, got {:?}", other),
        }
    }

    #[test]
    fn test_needs_rebuild() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("test.do");
        fs::write(&script, "display 1").unwrap();

        let cache = BuildCache::new();

        // Not cached - needs rebuild
        assert!(needs_rebuild(&cache, &script, None, false).unwrap());
    }

    #[test]
    fn test_rebuild_reason_display() {
        assert_eq!(RebuildReason::NotCached.to_string(), "not in cache");
        assert_eq!(RebuildReason::ScriptChanged.to_string(), "script changed");
        assert_eq!(
            RebuildReason::LockfileChanged.to_string(),
            "lockfile changed"
        );
        assert_eq!(
            RebuildReason::WorkingDirChanged.to_string(),
            "working directory changed"
        );
        assert_eq!(RebuildReason::ForceRebuild.to_string(), "forced rebuild");
        assert_eq!(
            RebuildReason::DependencyChanged("helper.do".to_string()).to_string(),
            "dependency changed: helper.do"
        );
    }

    #[test]
    fn test_working_dir_changed() {
        let temp = TempDir::new().unwrap();
        let script = temp.path().join("test.do");
        fs::write(&script, "display 1").unwrap();

        let mut cache = BuildCache::new();
        let hash = crate::cache::hash::hash_file(&script).unwrap();

        // Cache with a specific working directory
        let working_dir_hash = Some(crate::cache::hash::hash_string(
            &temp.path().display().to_string(),
        ));
        let mut entry = create_cache_entry(&hash, HashMap::new());
        entry.working_dir_hash = working_dir_hash;
        cache.insert(&script, entry);

        // Check with same working dir - should be hit
        let status =
            check_cache_with_working_dir(&cache, &script, None, Some(temp.path()), false).unwrap();
        assert!(status.is_hit());

        // Check with different working dir - should be miss
        let other_dir = TempDir::new().unwrap();
        let status =
            check_cache_with_working_dir(&cache, &script, None, Some(other_dir.path()), false)
                .unwrap();
        assert!(status.is_miss());
        assert_eq!(status.reason(), Some(&RebuildReason::WorkingDirChanged));

        // Check with no working dir when cache has one - should be miss
        let status = check_cache_with_working_dir(&cache, &script, None, None, false).unwrap();
        assert!(status.is_miss());
        assert_eq!(status.reason(), Some(&RebuildReason::WorkingDirChanged));
    }

    #[test]
    fn test_cache_hit_with_dependencies() {
        let temp = TempDir::new().unwrap();

        // Create helper
        let helper = temp.path().join("helper.do");
        fs::write(&helper, "display \"helper\"").unwrap();

        // Create main
        let main = temp.path().join("main.do");
        fs::write(&main, "do \"helper.do\"\ndisplay \"main\"").unwrap();

        // Get all hashes
        let hashes = crate::cache::hash::hash_dependency_tree(&main).unwrap();

        // Create cache with correct hashes
        let mut cache = BuildCache::new();
        cache.insert(
            &main,
            create_cache_entry(&hashes.script_hash, hashes.dependency_hashes),
        );

        // Should be a cache hit
        let status = check_cache(&cache, &main, None, false).unwrap();
        assert!(status.is_hit());
    }
}
