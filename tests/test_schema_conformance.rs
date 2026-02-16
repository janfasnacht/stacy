//! Schema conformance tests
//!
//! These tests verify that the CLI JSON output matches the schema structure
//! and that generated files are correct.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Load and parse the schema
fn load_schema() -> toml::Value {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/commands.toml");
    let content = fs::read_to_string(&schema_path).expect("Failed to read schema");
    toml::from_str(&content).expect("Failed to parse schema")
}

#[test]
fn test_schema_has_all_commands() {
    let schema = load_schema();
    let commands = schema.get("commands").expect("Missing commands section");
    let commands_table = commands.as_table().expect("Commands should be a table");

    // Dynamically verify all commands in schema have required structure
    // rather than hardcoding a subset
    let expected_commands = [
        "run",
        "doctor",
        "env",
        "explain",
        "install",
        "deps",
        "init",
        "add",
        "remove",
        "update",
        "list",
        "outdated",
        "lock",
        "bench",
        "task",
        "test",
        "cache_info",
        "cache_clean",
    ];

    // Ensure we know about all schema commands (catches additions)
    for key in commands_table.keys() {
        assert!(
            expected_commands.contains(&key.as_str()),
            "Schema has unexpected command '{}' — add it to expected_commands",
            key
        );
    }

    // Ensure all expected commands exist in schema
    for cmd in expected_commands {
        assert!(
            commands.get(cmd).is_some(),
            "Schema missing command: {}",
            cmd
        );
    }
}

#[test]
fn test_schema_commands_have_required_fields() {
    let schema = load_schema();
    let commands = schema.get("commands").expect("Missing commands section");
    let commands = commands.as_table().expect("Commands should be a table");

    for (name, command) in commands {
        let cmd = command.as_table().expect("Command should be a table");

        assert!(
            cmd.contains_key("description"),
            "Command {} missing description",
            name
        );
        assert!(
            cmd.contains_key("category"),
            "Command {} missing category",
            name
        );
        assert!(
            cmd.contains_key("stata_command"),
            "Command {} missing stata_command",
            name
        );
        assert!(
            cmd.contains_key("returns"),
            "Command {} missing returns",
            name
        );
    }
}

#[test]
fn test_schema_return_types_are_valid() {
    let schema = load_schema();
    let commands = schema.get("commands").expect("Missing commands section");
    let commands = commands.as_table().expect("Commands should be a table");

    let valid_types = ["bool", "int", "float", "string", "path"];
    let valid_stata_types = ["scalar", "local"];

    for (cmd_name, command) in commands {
        let empty_map = toml::map::Map::new();
        let returns = command
            .get("returns")
            .and_then(|r| r.as_table())
            .unwrap_or(&empty_map);

        for (ret_name, ret_val) in returns {
            let ret = ret_val.as_table().expect("Return should be a table");

            // Check type
            if let Some(ret_type) = ret.get("type").and_then(|t| t.as_str()) {
                assert!(
                    valid_types.contains(&ret_type),
                    "Command {} return {} has invalid type: {}",
                    cmd_name,
                    ret_name,
                    ret_type
                );
            }

            // Check stata_type
            if let Some(stata_type) = ret.get("stata_type").and_then(|t| t.as_str()) {
                assert!(
                    valid_stata_types.contains(&stata_type),
                    "Command {} return {} has invalid stata_type: {}",
                    cmd_name,
                    ret_name,
                    stata_type
                );
            }
        }
    }
}

#[test]
fn test_generated_stata_files_exist() {
    let stata_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("stata");

    // Core utilities (JSON parsing helpers moved to legacy/)
    let expected_files = [
        // Core utilities
        "_stacy_exec.ado",
        "_stacy_find_binary.ado",
        // Main dispatcher
        "stacy.ado",
        "stacy.sthlp",
        // Command wrappers
        "stacy_run.ado",
        "stacy_run.sthlp",
        "stacy_doctor.ado",
        "stacy_doctor.sthlp",
        "stacy_env.ado",
        "stacy_env.sthlp",
        "stacy_install.ado",
        "stacy_install.sthlp",
        "stacy_deps.ado",
        "stacy_deps.sthlp",
        "stacy_init.ado",
        "stacy_init.sthlp",
        "stacy_explain.ado",
        "stacy_explain.sthlp",
        // Installer
        "stacy_setup.ado",
        "stacy_setup.sthlp",
        // Package metadata
        "stata.toc",
        "stacy.pkg",
    ];

    for file in expected_files {
        let path = stata_dir.join(file);
        assert!(path.exists(), "Missing Stata file: {}", file);
    }
}

#[test]
fn test_ado_files_have_proper_headers() {
    let stata_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("stata");

    // All .ado files should have proper Stata headers
    let ado_files = [
        "stacy.ado",
        "stacy_run.ado",
        "stacy_doctor.ado",
        "stacy_env.ado",
        "stacy_install.ado",
        "stacy_deps.ado",
        "stacy_init.ado",
        "stacy_explain.ado",
        "_stacy_exec.ado",
        "_stacy_find_binary.ado",
    ];

    for file in ado_files {
        let path = stata_dir.join(file);
        let content = fs::read_to_string(&path).expect("Failed to read file");

        // Should have a Stata-style header comment
        assert!(
            content.starts_with("*!"),
            "File {} should start with *! header",
            file
        );

        // Should have version requirement
        assert!(
            content.contains("version 14.0"),
            "File {} should require version 14.0",
            file
        );

        // Should have program definition
        assert!(
            content.contains("program define"),
            "File {} should have program definition",
            file
        );
    }
}

#[test]
fn test_schema_returns_match_output_types() {
    let schema = load_schema();
    let commands = schema.get("commands").expect("Missing commands section");
    let commands = commands.as_table().expect("Commands should be a table");

    // Read the hand-maintained output_types.rs
    // NOTE: output_types.rs is hand-maintained, not generated from schema.
    // It uses structured types (Vec<T>) instead of comma-separated strings.
    let output_types_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cli/output_types.rs");
    let output_types = fs::read_to_string(&output_types_path).expect("Failed to read output_types");

    // Commands with complex array handling that use structured types in Rust
    // but comma-separated strings in the Stata schema
    let skip_field_check: HashSet<_> = [
        "list",     // Uses packages: Vec<ListPackageInfo>
        "outdated", // Uses packages: Vec<OutdatedPackageInfo>
        "task",     // Uses scripts: Vec<ScriptResultOutput>
        "test",     // Uses tests: Vec<TestResultOutput>
    ]
    .iter()
    .cloned()
    .collect();

    for (cmd_name, command) in commands {
        // Check struct exists
        let struct_name = format!(
            "{}Output",
            cmd_name
                .split('_')
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().chain(c).collect(),
                    }
                })
                .collect::<String>()
        );

        assert!(
            output_types.contains(&format!("pub struct {}", struct_name)),
            "Missing output struct for command {}: expected {}",
            cmd_name,
            struct_name
        );

        // Skip field checking for commands with complex array types
        if skip_field_check.contains(cmd_name.as_str()) {
            continue;
        }

        // Check fields exist
        let empty_map = toml::map::Map::new();
        let returns = command
            .get("returns")
            .and_then(|r| r.as_table())
            .unwrap_or(&empty_map);

        for (ret_name, ret_def) in returns {
            // Skip fields with array_handling - they use structured types in Rust
            if let Some(ret_table) = ret_def.as_table() {
                if ret_table.contains_key("array_handling") {
                    continue;
                }
            }

            // Field should exist in struct (either as-is or snake_cased)
            let field_name = ret_name.replace('-', "_").to_lowercase();
            assert!(
                output_types.contains(&format!("pub {}", field_name)),
                "Missing field {} in {} struct",
                field_name,
                struct_name
            );
        }
    }
}

#[test]
fn test_run_command_schema_matches_cli_output() {
    let schema = load_schema();
    let run_cmd = schema
        .get("commands")
        .and_then(|c| c.get("run"))
        .expect("Missing run command");
    let returns = run_cmd
        .get("returns")
        .and_then(|r| r.as_table())
        .expect("Missing run returns");

    // Expected fields from schema
    let expected_fields: HashSet<_> = returns.keys().cloned().collect();

    // These fields should be in the JSON output (except array counts)
    let json_fields: HashSet<_> = vec![
        "success".to_string(),
        "exit_code".to_string(),
        "duration_secs".to_string(),
        "source".to_string(),
        "script".to_string(),
        "log_file".to_string(),
        "error_count".to_string(), // This is derived from errors array
    ]
    .into_iter()
    .collect();

    // Check all schema fields are accounted for
    for field in &expected_fields {
        assert!(
            json_fields.contains(field),
            "Schema field {} not in expected JSON output",
            field
        );
    }
}

#[test]
fn test_stata_wrapper_returns_match_schema() {
    let schema = load_schema();
    let stata_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("stata");
    let commands = schema.get("commands").expect("Missing commands section");
    let commands = commands.as_table().expect("Commands should be a table");

    for (cmd_name, command) in commands {
        let stata_command = command
            .get("stata_command")
            .and_then(|s| s.as_str())
            .expect("Missing stata_command");

        let ado_path = stata_dir.join(format!("{}.ado", stata_command));
        let content = fs::read_to_string(&ado_path).expect("Failed to read ado file");

        let empty_map = toml::map::Map::new();
        let returns = command
            .get("returns")
            .and_then(|r| r.as_table())
            .unwrap_or(&empty_map);

        for (ret_name, ret_val) in returns {
            let ret = ret_val.as_table().expect("Return should be a table");
            let stata_type = ret.get("stata_type").and_then(|t| t.as_str()).unwrap_or("");

            // Check that the return statement exists
            let expected_pattern = if stata_type == "scalar" {
                format!("return scalar {}", ret_name)
            } else {
                format!("return local {}", ret_name)
            };

            assert!(
                content.contains(&expected_pattern),
                "Command {} missing return for {}: expected '{}'",
                cmd_name,
                ret_name,
                expected_pattern
            );
        }
    }
}

#[test]
fn test_help_files_have_correct_sections() {
    let stata_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("stata");

    // Dynamically find all .sthlp files instead of hardcoding a subset
    let mut sthlp_files: Vec<String> = fs::read_dir(&stata_dir)
        .expect("Failed to read stata directory")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".sthlp") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    sthlp_files.sort();

    assert!(
        !sthlp_files.is_empty(),
        "No .sthlp files found in stata directory"
    );

    let required_sections = [
        "{title:Title}",
        "{title:Syntax}",
        "{title:Description}",
        "{title:Author}",
    ];

    for file in &sthlp_files {
        let path = stata_dir.join(file);
        let content = fs::read_to_string(&path).expect("Failed to read help file");

        assert!(
            content.starts_with("{smcl}"),
            "Help file {} should start with {{smcl}}",
            file
        );

        for section in required_sections {
            assert!(
                content.contains(section),
                "Help file {} missing section: {}",
                file,
                section
            );
        }
    }
}

#[test]
fn test_main_dispatcher_has_all_commands() {
    let schema = load_schema();
    let stata_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("stata");
    let dispatcher =
        fs::read_to_string(stata_dir.join("stacy.ado")).expect("Failed to read stacy.ado");

    let commands = schema.get("commands").expect("Missing commands section");
    let commands = commands.as_table().expect("Commands should be a table");

    for cmd_name in commands.keys() {
        // Check dispatch condition exists
        assert!(
            dispatcher.contains(&format!("if \"`subcmd'\" == \"{}\"", cmd_name)),
            "Dispatcher missing command: {}",
            cmd_name
        );
    }

    // Check special commands
    assert!(
        dispatcher.contains("stacy_setup"),
        "Dispatcher missing setup command"
    );
    assert!(
        dispatcher.contains("version"),
        "Dispatcher missing version command"
    );
    assert!(
        dispatcher.contains("help stacy"),
        "Dispatcher missing help command"
    );
}

#[test]
fn test_dispatcher_forwards_return_values() {
    let stata_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("stata");
    let dispatcher =
        fs::read_to_string(stata_dir.join("stacy.ado")).expect("Failed to read stacy.ado");

    // Dispatcher must use 'return add' to forward r() values from subcommands
    assert!(
        dispatcher.contains("return add"),
        "Dispatcher must use 'return add' to forward r() values"
    );
}

#[test]
fn test_dispatcher_handles_unknown_commands() {
    let stata_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("stata");
    let dispatcher =
        fs::read_to_string(stata_dir.join("stacy.ado")).expect("Failed to read stacy.ado");

    // Dispatcher must handle unknown commands with exit 198
    assert!(
        dispatcher.contains("exit 198"),
        "Dispatcher must exit 198 for unknown commands"
    );
    assert!(
        dispatcher.contains("unknown subcommand"),
        "Dispatcher must show error message for unknown commands"
    );
}

// =============================================================================
// Generated .ado file Stata syntax validation
// =============================================================================

/// Read all generated .ado files from stata/ directory
fn read_all_generated_ado_files() -> Vec<(String, String)> {
    let stata_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("stata");
    let mut results = Vec::new();

    for entry in fs::read_dir(&stata_dir).expect("Failed to read stata directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "ado") {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let content = fs::read_to_string(&path).expect("Failed to read ado file");
            // Only check auto-generated files
            if content.contains("AUTO-GENERATED") {
                results.push((name, content));
            }
        }
    }

    assert!(
        !results.is_empty(),
        "No auto-generated .ado files found in stata/"
    );
    results
}

#[test]
fn test_generated_ado_syntax_no_bare_integer() {
    // Bug class 3: `syntax` with `(integer)` or `(real)` without a default value
    // causes Stata rc=197. Must be `(integer 0)` or `(string)`.
    let re = regex::Regex::new(r"\(integer\)|\(real\)").unwrap();

    for (name, content) in read_all_generated_ado_files() {
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("syntax ") || trimmed.starts_with("syntax\t") {
                assert!(
                    !re.is_match(trimmed),
                    "{}:{}: syntax line has bare (integer) or (real) without default: {}",
                    name,
                    line_num + 1,
                    trimmed
                );
            }
        }
    }
}

#[test]
fn test_generated_ado_globals_use_stacy_prefix() {
    // Bug class 1: `_stacy_` prefix instead of `stacy_` for globals.
    // Stata forbids globals starting with underscore.
    for (name, content) in read_all_generated_ado_files() {
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Check global references use stacy_ not _stacy_
            if trimmed.contains("_stacy_") {
                // Allow references to programs named _stacy_* (e.g., _stacy_exec)
                // and comments. Only flag global/scalar variable references.
                let is_program_call =
                    trimmed.starts_with("_stacy_") || trimmed.contains("program define _stacy_");
                let is_comment = trimmed.starts_with("*") || trimmed.starts_with("//");

                if !is_program_call && !is_comment {
                    // Check for global macro references: ${_stacy_...} or $(_stacy_...)
                    assert!(
                        !trimmed.contains("${_stacy_") && !trimmed.contains("$(_stacy_"),
                        "{}:{}: uses _stacy_ prefix for global macro (Stata forbids _ prefix): {}",
                        name,
                        line_num + 1,
                        trimmed
                    );
                    // Check for scalar references: scalar(_stacy_...)
                    assert!(
                        !trimmed.contains("scalar(_stacy_") && !trimmed.contains("scalar _stacy_"),
                        "{}:{}: uses _stacy_ prefix for scalar (should be stacy_): {}",
                        name,
                        line_num + 1,
                        trimmed
                    );
                    // Check for global assignment: global _stacy_...
                    assert!(
                        !trimmed.contains("global _stacy_"),
                        "{}:{}: uses _stacy_ prefix in global assignment: {}",
                        name,
                        line_num + 1,
                        trimmed
                    );
                }
            }
        }
    }
}

#[test]
fn test_generated_ado_no_compound_quotes_in_globals() {
    // Bug class 2: compound quotes (`"..."') in `global` statements.
    // Stata's `global` command does NOT accept compound quote syntax.
    let re = regex::Regex::new(r#"global\s+\S+\s+`""#).unwrap();

    for (name, content) in read_all_generated_ado_files() {
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            assert!(
                !re.is_match(trimmed),
                "{}:{}: global statement uses compound quotes (Stata rejects this): {}",
                name,
                line_num + 1,
                trimmed
            );
        }
    }
}
