*! stacy_cache_info.ado - Show cache statistics
*! Part of stacy: Modern Stata Workflow Tool
*! Version: 0.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Show cache statistics

    Syntax:
        stacy_cache_info 

    Returns:
        r(cache_exists        ) - Whether cache file exists (1=yes, 0=no) (scalar)
        r(entry_count         ) - Number of cached entries (scalar)
        r(size_bytes          ) - Approximate size in bytes (scalar)
        r(cache_path          ) - Path to cache file (local)
*/

program define stacy_cache_info, rclass
    version 14.0
    syntax 

    * Build command arguments
    local cmd "cache_info"

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar _stacy_json_cache_exists
    if _rc == 0 {
        return scalar cache_exists = scalar(_stacy_json_cache_exists)
    }

    capture confirm scalar _stacy_json_entry_count
    if _rc == 0 {
        return scalar entry_count = scalar(_stacy_json_entry_count)
    }

    capture confirm scalar _stacy_json_size_bytes
    if _rc == 0 {
        return scalar size_bytes = scalar(_stacy_json_size_bytes)
    }

    if `"`_stacy_json_cache_path'"' != "" {
        return local cache_path `"`_stacy_json_cache_path'"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
