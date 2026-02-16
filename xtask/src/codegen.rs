//! Code generation from schema
//!
//! Generates:
//! - Stata .ado wrapper files
//! - Stata .sthlp help files
//! - Documentation .md files (docs/src/commands/)

use crate::schema::{Command, Schema};
use anyhow::{bail, Context, Result};
use similar::{ChangeTag, TextDiff};

/// Project root directory
fn project_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Run code generation
pub fn run(check: bool, verbose: bool) -> Result<()> {
    let schema_path = project_root().join("schema/commands.toml");
    let stata_dir = project_root().join("stata");
    let docs_dir = project_root().join("docs/src/commands");

    println!("Loading schema from: {}", schema_path.display());
    let schema = Schema::load(&schema_path)?;

    println!("Generating Stata wrappers...");

    let mut generated_files = Vec::new();
    let mut errors = Vec::new();

    // Generate command wrappers
    for (name, command) in schema.commands_sorted() {
        if verbose {
            println!("  Generating {}_*.ado...", name);
        }

        // Generate .ado file
        let ado_content = generate_ado(name, command, &schema)?;
        let ado_path = stata_dir.join(format!("{}.ado", command.stata_command));

        // Generate .sthlp file
        let sthlp_content = generate_sthlp(name, command, &schema)?;
        let sthlp_path = stata_dir.join(format!("{}.sthlp", command.stata_command));

        generated_files.push((ado_path, ado_content));
        generated_files.push((sthlp_path, sthlp_content));
    }

    // Generate main stacy.ado dispatcher
    let main_ado = generate_main_ado(&schema)?;
    generated_files.push((stata_dir.join("stacy.ado"), main_ado));

    // Generate main stacy.sthlp
    let main_sthlp = generate_main_sthlp(&schema)?;
    generated_files.push((stata_dir.join("stacy.sthlp"), main_sthlp));

    // Generate documentation markdown files
    println!("Generating documentation...");
    for (name, command) in schema.commands_sorted() {
        // Handle cache subcommands: cache_info -> cache.md (skip cache_clean, combined)
        let doc_name = if name == "cache_info" {
            "cache".to_string()
        } else if name == "cache_clean" {
            continue; // Combined with cache_info
        } else {
            name.to_string()
        };

        if verbose {
            println!("  Generating {}.md...", doc_name);
        }

        let md_content = generate_markdown(name, command, &schema)?;
        let md_path = docs_dir.join(format!("{}.md", doc_name));
        generated_files.push((md_path, md_content));
    }

    // Generate reference documentation
    let reference_dir = project_root().join("docs/src/reference");
    if verbose {
        println!("  Generating exit-codes.md...");
    }
    let exit_codes_md = generate_exit_codes_md(&schema)?;
    generated_files.push((reference_dir.join("exit-codes.md"), exit_codes_md));

    // NOTE: Rust output types (output_types.rs) are hand-maintained because:
    // 1. They need additional helper types (ScriptRunResult, ListPackageInfo, etc.)
    // 2. They require to_json() and to_stata() trait methods with custom implementations
    // 3. The types need to match what the CLI commands actually produce
    //
    // The generated version is too simplistic. Only generate Stata wrappers.

    // Write or check files
    let mut changed_count = 0;
    for (path, content) in &generated_files {
        let existing = std::fs::read_to_string(path).unwrap_or_default();

        if existing != *content {
            changed_count += 1;

            if check {
                errors.push(format!(
                    "File out of date: {}\nRun 'cargo xtask codegen' to update",
                    path.display()
                ));

                if verbose {
                    print_diff(&existing, content);
                }
            } else {
                std::fs::write(path, content)
                    .with_context(|| format!("Failed to write {}", path.display()))?;
                println!("  Updated: {}", path.display());
            }
        } else if verbose {
            println!("  Unchanged: {}", path.display());
        }
    }

    if check {
        if errors.is_empty() {
            println!(
                "\nAll {} generated files are up to date.",
                generated_files.len()
            );
            Ok(())
        } else {
            for error in &errors {
                eprintln!("{}", error);
            }
            bail!("{} file(s) need regeneration", errors.len());
        }
    } else {
        println!(
            "\nGenerated {} files ({} changed)",
            generated_files.len(),
            changed_count
        );
        Ok(())
    }
}

/// Verify generated files match schema
pub fn verify(show_diff: bool) -> Result<()> {
    run(true, show_diff)
}

/// Print a diff between two strings
fn print_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        print!("{}{}", sign, change);
    }
}

// =============================================================================
// ADO GENERATION
// =============================================================================

/// Generate a command wrapper .ado file
fn generate_ado(name: &str, command: &Command, _schema: &Schema) -> Result<String> {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "*! {}.ado - {}\n",
        command.stata_command, command.description
    ));
    out.push_str("*! Part of stacy: Reproducible Stata Workflow Tool\n");
    out.push_str("*! Version: 0.1.0\n");
    out.push_str("*! AUTO-GENERATED - DO NOT EDIT\n");
    out.push_str("*! Regenerate with: cargo xtask codegen\n\n");

    // Documentation block
    out.push_str("/*\n");
    out.push_str(&format!("    {}\n\n", command.description));
    out.push_str("    Syntax:\n");
    out.push_str(&format!("        {} ", command.stata_command));

    // Build syntax line
    let positional = command.positional_args();
    let options = command.stata_options();

    for (arg_name, arg) in &positional {
        if arg.required || arg.required_unless.is_none() {
            out.push_str(&format!("<{}> ", arg_name));
        } else {
            out.push_str(&format!("[{}] ", arg_name));
        }
    }

    if !options.is_empty() {
        out.push_str("[, options]");
    }
    out.push_str("\n\n");

    // Options documentation
    if !options.is_empty() {
        out.push_str("    Options:\n");
        for (_, arg) in &options {
            if let Some(ref opt) = arg.stata_option {
                out.push_str(&format!("        {:20} - {}\n", opt, arg.description));
            }
        }
        out.push_str("\n");
    }

    // Returns documentation
    out.push_str("    Returns:\n");
    for (ret_name, ret) in command.returns_sorted() {
        let type_str = if ret.is_scalar() { "scalar" } else { "local" };
        out.push_str(&format!(
            "        r({:20}) - {} ({})\n",
            ret_name, ret.description, type_str
        ));
    }
    out.push_str("*/\n\n");

    // Program definition
    out.push_str(&format!(
        "program define {}, rclass\n",
        command.stata_command
    ));
    out.push_str("    version 14.0\n");

    // Syntax statement
    let syntax = build_stata_syntax(command);
    out.push_str(&format!("    syntax {}\n\n", syntax));

    // Build command string
    out.push_str(&format!("    * Build command arguments\n"));
    out.push_str(&format!("    local cmd \"{}\"\n\n", name));

    // Add positional arguments
    for (arg_name, arg) in command.positional_args() {
        if arg.required {
            out.push_str(&format!("    * Validate required argument: {}\n", arg_name));
            // Check if macro is empty using compound quotes
            out.push_str(&format!("    if `\"`{}'\"' == \"\" {{\n", arg_name));
            out.push_str(&format!(
                "        di as error \"{}: {} is required\"\n",
                command.stata_command, arg_name
            ));
            out.push_str("        exit 198\n");
            out.push_str("    }\n\n");
        }

        // Check if macro is non-empty and add to command
        out.push_str(&format!("    if `\"`{}'\"' != \"\" {{\n", arg_name));
        // Build command with embedded macro: `"`cmd' \"`arg'\""'
        out.push_str(&format!(
            "        local cmd `\"`cmd' \"`{}'\"\"\'\n",
            arg_name
        ));
        out.push_str("    }\n\n");
    }

    // Add options
    for (arg_name, arg) in command.stata_options() {
        if arg.arg_type == "bool" && arg.stata_option.is_some() {
            // Boolean options: check if set (non-empty)
            out.push_str(&format!("    if \"`{}'\" != \"\" {{\n", arg_name));
            out.push_str(&format!("        local cmd `\"`cmd' --{}\"\'\n", arg_name));
            out.push_str("    }\n\n");
        } else if let Some(ref _opt) = arg.stata_option {
            // String options: check if non-empty and add with value
            out.push_str(&format!("    if `\"`{}'\"' != \"\" {{\n", arg_name));
            out.push_str(&format!(
                "        local cmd `\"`cmd' --{} \"`{}'\"\"\'\n",
                arg_name, arg_name
            ));
            out.push_str("    }\n\n");
        }
    }

    // Execute command
    out.push_str("    * Execute via _stacy_exec\n");
    out.push_str("    _stacy_exec `cmd'\n");
    out.push_str("    local exec_rc = r(exit_code)\n\n");

    // Map return values
    out.push_str("    * Map parsed values to r() returns\n");

    // Scalars first
    for (ret_name, ret) in command.returns_sorted() {
        if ret.is_scalar() {
            let internal_name = ret.internal_scalar_name(ret_name);
            out.push_str(&format!("    capture confirm scalar {}\n", internal_name));
            out.push_str("    if _rc == 0 {\n");
            out.push_str(&format!(
                "        return scalar {} = scalar({})\n",
                ret_name, internal_name
            ));
            out.push_str("    }\n\n");
        }
    }

    // Then locals
    for (ret_name, ret) in command.returns_sorted() {
        if ret.is_local() {
            let internal_name = ret.internal_scalar_name(ret_name);
            out.push_str(&format!("    if `\"`{}'\"' != \"\" {{\n", internal_name));
            out.push_str(&format!(
                "        return local {} `\"`{}'\"'\n",
                ret_name, internal_name
            ));
            out.push_str("    }\n\n");
        }
    }

    // Return exit code if non-zero
    out.push_str("    * Return failure if command failed\n");
    out.push_str("    if `exec_rc' != 0 {\n");
    out.push_str("        exit `exec_rc'\n");
    out.push_str("    }\n");

    out.push_str("end\n");

    Ok(out)
}

/// Build Stata syntax statement
fn build_stata_syntax(command: &Command) -> String {
    let mut parts = Vec::new();

    // Positional arguments
    for (name, arg) in command.positional_args() {
        if arg.required || arg.required_unless.is_none() {
            parts.push(format!("anything(name={})", name));
        } else {
            parts.push(format!("[anything(name={})]", name));
        }
    }

    // Options
    let options: Vec<String> = command
        .stata_options()
        .iter()
        .filter_map(|(_, arg)| arg.stata_option.clone())
        .collect();

    if !options.is_empty() {
        parts.push(format!("[, {}]", options.join(" ")));
    }

    parts.join(" ")
}

// =============================================================================
// STHLP GENERATION
// =============================================================================

/// Generate a command help .sthlp file
fn generate_sthlp(name: &str, command: &Command, _schema: &Schema) -> Result<String> {
    let mut out = String::new();

    // Header
    out.push_str("{smcl}\n");
    out.push_str("{* *! version 0.1.0 - AUTO-GENERATED}{...}\n");
    out.push_str(&format!(
        "{{viewerjumpto \"Syntax\" \"{}##syntax\"}}{{...}}\n",
        command.stata_command
    ));
    out.push_str(&format!(
        "{{viewerjumpto \"Description\" \"{}##description\"}}{{...}}\n",
        command.stata_command
    ));
    out.push_str(&format!(
        "{{viewerjumpto \"Options\" \"{}##options\"}}{{...}}\n",
        command.stata_command
    ));
    out.push_str(&format!(
        "{{viewerjumpto \"Returns\" \"{}##returns\"}}{{...}}\n",
        command.stata_command
    ));
    out.push_str(&format!(
        "{{viewerjumpto \"Examples\" \"{}##examples\"}}{{...}}\n",
        command.stata_command
    ));

    // Title
    out.push_str("{title:Title}\n\n");
    out.push_str("{phang}\n");
    out.push_str(&format!(
        "{{bf:stacy {}}} {{hline 2}} {}\n\n\n",
        name, command.description
    ));

    // Syntax
    out.push_str(&format!("{{marker syntax}}{{...}}\n"));
    out.push_str("{title:Syntax}\n\n");
    out.push_str("{p 8 17 2}\n");
    out.push_str(&format!("{{cmd:stacy {}}} ", name));

    // Positional args
    for (arg_name, arg) in command.positional_args() {
        if arg.required || arg.required_unless.is_none() {
            out.push_str(&format!("{{it:{}}} ", arg_name));
        } else {
            out.push_str(&format!("[{{it:{}}}] ", arg_name));
        }
    }

    // Options
    let options = command.stata_options();
    if !options.is_empty() {
        out.push_str("[{cmd:,} {it:options}]");
    }
    out.push_str("\n\n");

    // Options table
    if !options.is_empty() {
        out.push_str("{synoptset 20 tabbed}{...}\n");
        out.push_str("{synopthdr}\n");
        out.push_str("{synoptline}\n");
        out.push_str("{syntab:Main}\n");

        for (_, arg) in &options {
            if let Some(ref opt) = arg.stata_option {
                let opt_display = opt.to_lowercase();
                out.push_str(&format!(
                    "{{synopt:{{opt:{}}}}}{}{{p_end}}\n",
                    opt_display, arg.description
                ));
            }
        }

        out.push_str("{synoptline}\n\n\n");
    }

    // Description
    out.push_str(&format!("{{marker description}}{{...}}\n"));
    out.push_str("{title:Description}\n\n");
    out.push_str("{pstd}\n");
    out.push_str(&format!(
        "{{cmd:stacy {}}} {}.\n\n\n",
        name,
        command.description.to_lowercase()
    ));

    // Options section
    if !options.is_empty() {
        out.push_str(&format!("{{marker options}}{{...}}\n"));
        out.push_str("{title:Options}\n\n");

        for (arg_name, arg) in &options {
            out.push_str("{phang}\n");
            out.push_str(&format!(
                "{{opt {}}} {}.\n\n",
                arg_name,
                arg.description.to_lowercase()
            ));
        }
        out.push_str("\n");
    }

    // Returns
    out.push_str(&format!("{{marker returns}}{{...}}\n"));
    out.push_str("{title:Stored results}\n\n");
    out.push_str("{pstd}\n");
    out.push_str(&format!(
        "{{cmd:stacy {}}} stores the following in {{cmd:r()}}:\n\n",
        name
    ));

    out.push_str("{synoptset 25 tabbed}{...}\n");

    // Scalars
    let scalars: Vec<_> = command
        .returns_sorted()
        .into_iter()
        .filter(|(_, r)| r.is_scalar())
        .collect();

    if !scalars.is_empty() {
        out.push_str("{p2col 5 25 29 2: Scalars}{p_end}\n");
        for (ret_name, ret) in scalars {
            out.push_str(&format!(
                "{{synopt:{{cmd:r({})}}}}{}{{p_end}}\n",
                ret_name, ret.description
            ));
        }
        out.push_str("\n");
    }

    // Locals
    let locals: Vec<_> = command
        .returns_sorted()
        .into_iter()
        .filter(|(_, r)| r.is_local())
        .collect();

    if !locals.is_empty() {
        out.push_str("{p2col 5 25 29 2: Macros}{p_end}\n");
        for (ret_name, ret) in locals {
            out.push_str(&format!(
                "{{synopt:{{cmd:r({})}}}}{}{{p_end}}\n",
                ret_name, ret.description
            ));
        }
        out.push_str("\n");
    }

    // Examples
    out.push_str(&format!("\n{{marker examples}}{{...}}\n"));
    out.push_str("{title:Examples}\n\n");
    out.push_str("{pstd}Basic usage:{p_end}\n");
    out.push_str(&format!("{{phang2}}{{cmd:. stacy {}}}{{p_end}}\n\n", name));

    // Author
    out.push_str("\n{marker author}{...}\n");
    out.push_str("{title:Author}\n\n");
    out.push_str("{pstd}\n");
    out.push_str("Jan Fasnacht{p_end}\n");
    out.push_str("{pstd}\n");
    out.push_str(
        "{browse \"https://github.com/janfasnacht/stacy\":github.com/janfasnacht/stacy}{p_end}\n\n\n",
    );

    // Also see
    out.push_str("{marker also_see}{...}\n");
    out.push_str("{title:Also see}\n\n");
    out.push_str("{psee}\n");
    out.push_str("{space 2}Help:  {helpb stacy}\n");
    out.push_str("{p_end}\n");

    Ok(out)
}

// =============================================================================
// MAIN DISPATCHER GENERATION
// =============================================================================

/// Generate the main stacy.ado dispatcher
fn generate_main_ado(schema: &Schema) -> Result<String> {
    let mut out = String::new();

    out.push_str("*! stacy.ado - Reproducible Stata Workflow Tool\n");
    out.push_str("*! Version: 0.1.0\n");
    out.push_str("*! Author: Jan Fasnacht\n");
    out.push_str("*! URL: https://github.com/janfasnacht/stacy\n");
    out.push_str("*! AUTO-GENERATED - DO NOT EDIT\n");
    out.push_str("*! Regenerate with: cargo xtask codegen\n\n");

    out.push_str("/*\n");
    out.push_str("    stacy - Main entry point for stacy Stata wrapper\n\n");
    out.push_str("    This command dispatches to subcommand-specific programs.\n");
    out.push_str("    \n");
    out.push_str("    Usage:\n");
    out.push_str("        stacy <subcommand> [options]\n");
    out.push_str("    \n");
    out.push_str("    For help:\n");
    out.push_str("        help stacy\n");
    out.push_str("*/\n\n");

    out.push_str("program define stacy, rclass\n");
    out.push_str("    version 14.0\n\n");
    out.push_str("    * Parse first argument as subcommand\n");
    out.push_str("    gettoken subcmd 0 : 0, parse(\" ,\")\n\n");

    out.push_str("    if \"`subcmd'\" == \"\" {\n");
    out.push_str("        di as error \"stacy: subcommand required\"\n");
    out.push_str("        di as text \"\"\n");
    out.push_str("        di as text \"Available commands:\"\n");

    for (name, command) in schema.commands_sorted() {
        out.push_str(&format!(
            "        di as text \"  stacy {:12} - {}\"\n",
            name, command.description
        ));
    }

    out.push_str("        di as text \"\"\n");
    out.push_str("        di as text \"For more help: help stacy\"\n");
    out.push_str("        exit 198\n");
    out.push_str("    }\n\n");

    out.push_str("    * Dispatch to appropriate subcommand\n");
    let mut first = true;
    for (name, command) in schema.commands_sorted() {
        let keyword = if first { "if" } else { "else if" };
        first = false;
        out.push_str(&format!(
            "    {} \"`subcmd'\" == \"{}\" {{\n",
            keyword, name
        ));
        out.push_str(&format!("        {} `0'\n", command.stata_command));
        out.push_str("    }\n");
    }

    // Special cases
    out.push_str("    else if \"`subcmd'\" == \"setup\" {\n");
    out.push_str("        stacy_setup `0'\n");
    out.push_str("    }\n");
    out.push_str("    else if \"`subcmd'\" == \"version\" | \"`subcmd'\" == \"--version\" {\n");
    out.push_str("        di as text \"stacy Stata wrapper v0.1.0\"\n");
    out.push_str("    }\n");
    out.push_str("    else if \"`subcmd'\" == \"help\" | \"`subcmd'\" == \"--help\" {\n");
    out.push_str("        help stacy\n");
    out.push_str("    }\n");
    out.push_str("    else {\n");
    out.push_str("        di as error \"stacy: unknown subcommand '`subcmd'\"\n");
    out.push_str("        di as text \"\"\n");
    out.push_str(
        "        di as text \"Run 'stacy' without arguments to see available commands.\"\n",
    );
    out.push_str("        exit 198\n");
    out.push_str("    }\n\n");

    out.push_str("    * Forward return values from subcommand\n");
    out.push_str("    return add\n");
    out.push_str("end\n");

    Ok(out)
}

/// Generate the main stacy.sthlp help file
fn generate_main_sthlp(schema: &Schema) -> Result<String> {
    let mut out = String::new();

    out.push_str("{smcl}\n");
    out.push_str("{* *! version 0.1.0 - AUTO-GENERATED}{...}\n");
    out.push_str("{viewerjumpto \"Syntax\" \"stacy##syntax\"}{...}\n");
    out.push_str("{viewerjumpto \"Description\" \"stacy##description\"}{...}\n");
    out.push_str("{viewerjumpto \"Commands\" \"stacy##commands\"}{...}\n");
    out.push_str("{viewerjumpto \"Examples\" \"stacy##examples\"}{...}\n");
    out.push_str("{viewerjumpto \"Installation\" \"stacy##installation\"}{...}\n");
    out.push_str("{viewerjumpto \"Author\" \"stacy##author\"}{...}\n");

    out.push_str("{title:Title}\n\n");
    out.push_str("{phang}\n");
    out.push_str(&format!(
        "{{bf:stacy}} {{hline 2}} {}\n\n\n",
        schema.meta.description
    ));

    out.push_str("{marker syntax}{...}\n");
    out.push_str("{title:Syntax}\n\n");
    out.push_str("{p 8 17 2}\n");
    out.push_str("{cmd:stacy} {it:subcommand} [{it:arguments}] [{cmd:,} {it:options}]\n\n\n");

    out.push_str("{marker description}{...}\n");
    out.push_str("{title:Description}\n\n");
    out.push_str("{pstd}\n");
    out.push_str(
        "{cmd:stacy} is a workflow tool for reproducible Stata projects that provides:\n\n",
    );
    out.push_str("{p 8 12 2}\n");
    out.push_str(
        "{bf:1.} Proper error detection and exit codes for build system integration{p_end}\n",
    );
    out.push_str("{p 8 12 2}\n");
    out.push_str("{bf:2.} Dependency analysis for Stata scripts{p_end}\n");
    out.push_str("{p 8 12 2}\n");
    out.push_str("{bf:3.} Package management with lockfile support{p_end}\n");
    out.push_str("{p 8 12 2}\n");
    out.push_str("{bf:4.} Project initialization and configuration{p_end}\n\n\n");

    out.push_str("{marker commands}{...}\n");
    out.push_str("{title:Commands}\n\n");
    out.push_str("{synoptset 25 tabbed}{...}\n");
    out.push_str("{synopthdr:subcommand}\n");
    out.push_str("{synoptline}\n");

    for (name, command) in schema.commands_sorted() {
        out.push_str(&format!(
            "{{synopt:{{helpb {}:stacy {}}}}}{}{{p_end}}\n",
            command.stata_command, name, command.description
        ));
    }
    out.push_str(
        "{synopt:{helpb stacy_setup:stacy setup}}Download and install the stacy binary{p_end}\n",
    );
    out.push_str("{synoptline}\n\n\n");

    out.push_str("{marker examples}{...}\n");
    out.push_str("{title:Examples}\n\n");
    out.push_str("{pstd}Setup stacy (first time only):{p_end}\n");
    out.push_str("{phang2}{cmd:. stacy setup}{p_end}\n\n");
    out.push_str("{pstd}Run system diagnostics:{p_end}\n");
    out.push_str("{phang2}{cmd:. stacy doctor}{p_end}\n\n");
    out.push_str("{pstd}Execute a Stata script:{p_end}\n");
    out.push_str("{phang2}{cmd:. stacy run \"analysis/main.do\"}{p_end}\n\n\n");

    out.push_str("{marker installation}{...}\n");
    out.push_str("{title:Installation}\n\n");
    out.push_str("{pstd}\n");
    out.push_str("To install the Stata wrapper, run:{p_end}\n\n");
    out.push_str("{phang2}{cmd:. net install stacy, from(\"https://raw.githubusercontent.com/janfasnacht/stacy/main/stata/\")}{p_end}\n\n");
    out.push_str("{pstd}\n");
    out.push_str("Then download the stacy binary:{p_end}\n\n");
    out.push_str("{phang2}{cmd:. stacy_setup}{p_end}\n\n\n");

    out.push_str("{marker author}{...}\n");
    out.push_str("{title:Author}\n\n");
    out.push_str("{pstd}\n");
    out.push_str("Jan Fasnacht{p_end}\n");
    out.push_str("{pstd}\n");
    out.push_str(
        "{browse \"https://github.com/janfasnacht/stacy\":github.com/janfasnacht/stacy}{p_end}\n\n\n",
    );

    out.push_str("{marker also_see}{...}\n");
    out.push_str("{title:Also see}\n\n");
    out.push_str("{pstd}\n");
    out.push_str("Help:  ");

    let help_links: Vec<String> = schema
        .commands_sorted()
        .iter()
        .map(|(_, cmd)| format!("{{helpb {}}}", cmd.stata_command))
        .collect();

    // Break into chunks of 6 for better line wrapping
    for (i, chunk) in help_links.chunks(6).enumerate() {
        if i > 0 {
            out.push_str(",\n{space 7}");
        }
        out.push_str(&chunk.join(", "));
    }
    out.push_str(",\n{space 7}{helpb stacy_setup}\n");
    out.push_str("{p_end}\n");

    Ok(out)
}

// =============================================================================
// MARKDOWN DOCUMENTATION GENERATION
// =============================================================================

/// Generate a command documentation .md file
fn generate_markdown(name: &str, command: &Command, schema: &Schema) -> Result<String> {
    let mut out = String::new();

    // Format command name (cache_info -> cache info)
    let display_name = name.replace('_', " ");

    // Header
    out.push_str(&format!("# stacy {}\n\n", display_name));
    out.push_str(&format!("{}\n\n", command.description));

    // Synopsis
    out.push_str("## Synopsis\n\n");
    out.push_str("```\n");
    out.push_str(&format!("stacy {} ", display_name));

    let positional = command.positional_args();
    let options = command.stata_options();

    for (arg_name, arg) in &positional {
        if arg.required || arg.required_unless.is_none() {
            out.push_str(&format!("<{}> ", arg_name.to_uppercase()));
        } else {
            out.push_str(&format!("[{}] ", arg_name.to_uppercase()));
        }
    }
    if !options.is_empty() {
        out.push_str("[OPTIONS]");
    }
    out.push_str("\n```\n\n");

    // Description
    out.push_str("## Description\n\n");
    if let Some(ref long_desc) = command.long_description {
        out.push_str(long_desc.trim());
        out.push_str("\n\n");
    } else {
        out.push_str(&format!(
            "`stacy {}` {}.\n\n",
            display_name,
            command.description.to_lowercase()
        ));
    }

    // Arguments table (if any positional args)
    if !positional.is_empty() {
        out.push_str("## Arguments\n\n");
        out.push_str("| Argument | Description |\n");
        out.push_str("|----------|-------------|\n");
        for (arg_name, arg) in &positional {
            let required = if arg.required { " (required)" } else { "" };
            out.push_str(&format!(
                "| `<{}>` | {}{} |\n",
                arg_name.to_uppercase(),
                arg.description,
                required
            ));
        }
        out.push_str("\n");
    }

    // Options table
    if !options.is_empty() {
        out.push_str("## Options\n\n");
        out.push_str("| Option | Description |\n");
        out.push_str("|--------|-------------|\n");
        for (arg_name, arg) in &options {
            let flag = if let Some(ref short) = arg.short {
                format!("-{}, --{}", short, arg_name)
            } else {
                format!("--{}", arg_name)
            };
            out.push_str(&format!("| `{}` | {} |\n", flag, arg.description));
        }
        out.push_str("\n");
    }

    // Examples
    if !command.examples.is_empty() {
        out.push_str("## Examples\n\n");
        for example in &command.examples {
            out.push_str(&format!("### {}\n\n", example.title));
            if let Some(ref desc) = example.description {
                out.push_str(&format!("{}\n\n", desc));
            }
            out.push_str("```bash\n");
            for cmd in &example.commands {
                out.push_str(&format!("{}\n", cmd));
            }
            out.push_str("```\n\n");
            if let Some(ref output) = example.output {
                out.push_str("```\n");
                out.push_str(output.trim());
                out.push_str("\n```\n\n");
            }
        }
    }

    // Exit codes - link to reference instead of repeating
    if !command.exit_codes.is_empty() {
        out.push_str("## Exit Codes\n\n");
        out.push_str("| Code | Meaning |\n");
        out.push_str("|------|--------|\n");
        let mut codes: Vec<_> = command.exit_codes.iter().collect();
        codes.sort_by_key(|(k, _)| k.parse::<i32>().unwrap_or(999));
        for (code, meaning) in codes {
            out.push_str(&format!("| {} | {} |\n", code, meaning));
        }
        out.push_str("\nSee [Exit Codes Reference](../reference/exit-codes.md) for details.\n\n");
    }

    // See Also
    out.push_str("## See Also\n\n");
    if let Some(ref see_also) = command.see_also {
        for item in see_also {
            let link = format_see_also_link(item, schema);
            out.push_str(&format!("- {}\n", link));
        }
    } else {
        // Default: link to related commands in same category
        let mut seen_docs = std::collections::HashSet::new();
        let related: Vec<_> = schema
            .commands_sorted()
            .into_iter()
            .filter(|(n, c)| *n != name && c.category == command.category)
            .filter_map(|(rel_name, _)| {
                // Map cache_* to cache
                let doc_name = if rel_name.starts_with("cache_") {
                    "cache"
                } else {
                    rel_name.as_str()
                };
                // Skip if we've already linked to this doc
                if seen_docs.insert(doc_name.to_string()) {
                    Some((rel_name.to_string(), doc_name.to_string()))
                } else {
                    None
                }
            })
            .take(3)
            .collect();
        for (rel_name, doc_name) in related {
            let display_name = if rel_name.starts_with("cache_") {
                "cache".to_string()
            } else {
                rel_name
            };
            out.push_str(&format!("- [stacy {}](./{}.md)\n", display_name, doc_name));
        }
    }
    out.push_str("\n");

    Ok(out)
}

/// Format a see_also item as a markdown link
fn format_see_also_link(item: &str, schema: &Schema) -> String {
    if item.starts_with("../") || item.starts_with("./") {
        // Already a relative path — extract filename for title
        let filename = item
            .rsplit('/')
            .next()
            .unwrap_or(item)
            .trim_end_matches(".md");
        // If it looks like a sibling command doc (./foo.md), prefix with "stacy "
        let title = if item.starts_with("./") {
            format!("stacy {}", filename)
        } else {
            title_case(filename)
        };
        format!("[{}]({})", title, item)
    } else if schema.commands.contains_key(item) {
        // Command name
        format!("[stacy {}](./{}.md)", item, item)
    } else {
        // Assume it's a relative doc path
        format!("[{}]({})", item, item)
    }
}

/// Convert kebab-case to Title Case
fn title_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// =============================================================================
// REFERENCE DOCUMENTATION GENERATION
// =============================================================================

/// Generate exit-codes.md reference documentation
fn generate_exit_codes_md(schema: &Schema) -> Result<String> {
    let mut out = String::new();

    out.push_str("# Exit Codes\n\n");
    out.push_str("stacy uses consistent exit codes to indicate success or failure type.\n\n");

    // Main table
    out.push_str("## Exit Code Table\n\n");
    out.push_str("| Code | Name | Description |\n");
    out.push_str("|------|------|-------------|\n");

    let mut codes: Vec<_> = schema.exit_codes.iter().collect();
    codes.sort_by_key(|(k, _)| k.parse::<i32>().unwrap_or(999));

    for (code, def) in &codes {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            code, def.name, def.description
        ));
    }
    out.push_str("\n");

    // Stata r() code mapping
    out.push_str("## Stata r() Code Mapping\n\n");
    out.push_str("stacy maps Stata's r() error codes to exit codes:\n\n");
    out.push_str("| Exit Code | Stata r() Codes |\n");
    out.push_str("|-----------|----------------|\n");

    for (code, def) in &codes {
        if let Some(ref r_codes) = def.r_codes {
            out.push_str(&format!("| {} | {} |\n", code, r_codes));
        }
    }
    out.push_str("\n");

    // Usage examples
    out.push_str("## Usage\n\n");
    out.push_str("### Shell\n\n");
    out.push_str("```bash\n");
    out.push_str("stacy run analysis.do\n");
    out.push_str("echo $?  # 0 on success, 1-10 on failure\n");
    out.push_str("```\n\n");

    out.push_str("### Makefile\n\n");
    out.push_str("```makefile\n");
    out.push_str("results.dta: analysis.do\n");
    out.push_str("\tstacy run analysis.do  # Stops on non-zero exit\n");
    out.push_str("```\n\n");

    // Stability note
    out.push_str("## Stability\n\n");
    out.push_str("Exit codes 0-10 are stable and will not change meaning. ");
    out.push_str("New categories may be added with codes 11+.\n\n");

    out.push_str("## See Also\n\n");
    out.push_str("- [Error Detection](./errors.md)\n");
    out.push_str("- [stacy run](../commands/run.md)\n");

    Ok(out)
}
