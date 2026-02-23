//! Integration tests for error code extraction and caching

use serial_test::serial;
use stacy::error::error_db::{ErrorCodeCache, ErrorCodeEntry, ErrorDatabase};
use stacy::error::extraction::parse_extraction_log;
use tempfile::TempDir;

/// Helper to run a test with a temporary cache directory.
/// Sets both XDG_CACHE_HOME (Unix) and LOCALAPPDATA (Windows).
fn with_test_cache<F: FnOnce(&TempDir)>(f: F) {
    let tmp = TempDir::new().unwrap();
    let old_xdg = std::env::var("XDG_CACHE_HOME").ok();
    let old_localappdata = std::env::var("LOCALAPPDATA").ok();
    std::env::set_var("XDG_CACHE_HOME", tmp.path());
    std::env::set_var("LOCALAPPDATA", tmp.path());
    f(&tmp);
    match old_xdg {
        Some(val) => std::env::set_var("XDG_CACHE_HOME", val),
        None => std::env::remove_var("XDG_CACHE_HOME"),
    }
    match old_localappdata {
        Some(val) => std::env::set_var("LOCALAPPDATA", val),
        None => std::env::remove_var("LOCALAPPDATA"),
    }
}

#[test]
#[serial]
fn test_cache_roundtrip() {
    with_test_cache(|_tmp| {
        let mut db = ErrorDatabase::empty();
        db.stata_version = Some("19.5".to_string());
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

        // Save and reload
        ErrorCodeCache::save(&db).unwrap();
        let loaded = ErrorCodeCache::load().unwrap().unwrap();

        // Verify
        assert_eq!(loaded.stata_version, Some("19.5".to_string()));
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.lookup(199).unwrap().message, "unrecognized command");
        assert_eq!(loaded.lookup(601).unwrap().message, "file not found");
    });
}

#[test]
fn test_parse_extraction_log_missing_start() {
    let log = "some content\nSTACY_EXTRACTION_END\n";
    let result = parse_extraction_log(log);
    assert!(result.is_err());
}

#[test]
fn test_parse_extraction_log_missing_end() {
    let log = "STACY_EXTRACTION_START\nsome content\n";
    let result = parse_extraction_log(log);
    assert!(result.is_err());
}

#[test]
fn test_parse_extraction_log_empty_region() {
    let log = "STACY_EXTRACTION_START\nSTACY_EXTRACTION_END\n";
    let db = parse_extraction_log(log).unwrap();
    assert!(db.is_empty());
    assert!(db.stata_version.is_none());
}

#[test]
#[serial]
fn test_cache_load_when_missing() {
    with_test_cache(|_tmp| {
        let result = ErrorCodeCache::load().unwrap();
        assert!(result.is_none());
    });
}

#[test]
#[ignore] // Requires Stata installation
fn test_full_extraction_e2e() {
    let binary = std::env::var("STATA_BINARY").unwrap_or_else(|_| "stata-mp".to_string());

    let db = stacy::error::extraction::extract_error_codes(&binary).unwrap();

    assert!(db.len() > 100, "Expected >100 codes, got {}", db.len());
    assert!(db.stata_version.is_some());
    assert!(db.lookup(199).is_some()); // Common code should exist
    assert!(db.lookup(601).is_some()); // File error should exist
}
