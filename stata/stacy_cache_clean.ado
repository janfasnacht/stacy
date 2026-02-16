*! stacy_cache_clean.ado - Remove cached entries
*! Part of stacy: Modern Stata Workflow Tool
*! Version: 0.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Remove cached entries

    Syntax:
        stacy_cache_clean [, options]

    Options:
        OLDERthan(integer)   - Remove entries older than N days
        Quiet                - Suppress output

    Returns:
        r(entries_remaining   ) - Number of entries remaining (scalar)
        r(entries_removed     ) - Number of entries removed (scalar)
        r(status              ) - 'success' or 'error' (local)
*/

program define stacy_cache_clean, rclass
    version 14.0
    syntax [, OLDERthan(integer) Quiet]

    * Build command arguments
    local cmd "cache_clean"

    if `"`older_than'"' != "" {
        local cmd `"`cmd' --older_than "`older_than'""'
    }

    if "`quiet'" != "" {
        local cmd `"`cmd' --quiet"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar _stacy_json_entries_remaining
    if _rc == 0 {
        return scalar entries_remaining = scalar(_stacy_json_entries_remaining)
    }

    capture confirm scalar _stacy_json_entries_removed
    if _rc == 0 {
        return scalar entries_removed = scalar(_stacy_json_entries_removed)
    }

    if `"`_stacy_json_status'"' != "" {
        return local status `"`_stacy_json_status'"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
