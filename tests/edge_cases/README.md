# Edge Case Tests

This directory contains test scripts for edge cases that `stacy` should handle gracefully.

## Test Scripts

### 1. `my script.do` - Spaces in Filenames
Tests that `stacy` correctly handles filenames containing spaces.

**Run with:**
```bash
cargo test --test edge_cases_test test_spaces_in_filename -- --ignored
```

**What it tests:**
- Command-line argument parsing with spaces
- File path handling in executor
- Log file creation with spaces in name

---

### 2. `café_analysis.do` - Unicode Characters
Tests UTF-8 path handling for filenames with non-ASCII characters.

**Run with:**
```bash
cargo test --test edge_cases_test test_unicode_in_filename -- --ignored
```

**What it tests:**
- UTF-8 encoding in file paths
- Cross-platform Unicode support (macOS, Linux, Windows)
- Log file creation with Unicode characters

---

### 3. `large_log_generator.do` - Memory Efficiency
Generates a large log file (~5-10 MB with 50,000 lines) to test memory-efficient parsing.

**Run with:**
```bash
cargo test --test edge_cases_test test_large_log_file -- --ignored --nocapture
```

**What it tests:**
- Large log file parsing doesn't load entire file into memory
- `read_last_lines()` optimization works correctly
- Error detection works on large logs
- Memory usage stays bounded

**Memory efficiency test (no Stata required):**
```bash
cargo test --test edge_cases_test test_large_log_memory_efficiency
```

---

## Running All Edge Case Tests

**With Stata installed:**
```bash
cargo test --test edge_cases_test -- --ignored --nocapture
```

**Without Stata (memory test only):**
```bash
cargo test --test edge_cases_test test_large_log_memory_efficiency
```

---

## Success Criteria

- ✅ Filenames with spaces work without escaping
- ✅ Unicode filenames (UTF-8) work on all platforms
- ✅ Large logs (>1 MB) parse efficiently without memory issues
- ✅ `read_last_lines()` reads only necessary data (< 100ms for 100k lines)

---

## Notes

- All tests marked `#[ignore]` require Stata to be installed
- Log files are generated in this directory during test runs
- Memory efficiency is tested without Stata (unit test)
- These tests verify robustness for real-world usage
