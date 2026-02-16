//! CLI implementation for `stacy test` command
//!
//! Run tests by convention from the project directory.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{
    CommandOutput, TestInfo, TestListOutput, TestOutput, TestResultOutput,
};
use crate::cli::test_output;
use crate::error::Result;
use crate::executor::StataExecutor;
use crate::project::Project;
use crate::test::discovery::{discover_tests, find_test};
use crate::test::runner::TestRunner;
use clap::Args;
use std::process;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy test                              Run all tests
  stacy test test_regression              Run a specific test
  stacy test -f \"clean*\"                  Filter tests by pattern
  stacy test --list                       List tests without running")]
pub struct TestArgs {
    /// Specific test to run (name or path)
    #[arg(value_name = "TEST")]
    pub test: Option<String>,

    /// Filter tests by pattern (can be used multiple times)
    #[arg(long, short = 'f', value_name = "PATTERN")]
    pub filter: Vec<String>,

    /// Run tests in parallel
    #[arg(long)]
    pub parallel: bool,

    /// List tests without running them
    #[arg(long)]
    pub list: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,

    /// Suppress progress output
    #[arg(short, long)]
    pub quiet: bool,

    /// Show full log context for failures
    #[arg(short = 'V', long)]
    pub verbose: bool,
}

pub fn execute(args: &TestArgs) -> Result<()> {
    let format = args.format;

    // Find project (optional for test command)
    let project = Project::find()?;
    let project_root = project
        .as_ref()
        .map(|p| p.root.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Handle specific test
    if let Some(ref test_name) = args.test {
        if let Some(test) = find_test(&project_root, test_name)? {
            return run_single_test(args, &project_root, &test);
        } else {
            let msg = format!("Test '{}' not found", test_name);
            if format.is_machine_readable() {
                let output = TestOutput {
                    test_count: 0,
                    passed: 0,
                    failed: 0,
                    skipped: 0,
                    duration_secs: 0.0,
                    success: false,
                    tests: vec![],
                };
                match format {
                    OutputFormat::Json => println!("{}", output.to_json()),
                    OutputFormat::Stata => println!("{}", output.to_stata()),
                    OutputFormat::Human => {}
                }
            } else {
                eprintln!("Error: {}", msg);
            }
            process::exit(5); // Internal error
        }
    }

    // Discover tests
    let tests = discover_tests(&project_root, &args.filter)?;

    // Handle --list flag
    if args.list {
        return execute_list(&tests, format);
    }

    // Check if there are any tests
    if tests.is_empty() {
        if format.is_machine_readable() {
            let output = TestOutput {
                test_count: 0,
                passed: 0,
                failed: 0,
                skipped: 0,
                duration_secs: 0.0,
                success: true, // No tests = success
                tests: vec![],
            };
            match format {
                OutputFormat::Json => println!("{}", output.to_json()),
                OutputFormat::Stata => println!("{}", output.to_stata()),
                OutputFormat::Human => {}
            }
        } else {
            println!("No tests found.");
            println!();
            println!("Tests are discovered by:");
            println!("  - Files matching test_*.do or *_test.do anywhere in project");
            println!("  - All .do files in tests/ or test/ directories");
        }
        return Ok(());
    }

    // Run tests
    run_tests(args, &project_root, &tests)
}

fn run_single_test(
    args: &TestArgs,
    project_root: &std::path::Path,
    test: &crate::test::discovery::TestFile,
) -> Result<()> {
    let format = args.format;

    // Create Stata executor with Quiet verbosity to suppress error context
    // (we show our own error messages in test results)
    let executor = StataExecutor::try_new(None, crate::executor::verbosity::Verbosity::Quiet)?;

    // Create test runner
    let runner = TestRunner::new(&executor, project_root);

    // Run the test
    if !args.quiet && format == OutputFormat::Human {
        println!("Running test: {}", test.name);
        println!();
    }

    let result = runner.run_test(test)?;

    // Build output
    let output = TestOutput {
        test_count: 1,
        passed: if result.passed { 1 } else { 0 },
        failed: if result.passed { 0 } else { 1 },
        skipped: 0,
        duration_secs: result.duration.as_secs_f64(),
        success: result.passed,
        tests: vec![TestResultOutput {
            name: result.name,
            path: result.path,
            status: if result.passed {
                "passed".to_string()
            } else {
                "failed".to_string()
            },
            duration_secs: result.duration.as_secs_f64(),
            exit_code: result.exit_code,
            error_message: result.error_message,
        }],
    };

    output_result(&output, format);

    if output.success {
        Ok(())
    } else {
        process::exit(1);
    }
}

fn run_tests(
    args: &TestArgs,
    project_root: &std::path::Path,
    tests: &[crate::test::discovery::TestFile],
) -> Result<()> {
    let format = args.format;

    // Create Stata executor with Quiet verbosity to suppress error context
    // (we show our own error messages in test results)
    let executor = StataExecutor::try_new(None, crate::executor::verbosity::Verbosity::Quiet)?;

    // Create test runner
    let runner = TestRunner::new(&executor, project_root).with_parallel(args.parallel);

    // Print header
    if !args.quiet && format == OutputFormat::Human {
        let mode = if args.parallel { " (parallel)" } else { "" };
        println!("Running {} tests{}...", tests.len(), mode);
        println!();
    }

    // Run tests with progress reporting
    let suite_result = if args.quiet || format.is_machine_readable() {
        runner.run_all(tests)?
    } else {
        // Run with progress output
        run_with_progress(&runner, tests, args.verbose)?
    };

    // Build output
    let output = TestOutput {
        test_count: suite_result.test_count,
        passed: suite_result.passed,
        failed: suite_result.failed,
        skipped: suite_result.skipped,
        duration_secs: suite_result.duration.as_secs_f64(),
        success: suite_result.success(),
        tests: suite_result
            .results
            .iter()
            .map(|r| TestResultOutput {
                name: r.name.clone(),
                path: r.path.clone(),
                status: if r.passed {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                },
                duration_secs: r.duration.as_secs_f64(),
                exit_code: r.exit_code,
                error_message: r.error_message.clone(),
            })
            .collect(),
    };

    output_result(&output, format);

    if output.success {
        Ok(())
    } else {
        process::exit(1);
    }
}

fn run_with_progress(
    runner: &TestRunner,
    tests: &[crate::test::discovery::TestFile],
    verbose: bool,
) -> Result<crate::test::runner::TestSuiteResult> {
    use crate::test::runner::TestSuiteResult;

    let mut suite_result = TestSuiteResult::new();

    for test in tests {
        let result = runner.run_test(test)?;

        // Print rich formatted output
        test_output::print_test_result(&result, verbose);

        suite_result.add_result(result);
    }

    println!();
    Ok(suite_result)
}

fn output_result(output: &TestOutput, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", output.to_json());
        }
        OutputFormat::Stata => {
            println!("{}", output.to_stata());
        }
        OutputFormat::Human => {
            // Rich formatted summary
            test_output::print_summary(
                output.passed,
                output.failed,
                std::time::Duration::from_secs_f64(output.duration_secs),
            );
        }
    }
}

fn execute_list(tests: &[crate::test::discovery::TestFile], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let output = TestListOutput {
                test_count: tests.len(),
                tests: tests
                    .iter()
                    .map(|t| TestInfo {
                        name: t.name.clone(),
                        path: t.path.clone(),
                    })
                    .collect(),
            };
            println!("{}", output.to_json());
        }
        OutputFormat::Stata => {
            let output = TestListOutput {
                test_count: tests.len(),
                tests: tests
                    .iter()
                    .map(|t| TestInfo {
                        name: t.name.clone(),
                        path: t.path.clone(),
                    })
                    .collect(),
            };
            println!("{}", output.to_stata());
        }
        OutputFormat::Human => {
            if tests.is_empty() {
                println!("No tests found.");
            } else {
                println!("Found {} tests:", tests.len());
                println!();
                for test in tests {
                    println!("  {} ({})", test.name, test.path.display());
                }
            }
        }
    }

    Ok(())
}
