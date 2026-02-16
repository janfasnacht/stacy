//! Error code database backed by cached Stata extraction
//!
//! Provides a thread-safe, lazily-loaded error code database that reads from
//! a JSON cache file (`~/.cache/stacy/error-codes.json`). Falls back to
//! range-based categories when no cache exists.

use super::categories::category_for_code;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

/// A single error code entry extracted from Stata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCodeEntry {
    pub code: u32,
    pub message: String,
    pub category: String,
}

/// Database of extracted error codes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDatabase {
    pub stata_version: Option<String>,
    pub sysdir: Option<String>,
    pub extracted_at: String,
    pub stacy_version: String,
    pub errors: Vec<ErrorCodeEntry>,

    /// Lookup map built at load time (not serialized)
    #[serde(skip)]
    lookup: HashMap<u32, usize>,
}

impl ErrorDatabase {
    /// Create an empty database (no cached data)
    pub fn empty() -> Self {
        Self {
            stata_version: None,
            sysdir: None,
            extracted_at: String::new(),
            stacy_version: env!("CARGO_PKG_VERSION").to_string(),
            errors: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    /// Build the lookup index from the errors vec
    pub fn build_index(&mut self) {
        self.lookup = self
            .errors
            .iter()
            .enumerate()
            .map(|(i, e)| (e.code, i))
            .collect();
    }

    /// Look up an error code entry
    pub fn lookup(&self, code: u32) -> Option<&ErrorCodeEntry> {
        self.lookup.get(&code).and_then(|&idx| self.errors.get(idx))
    }

    /// Get all known error codes
    pub fn all_codes(&self) -> Vec<u32> {
        self.errors.iter().map(|e| e.code).collect()
    }

    /// Number of entries
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Whether the database is empty (no extraction data)
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Cache file management
pub struct ErrorCodeCache;

impl ErrorCodeCache {
    /// Path to the error codes cache file: `~/.cache/stacy/error-codes.json`
    pub fn path() -> crate::error::Result<PathBuf> {
        let cache_base = if cfg!(windows) {
            std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("AppData")
                        .join("Local")
                })
                .join("stacy")
                .join("cache")
        } else {
            std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(".cache")
                })
                .join("stacy")
        };

        Ok(cache_base.join("error-codes.json"))
    }

    /// Load the error database from cache, if it exists
    pub fn load() -> crate::error::Result<Option<ErrorDatabase>> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(&path)?;
        let mut db: ErrorDatabase = serde_json::from_str(&contents)?;
        db.build_index();
        Ok(Some(db))
    }

    /// Save the error database to cache (atomic write via temp + rename)
    pub fn save(db: &ErrorDatabase) -> crate::error::Result<()> {
        let path = Self::path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(db)?;

        // Atomic write: write to temp file then rename
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(&tmp_path, &path)?;

        Ok(())
    }

    /// Check if a cache file exists
    pub fn exists() -> bool {
        Self::path().map(|p| p.exists()).unwrap_or(false)
    }
}

/// Thread-safe lazy accessor for the error database.
///
/// Once loaded, the database is cached for the process lifetime.
/// `stacy doctor --refresh` updates the disk cache, but a new stacy invocation
/// is needed to pick up the refreshed data.
static ERROR_DB: OnceLock<ErrorDatabase> = OnceLock::new();

/// Get the global error database (loads from cache on first access)
pub fn get_error_database() -> &'static ErrorDatabase {
    ERROR_DB.get_or_init(|| {
        ErrorCodeCache::load()
            .ok()
            .flatten()
            .unwrap_or_else(ErrorDatabase::empty)
    })
}

/// Drop-in replacement for `lookup_official_error()`
///
/// Returns the cached entry if available, None otherwise.
/// Callers should fall back to `category_for_code()` when this returns None.
pub fn lookup_error(code: u32) -> Option<&'static ErrorCodeEntry> {
    get_error_database().lookup(code)
}

/// Look up an error code, returning either the cached message or a
/// range-based category description as fallback.
pub fn lookup_error_message(code: u32) -> String {
    match lookup_error(code) {
        Some(entry) => entry.message.clone(),
        None => format!("{} error", category_for_code(code)),
    }
}

/// Get the category for an error code (cached or range-based fallback)
pub fn lookup_category(code: u32) -> &'static str {
    // Categories are always derived from ranges, so we can use category_for_code directly.
    // The cached entry's category was also assigned by category_for_code() during extraction.
    category_for_code(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_database() {
        let db = ErrorDatabase::empty();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
        assert!(db.lookup(199).is_none());
        assert!(db.all_codes().is_empty());
    }

    #[test]
    fn test_database_with_entries() {
        let mut db = ErrorDatabase::empty();
        db.errors = vec![
            ErrorCodeEntry {
                code: 199,
                message: "unrecognized command".to_string(),
                category: "Syntax/Command".to_string(),
            },
            ErrorCodeEntry {
                code: 601,
                message: "file not found".to_string(),
                category: "File I/O".to_string(),
            },
        ];
        db.build_index();

        assert_eq!(db.len(), 2);
        assert!(!db.is_empty());

        let entry = db.lookup(199).unwrap();
        assert_eq!(entry.message, "unrecognized command");
        assert_eq!(entry.category, "Syntax/Command");

        let entry = db.lookup(601).unwrap();
        assert_eq!(entry.message, "file not found");

        assert!(db.lookup(999).is_none());
    }

    #[test]
    fn test_all_codes() {
        let mut db = ErrorDatabase::empty();
        db.errors = vec![
            ErrorCodeEntry {
                code: 1,
                message: "break".to_string(),
                category: "General".to_string(),
            },
            ErrorCodeEntry {
                code: 199,
                message: "unrecognized command".to_string(),
                category: "Syntax/Command".to_string(),
            },
        ];
        db.build_index();

        let codes = db.all_codes();
        assert_eq!(codes, vec![1, 199]);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let mut db = ErrorDatabase::empty();
        db.stata_version = Some("19.5".to_string());
        db.sysdir = Some("/usr/local/stata19".to_string());
        db.extracted_at = "2026-02-05T12:00:00Z".to_string();
        db.errors = vec![ErrorCodeEntry {
            code: 199,
            message: "unrecognized command".to_string(),
            category: "Syntax/Command".to_string(),
        }];
        db.build_index();

        let json = serde_json::to_string(&db).unwrap();
        let mut db2: ErrorDatabase = serde_json::from_str(&json).unwrap();
        db2.build_index();

        assert_eq!(db2.stata_version, Some("19.5".to_string()));
        assert_eq!(db2.errors.len(), 1);
        assert_eq!(db2.lookup(199).unwrap().message, "unrecognized command");
    }

    #[test]
    fn test_lookup_error_message_fallback() {
        // With empty OnceLock, should fall back to category
        let msg = lookup_error_message(601);
        // Either cached message or fallback
        assert!(msg.contains("File I/O") || msg.contains("file"));
    }
}
