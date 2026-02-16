*! stacy_add.ado - Add packages to project
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 0.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Add packages to project

    Syntax:
        stacy_add <packages> [, options]

    Options:
        DEV                  - Add as development dependency
        Source(string)       - Package source: ssc or github:user/repo[@ref]
        TEST                 - Add as test dependency

    Returns:
        r(added               ) - Number of packages added (scalar)
        r(failed              ) - Number of packages that failed (scalar)
        r(skipped             ) - Number of packages skipped (already present) (scalar)
        r(total               ) - Total packages processed (scalar)
        r(group               ) - Dependency group: 'production', 'dev', or 'test' (local)
        r(status              ) - 'success', 'partial', or 'error' (local)
*/

program define stacy_add, rclass
    version 14.0
    syntax anything(name=packages) [, DEV Source(string) TEST]

    * Build command arguments
    local cmd "add"

    * Validate required argument: packages
    if `"`packages'"' == "" {
        di as error "stacy_add: packages is required"
        exit 198
    }

    if `"`packages'"' != "" {
        local cmd `"`cmd' "`packages'""'
    }

    if "`dev'" != "" {
        local cmd `"`cmd' --dev"'
    }

    if `"`source'"' != "" {
        local cmd `"`cmd' --source "`source'""'
    }

    if "`test'" != "" {
        local cmd `"`cmd' --test"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar _stacy_json_added
    if _rc == 0 {
        return scalar added = scalar(_stacy_json_added)
    }

    capture confirm scalar _stacy_json_failed
    if _rc == 0 {
        return scalar failed = scalar(_stacy_json_failed)
    }

    capture confirm scalar _stacy_json_skipped
    if _rc == 0 {
        return scalar skipped = scalar(_stacy_json_skipped)
    }

    capture confirm scalar _stacy_json_total
    if _rc == 0 {
        return scalar total = scalar(_stacy_json_total)
    }

    if `"`_stacy_json_group'"' != "" {
        return local group `"`_stacy_json_group'"'
    }

    if `"`_stacy_json_status'"' != "" {
        return local status `"`_stacy_json_status'"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
