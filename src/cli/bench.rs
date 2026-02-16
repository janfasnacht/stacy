//! `stacy bench` command implementation
//!
//! Performance profiling for Stata scripts with statistics.

use crate::cli::output_format::OutputFormat;
use crate::cli::output_types::{BenchOutput, CommandOutput};
use crate::error::Result;
use crate::executor::{verbosity::Verbosity, StataExecutor};
use crate::project::Project;
use clap::Args;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

#[derive(Args)]
#[command(about = "Benchmark script execution")]
#[command(after_help = "\
Examples:
  stacy bench analysis.do              Run 10 times with 2 warmups (defaults)
  stacy bench analysis.do -n 20        Run 20 times
  stacy bench analysis.do --warmup 0   No warmup runs
  stacy bench analysis.do --format json   Machine-readable output")]
pub struct BenchArgs {
    /// Stata script to benchmark
    #[arg(value_name = "SCRIPT")]
    pub script: PathBuf,

    /// Number of measured runs
    #[arg(short = 'n', long, default_value = "10")]
    pub runs: usize,

    /// Number of warmup runs (not measured)
    #[arg(short = 'w', long, default_value = "2")]
    pub warmup: usize,

    /// Skip warmup runs (equivalent to --warmup 0)
    #[arg(long)]
    pub no_warmup: bool,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,

    /// Stata engine to use (overrides config and auto-detection)
    #[arg(long, value_name = "ENGINE")]
    pub engine: Option<String>,

    /// Suppress progress output
    #[arg(short, long)]
    pub quiet: bool,
}

/// Benchmark statistics
#[derive(Debug, Clone)]
pub struct BenchStats {
    pub count: usize,
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
    pub median: Duration,
    pub stddev: Duration,
}

impl BenchStats {
    /// Compute statistics from a list of durations
    pub fn from_durations(durations: &[Duration]) -> Option<Self> {
        if durations.is_empty() {
            return None;
        }

        let count = durations.len();

        // Sort for median calculation
        let mut sorted: Vec<Duration> = durations.to_vec();
        sorted.sort();

        let min = sorted[0];
        let max = sorted[count - 1];

        // Median
        let median = if count.is_multiple_of(2) {
            let mid = count / 2;
            (sorted[mid - 1] + sorted[mid]) / 2
        } else {
            sorted[count / 2]
        };

        // Mean
        let total: Duration = durations.iter().sum();
        let mean = total / count as u32;

        // Standard deviation
        let mean_secs = mean.as_secs_f64();
        let variance: f64 = durations
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() - mean_secs;
                diff * diff
            })
            .sum::<f64>()
            / count as f64;
        let stddev = Duration::from_secs_f64(variance.sqrt());

        Some(BenchStats {
            count,
            min,
            max,
            mean,
            median,
            stddev,
        })
    }
}

/// Execute the bench command
pub fn execute(args: &BenchArgs) -> Result<()> {
    let format = args.format;

    // Verify script exists
    if !args.script.exists() {
        if format == OutputFormat::Human {
            eprintln!("Error: Script not found: {}", args.script.display());
        }
        process::exit(3);
    }

    // Determine warmup count
    let warmup_count = if args.no_warmup { 0 } else { args.warmup };

    // Find project
    let project = Project::find()?;
    let project_root = project.as_ref().map(|p| p.root.as_path());

    // Create executor (quiet mode for benchmarking)
    let engine_ref = args.engine.as_deref();
    let executor = StataExecutor::try_new(engine_ref, Verbosity::Quiet)?;

    // Print header
    if !args.quiet && format == OutputFormat::Human {
        println!("Benchmarking: {}", args.script.display());
        if warmup_count > 0 {
            println!(
                "  {} warmup {}, {} measured {}",
                warmup_count,
                if warmup_count == 1 { "run" } else { "runs" },
                args.runs,
                if args.runs == 1 { "run" } else { "runs" }
            );
        } else {
            println!(
                "  {} measured {}",
                args.runs,
                if args.runs == 1 { "run" } else { "runs" }
            );
        }
        println!();
    }

    // Warmup runs
    if warmup_count > 0 {
        if !args.quiet && format == OutputFormat::Human {
            print!("Warming up");
        }

        for i in 0..warmup_count {
            let result = executor.run(&args.script, project_root)?;

            if !result.success {
                if format == OutputFormat::Human {
                    eprintln!("\nError: Script failed during warmup run {}", i + 1);
                }
                process::exit(result.exit_code);
            }

            if !args.quiet && format == OutputFormat::Human {
                print!(".");
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }

        if !args.quiet && format == OutputFormat::Human {
            println!(" done");
        }
    }

    // Measured runs
    let mut durations: Vec<Duration> = Vec::with_capacity(args.runs);

    if !args.quiet && format == OutputFormat::Human {
        print!("Measuring");
    }

    for i in 0..args.runs {
        let result = executor.run(&args.script, project_root)?;

        if !result.success {
            if format == OutputFormat::Human {
                eprintln!("\nError: Script failed on run {}", i + 1);
            }
            process::exit(result.exit_code);
        }

        durations.push(result.duration);

        if !args.quiet && format == OutputFormat::Human {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
    }

    if !args.quiet && format == OutputFormat::Human {
        println!(" done\n");
    }

    // Compute statistics
    let stats = BenchStats::from_durations(&durations).expect("Should have at least one run");

    // Build output
    // Note: if we get here, all runs succeeded (we exit early on failure)
    let output = BenchOutput {
        script: args.script.clone(),
        measured_runs: stats.count,
        warmup_runs: warmup_count,
        mean_secs: stats.mean.as_secs_f64(),
        median_secs: stats.median.as_secs_f64(),
        min_secs: stats.min.as_secs_f64(),
        max_secs: stats.max.as_secs_f64(),
        stddev_secs: stats.stddev.as_secs_f64(),
        success: true,
    };

    // Handle output
    match format {
        OutputFormat::Json => println!("{}", output.to_json()),
        OutputFormat::Stata => println!("{}", output.to_stata()),
        OutputFormat::Human => {
            println!("Benchmark Results: {}", args.script.display());
            println!(
                "  Runs:     {} ({})",
                stats.count,
                if warmup_count > 0 {
                    format!("{} warmup", warmup_count)
                } else {
                    "no warmup".to_string()
                }
            );
            println!();
            println!("  Time (seconds):");
            println!(
                "    mean      {:.3} +/- {:.3}",
                stats.mean.as_secs_f64(),
                stats.stddev.as_secs_f64()
            );
            println!("    median    {:.3}", stats.median.as_secs_f64());
            println!("    min       {:.3}", stats.min.as_secs_f64());
            println!("    max       {:.3}", stats.max.as_secs_f64());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bench_stats_single() {
        let durations = vec![Duration::from_millis(100)];
        let stats = BenchStats::from_durations(&durations).unwrap();

        assert_eq!(stats.count, 1);
        assert_eq!(stats.min, Duration::from_millis(100));
        assert_eq!(stats.max, Duration::from_millis(100));
        assert_eq!(stats.mean, Duration::from_millis(100));
        assert_eq!(stats.median, Duration::from_millis(100));
        assert_eq!(stats.stddev, Duration::ZERO);
    }

    #[test]
    fn test_bench_stats_multiple() {
        let durations = vec![
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(300),
        ];
        let stats = BenchStats::from_durations(&durations).unwrap();

        assert_eq!(stats.count, 3);
        assert_eq!(stats.min, Duration::from_millis(100));
        assert_eq!(stats.max, Duration::from_millis(300));
        assert_eq!(stats.mean, Duration::from_millis(200));
        assert_eq!(stats.median, Duration::from_millis(200));
    }

    #[test]
    fn test_bench_stats_even_count() {
        let durations = vec![
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(300),
            Duration::from_millis(400),
        ];
        let stats = BenchStats::from_durations(&durations).unwrap();

        assert_eq!(stats.count, 4);
        // Median of [100, 200, 300, 400] = (200 + 300) / 2 = 250
        assert_eq!(stats.median, Duration::from_millis(250));
        assert_eq!(stats.mean, Duration::from_millis(250));
    }

    #[test]
    fn test_bench_stats_empty() {
        let durations: Vec<Duration> = vec![];
        let stats = BenchStats::from_durations(&durations);
        assert!(stats.is_none());
    }

    #[test]
    fn test_bench_stats_stddev() {
        // Values: 2, 4, 4, 4, 5, 5, 7, 9
        // Mean: 5
        // Variance: sum((x-5)^2) / 8 = (9+1+1+1+0+0+4+16)/8 = 32/8 = 4
        // Stddev: sqrt(4) = 2
        let durations = vec![
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(4),
            Duration::from_secs(4),
            Duration::from_secs(5),
            Duration::from_secs(5),
            Duration::from_secs(7),
            Duration::from_secs(9),
        ];
        let stats = BenchStats::from_durations(&durations).unwrap();

        assert_eq!(stats.mean, Duration::from_secs(5));
        // Allow small floating point tolerance
        let stddev_diff = (stats.stddev.as_secs_f64() - 2.0).abs();
        assert!(stddev_diff < 0.001, "stddev should be ~2.0");
    }

    #[test]
    fn test_bench_stats_unsorted_input() {
        // Make sure order doesn't matter
        let durations = vec![
            Duration::from_millis(300),
            Duration::from_millis(100),
            Duration::from_millis(200),
        ];
        let stats = BenchStats::from_durations(&durations).unwrap();

        assert_eq!(stats.min, Duration::from_millis(100));
        assert_eq!(stats.max, Duration::from_millis(300));
        assert_eq!(stats.median, Duration::from_millis(200));
    }
}
