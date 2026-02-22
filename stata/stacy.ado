*! stacy.ado - Reproducible Stata Workflow Tool
*! Version: 1.1.0
*! Author: Jan Fasnacht
*! URL: https://github.com/janfasnacht/stacy
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    stacy - Main entry point for stacy Stata wrapper

    This command dispatches to subcommand-specific programs.
    
    Usage:
        stacy <subcommand> [options]
    
    For help:
        help stacy
*/

program define stacy, rclass
    version 14.0

    * Parse first argument as subcommand
    gettoken subcmd 0 : 0, parse(" ,")

    if "`subcmd'" == "" {
        di as error "stacy: subcommand required"
        di as text ""
        di as text "Available commands:"
        di as text "  stacy add          - Add packages to project"
        di as text "  stacy bench        - Benchmark script execution"
        di as text "  stacy cache_clean  - Remove cached entries"
        di as text "  stacy cache_info   - Show cache statistics"
        di as text "  stacy deps         - Show dependency tree for Stata scripts"
        di as text "  stacy doctor       - Run system diagnostics"
        di as text "  stacy env          - Show environment configuration"
        di as text "  stacy explain      - Look up Stata error code details"
        di as text "  stacy init         - Initialize new stacy project"
        di as text "  stacy install      - Install packages from lockfile or SSC/GitHub"
        di as text "  stacy list         - List installed packages"
        di as text "  stacy lock         - Generate or verify lockfile"
        di as text "  stacy outdated     - Check for package updates"
        di as text "  stacy remove       - Remove packages from project"
        di as text "  stacy run          - Execute a Stata script with error detection"
        di as text "  stacy task         - Run tasks from stacy.toml"
        di as text "  stacy test         - Run tests"
        di as text "  stacy update       - Update packages to latest versions"
        di as text ""
        di as text "For more help: help stacy"
        exit 198
    }

    * Dispatch to appropriate subcommand
    if "`subcmd'" == "add" {
        stacy_add `0'
    }
    else if "`subcmd'" == "bench" {
        stacy_bench `0'
    }
    else if "`subcmd'" == "cache_clean" {
        stacy_cache_clean `0'
    }
    else if "`subcmd'" == "cache_info" {
        stacy_cache_info `0'
    }
    else if "`subcmd'" == "deps" {
        stacy_deps `0'
    }
    else if "`subcmd'" == "doctor" {
        stacy_doctor `0'
    }
    else if "`subcmd'" == "env" {
        stacy_env `0'
    }
    else if "`subcmd'" == "explain" {
        stacy_explain `0'
    }
    else if "`subcmd'" == "init" {
        stacy_init `0'
    }
    else if "`subcmd'" == "install" {
        stacy_install `0'
    }
    else if "`subcmd'" == "list" {
        stacy_list `0'
    }
    else if "`subcmd'" == "lock" {
        stacy_lock `0'
    }
    else if "`subcmd'" == "outdated" {
        stacy_outdated `0'
    }
    else if "`subcmd'" == "remove" {
        stacy_remove `0'
    }
    else if "`subcmd'" == "run" {
        stacy_run `0'
    }
    else if "`subcmd'" == "task" {
        stacy_task `0'
    }
    else if "`subcmd'" == "test" {
        stacy_test `0'
    }
    else if "`subcmd'" == "update" {
        stacy_update `0'
    }
    else if "`subcmd'" == "setup" {
        stacy_setup `0'
    }
    else if "`subcmd'" == "version" | "`subcmd'" == "--version" {
        di as text "stacy Stata wrapper v1.1.0"
    }
    else if "`subcmd'" == "help" | "`subcmd'" == "--help" {
        help stacy
    }
    else {
        di as error "stacy: unknown subcommand '`subcmd'"
        di as text ""
        di as text "Run 'stacy' without arguments to see available commands."
        exit 198
    }

    * Forward return values from subcommand
    return add
end
