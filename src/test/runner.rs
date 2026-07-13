//! Test execution engine
//!
//! Executes discovered tests sequentially or in parallel using StataExecutor.

use crate::error::{Result, StataError};
use crate::executor::log_policy::LogPolicy;
use crate::executor::StataExecutor;
use crate::test::discovery::TestFile;

/// Format a StataError into a human-readable string
fn format_stata_error(err: &StataError) -> String {
    match err {
        StataError::StataCode {
            r_code,
            message,
            line_number,
            ..
        } => {
            let line_info = line_number
                .map(|l| format!(" at line {}", l))
                .unwrap_or_default();
            format!("r({}){} - {}", r_code, line_info, message)
        }
        StataError::ProcessKilled { exit_code } => {
            format!("Process killed (exit code {})", exit_code)
        }
    }
}
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Working directory for test execution
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TestWorkingDir {
    /// Project root (default)
    #[default]
    ProjectRoot,
    /// A fixed directory (from -C/--directory)
    Fixed(PathBuf),
    /// Each test's own parent directory (from --cd)
    TestDir,
}

/// Resolve the effective working directory for a single test
fn resolve_working_dir(mode: &TestWorkingDir, project_root: &Path, test_path: &Path) -> PathBuf {
    match mode {
        TestWorkingDir::ProjectRoot => project_root.to_path_buf(),
        TestWorkingDir::Fixed(dir) => dir.clone(),
        TestWorkingDir::TestDir => test_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| project_root.to_path_buf()),
    }
}

/// Result of running a single test
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// Path to the test file
    pub path: std::path::PathBuf,
    /// Whether the test passed
    pub passed: bool,
    /// Exit code from execution
    pub exit_code: i32,
    /// How long the test took
    pub duration: Duration,
    /// Error message if test failed
    pub error_message: Option<String>,
    /// Path to log file (for verbose error context)
    pub log_file: Option<std::path::PathBuf>,
}

/// Result of running all tests
#[derive(Debug)]
pub struct TestSuiteResult {
    /// Total number of tests
    pub test_count: usize,
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// Number of skipped tests
    pub skipped: usize,
    /// Total duration
    pub duration: Duration,
    /// Individual test results
    pub results: Vec<TestResult>,
}

impl TestSuiteResult {
    /// Create an empty result
    pub fn new() -> Self {
        Self {
            test_count: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            duration: Duration::ZERO,
            results: vec![],
        }
    }

    /// Whether all tests passed
    pub fn success(&self) -> bool {
        self.failed == 0
    }

    /// Add a test result
    pub fn add_result(&mut self, result: TestResult) {
        self.test_count += 1;
        if result.passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.duration += result.duration;
        self.results.push(result);
    }
}

impl Default for TestSuiteResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Test runner configuration
pub struct TestRunner<'a> {
    /// Stata executor
    stata: &'a StataExecutor,
    /// Project root directory
    project_root: &'a Path,
    /// Run tests in parallel
    parallel: bool,
    /// Working directory mode for test execution
    working_dir: TestWorkingDir,
    /// What happens to each test's log once it has run
    log_policy: LogPolicy,
}

impl<'a> TestRunner<'a> {
    /// Create a new test runner
    pub fn new(stata: &'a StataExecutor, project_root: &'a Path) -> Self {
        Self {
            stata,
            project_root,
            parallel: false,
            working_dir: TestWorkingDir::default(),
            log_policy: LogPolicy::new(),
        }
    }

    /// Enable parallel test execution
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Set the log-retention policy applied after each test (#98).
    pub fn with_log_policy(mut self, policy: LogPolicy) -> Self {
        self.log_policy = policy;
        self
    }

    /// Set the working directory mode for test execution
    pub fn with_working_dir(mut self, working_dir: TestWorkingDir) -> Self {
        self.working_dir = working_dir;
        self
    }

    /// Run a single test
    pub fn run_test(&self, test: &TestFile) -> Result<TestResult> {
        let start = Instant::now();

        let working_dir = resolve_working_dir(&self.working_dir, self.project_root, &test.path);
        let result = self
            .stata
            .run_in_dir(&test.path, Some(self.project_root), &working_dir)?;
        let duration = start.elapsed();

        let error_message = if !result.success {
            if let Some(err) = result.errors.first() {
                Some(format_stata_error(err))
            } else {
                Some(format!("Exit code {}", result.exit_code))
            }
        } else {
            None
        };

        // A passing test's log is internal and is removed; a failing test keeps
        // its log (in `[run] log_dir` when set) — the failure report reads it.
        let log_file = self.log_policy.finalize(&result.log_file, result.success);

        Ok(TestResult {
            name: test.name.clone(),
            path: test.path.clone(),
            passed: result.success,
            exit_code: result.exit_code,
            duration,
            error_message,
            log_file: Some(log_file),
        })
    }

    /// Run all tests
    pub fn run_all(&self, tests: &[TestFile]) -> Result<TestSuiteResult> {
        if self.parallel {
            self.run_parallel(tests)
        } else {
            self.run_sequential(tests)
        }
    }

    /// Run tests sequentially
    fn run_sequential(&self, tests: &[TestFile]) -> Result<TestSuiteResult> {
        let mut suite_result = TestSuiteResult::new();

        for test in tests {
            let result = self.run_test(test)?;
            suite_result.add_result(result);
        }

        Ok(suite_result)
    }

    /// Run tests in parallel using scoped threads
    fn run_parallel(&self, tests: &[TestFile]) -> Result<TestSuiteResult> {
        if tests.is_empty() {
            return Ok(TestSuiteResult::new());
        }

        let results = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        std::thread::scope(|s| {
            for test in tests {
                let results = Arc::clone(&results);
                let errors = Arc::clone(&errors);

                s.spawn(move || match self.run_test(test) {
                    Ok(result) => {
                        results.lock().unwrap().push(result);
                    }
                    Err(e) => {
                        errors.lock().unwrap().push(e);
                    }
                });
            }
        });

        // Check for execution errors
        let errs = Arc::try_unwrap(errors).unwrap().into_inner().unwrap();
        if !errs.is_empty() {
            return Err(errs.into_iter().next().unwrap());
        }

        // Build suite result
        let test_results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
        let mut suite_result = TestSuiteResult::new();
        for result in test_results {
            suite_result.add_result(result);
        }

        // Sort results by name for consistent output
        suite_result.results.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(suite_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_working_dir_project_root() {
        let dir = resolve_working_dir(
            &TestWorkingDir::ProjectRoot,
            Path::new("/project"),
            Path::new("/project/tests/test_foo.do"),
        );
        assert_eq!(dir, PathBuf::from("/project"));
    }

    #[test]
    fn test_resolve_working_dir_fixed() {
        let dir = resolve_working_dir(
            &TestWorkingDir::Fixed(PathBuf::from("/project/data")),
            Path::new("/project"),
            Path::new("/project/tests/test_foo.do"),
        );
        assert_eq!(dir, PathBuf::from("/project/data"));
    }

    #[test]
    fn test_resolve_working_dir_test_dir() {
        let dir = resolve_working_dir(
            &TestWorkingDir::TestDir,
            Path::new("/project"),
            Path::new("/project/tests/nested/test_foo.do"),
        );
        assert_eq!(dir, PathBuf::from("/project/tests/nested"));
    }

    #[test]
    fn test_suite_result_new() {
        let result = TestSuiteResult::new();
        assert_eq!(result.test_count, 0);
        assert_eq!(result.passed, 0);
        assert_eq!(result.failed, 0);
        assert!(result.success());
    }

    #[test]
    fn test_suite_result_add_passed() {
        let mut suite = TestSuiteResult::new();
        suite.add_result(TestResult {
            name: "test_foo".to_string(),
            path: std::path::PathBuf::from("test_foo.do"),
            passed: true,
            exit_code: 0,
            duration: Duration::from_secs(1),
            error_message: None,
            log_file: None,
        });

        assert_eq!(suite.test_count, 1);
        assert_eq!(suite.passed, 1);
        assert_eq!(suite.failed, 0);
        assert!(suite.success());
    }

    #[test]
    fn test_suite_result_add_failed() {
        let mut suite = TestSuiteResult::new();
        suite.add_result(TestResult {
            name: "test_foo".to_string(),
            path: std::path::PathBuf::from("test_foo.do"),
            passed: false,
            exit_code: 1,
            duration: Duration::from_secs(1),
            error_message: Some("r(601) - file not found".to_string()),
            log_file: None,
        });

        assert_eq!(suite.test_count, 1);
        assert_eq!(suite.passed, 0);
        assert_eq!(suite.failed, 1);
        assert!(!suite.success());
    }

    #[test]
    fn test_suite_result_mixed() {
        let mut suite = TestSuiteResult::new();
        suite.add_result(TestResult {
            name: "test_pass".to_string(),
            path: std::path::PathBuf::from("test_pass.do"),
            passed: true,
            exit_code: 0,
            duration: Duration::from_secs(1),
            error_message: None,
            log_file: None,
        });
        suite.add_result(TestResult {
            name: "test_fail".to_string(),
            path: std::path::PathBuf::from("test_fail.do"),
            passed: false,
            exit_code: 1,
            duration: Duration::from_secs(2),
            error_message: Some("error".to_string()),
            log_file: None,
        });

        assert_eq!(suite.test_count, 2);
        assert_eq!(suite.passed, 1);
        assert_eq!(suite.failed, 1);
        assert!(!suite.success());
        assert_eq!(suite.duration, Duration::from_secs(3));
    }
}
