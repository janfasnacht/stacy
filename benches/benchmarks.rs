use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use std::path::Path;

/// Benchmark log parsing performance
fn bench_log_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("log_parsing");

    // Small log (success case)
    if let Ok(small_log) = fs::read_to_string("tests/log-analysis/01_success.log") {
        group.bench_with_input(
            BenchmarkId::new("small_log", small_log.len()),
            &small_log,
            |b, log| {
                b.iter(|| {
                    // Simulate parsing: look for "end of do-file" and r() codes
                    let lines: Vec<&str> = log.lines().collect();
                    let has_end = lines.iter().any(|l| l.contains("end of do-file"));
                    let last_5: Vec<&str> = lines.iter().rev().take(5).copied().collect();
                    let has_error = last_5.iter().any(|l| l.starts_with("r("));
                    black_box((has_end, has_error))
                });
            },
        );
    }

    // Medium log (with error)
    if let Ok(medium_log) = fs::read_to_string("tests/log-analysis/02_syntax_error.log") {
        group.bench_with_input(
            BenchmarkId::new("medium_log", medium_log.len()),
            &medium_log,
            |b, log| {
                b.iter(|| {
                    let lines: Vec<&str> = log.lines().collect();
                    let has_end = lines.iter().any(|l| l.contains("end of do-file"));
                    let last_5: Vec<&str> = lines.iter().rev().take(5).copied().collect();
                    let has_error = last_5.iter().any(|l| l.starts_with("r("));
                    black_box((has_end, has_error))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark error code extraction
fn bench_error_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_extraction");

    let test_lines = vec!["", "end of do-file", "r(199);", ""];

    group.bench_function("extract_r_code", |b| {
        b.iter(|| {
            for line in &test_lines {
                if line.starts_with("r(") && line.ends_with(");") {
                    let code_str = &line[2..line.len() - 2];
                    let _code: Result<u32, _> = code_str.parse();
                    black_box(_code);
                }
            }
        });
    });

    group.finish();
}

/// Benchmark regex-based error pattern matching
fn bench_error_patterns(c: &mut Criterion) {
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        static ref R_CODE_PATTERN: Regex = Regex::new(r"^r\((\d+)\);$").unwrap();
        static ref ERROR_MSG_PATTERN: Regex =
            Regex::new(r"(?i)(not found|invalid|error|failed)").unwrap();
    }

    let mut group = c.benchmark_group("error_patterns");

    let test_lines = vec![
        "r(199);",
        "r(601);",
        "file not found",
        "unrecognized command",
        "end of do-file",
        "normal output line",
    ];

    group.bench_function("regex_r_code", |b| {
        b.iter(|| {
            for line in &test_lines {
                if let Some(caps) = R_CODE_PATTERN.captures(line) {
                    let _code = caps.get(1).unwrap().as_str();
                    black_box(_code);
                }
            }
        });
    });

    group.bench_function("regex_error_msg", |b| {
        b.iter(|| {
            for line in &test_lines {
                let _matches = ERROR_MSG_PATTERN.is_match(line);
                black_box(_matches);
            }
        });
    });

    group.finish();
}

/// Benchmark exit code mapping
fn bench_exit_code_mapping(c: &mut Criterion) {
    let mut group = c.benchmark_group("exit_code_mapping");

    let test_codes = vec![
        111, // not found
        199, // unrecognized command
        601, // file not found
        900, // matsize
        402, // negative weights
    ];

    group.bench_function("map_r_to_exit", |b| {
        b.iter(|| {
            for code in &test_codes {
                // Simplified mapping logic
                let exit_code = match code {
                    100..=199 => 2, // Syntax errors
                    600..=699 => 3, // File errors
                    800..=999 => 4, // Memory/system errors
                    _ => 1,         // Generic error
                };
                black_box(exit_code);
            }
        });
    });

    group.finish();
}

/// Benchmark file I/O operations
fn bench_file_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_io");

    let test_log = "tests/log-analysis/01_success.log";

    if Path::new(test_log).exists() {
        group.bench_function("read_full_file", |b| {
            b.iter(|| {
                let _contents = fs::read_to_string(test_log);
                black_box(_contents)
            });
        });

        group.bench_function("read_and_parse_lines", |b| {
            b.iter(|| {
                if let Ok(contents) = fs::read_to_string(test_log) {
                    let _lines: Vec<&str> = contents.lines().collect();
                    black_box(_lines);
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_log_parsing,
    bench_error_extraction,
    bench_error_patterns,
    bench_exit_code_mapping,
    bench_file_operations
);
criterion_main!(benches);
