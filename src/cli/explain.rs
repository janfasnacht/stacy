//! `stacy explain` command implementation
//!
//! Displays detailed information about Stata error codes.

use crate::cli::output_format::OutputFormat;
use crate::error::categories::category_for_code;
use crate::error::error_db::{lookup_error, ErrorCodeEntry};
use crate::error::Result;
use clap::Args;

#[derive(Args)]
#[command(after_help = "\
Examples:
  stacy explain 199                   Look up error code 199
  stacy explain r(199)                Also accepts r() syntax
  stacy explain 111 --format json     Output as JSON")]
pub struct ExplainArgs {
    /// Error code to look up (e.g., 199 or r(199))
    pub code: String,

    /// Output format: human (default), json, or stata
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

pub fn execute(args: &ExplainArgs) -> Result<()> {
    // Parse the error code - accept both "199" and "r(199)"
    let code_str = args
        .code
        .trim()
        .trim_start_matches("r(")
        .trim_end_matches(')');

    let code: u32 = code_str.parse().map_err(|_| {
        crate::error::Error::Parse(format!(
            "Invalid error code '{}'. Expected a number like 199 or r(199)",
            args.code
        ))
    })?;

    match lookup_error(code) {
        Some(entry) => {
            match args.format {
                OutputFormat::Human => print_human_output(code, entry),
                OutputFormat::Json => print_json_output(code, entry),
                OutputFormat::Stata => print_stata_output(code, entry),
            }
            Ok(())
        }
        None => {
            // No cached entry — show range-based category info
            let category = category_for_code(code);
            match args.format {
                OutputFormat::Human => print_human_fallback(code, category),
                OutputFormat::Json => print_json_fallback(code, category),
                OutputFormat::Stata => print_stata_fallback(code, category),
            }
            Ok(())
        }
    }
}

fn print_human_output(code: u32, entry: &ErrorCodeEntry) {
    println!("r({}) - {}", code, entry.message);
    println!();
    println!("Category: {}", entry.category);
    println!();
    println!("See: https://www.stata.com/manuals/perror.pdf#r{}", code);
}

fn print_json_output(code: u32, entry: &ErrorCodeEntry) {
    use serde_json::json;

    let output = json!({
        "code": code,
        "message": entry.message,
        "category": entry.category,
        "url": format!("https://www.stata.com/manuals/perror.pdf#r{}", code),
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn print_stata_output(code: u32, entry: &ErrorCodeEntry) {
    println!("local _stacy_error_code {}", code);
    println!("local _stacy_error_message `\"{}\"'", entry.message);
    println!("local _stacy_error_category `\"{}\"'", entry.category);
}

fn print_human_fallback(code: u32, category: &str) {
    println!("r({}) - {} error", code, category);
    println!();
    println!("Category: {}", category);
    println!();
    println!("Note: Run 'stacy doctor --refresh' for detailed error messages");
    println!("      extracted from your Stata installation.");
    println!();
    println!("See: https://www.stata.com/manuals/perror.pdf#r{}", code);
}

fn print_json_fallback(code: u32, category: &str) {
    use serde_json::json;

    let output = json!({
        "code": code,
        "message": format!("{} error", category),
        "category": category,
        "url": format!("https://www.stata.com/manuals/perror.pdf#r{}", code),
        "note": "Run 'stacy doctor --refresh' for detailed messages",
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn print_stata_fallback(code: u32, category: &str) {
    println!("local _stacy_error_code {}", code);
    println!("local _stacy_error_message `\"{} error\"'", category);
    println!("local _stacy_error_category `\"{}\"'", category);
}
