*! stacy_remove.ado - Remove packages from project
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.2
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Remove packages from project

    Syntax:
        stacy_remove <packages> 

    Returns:
        r(not_found           ) - Number of packages not found (scalar)
        r(removed             ) - Number of packages removed (scalar)
        r(total               ) - Total packages processed (scalar)
        r(status              ) - 'success' or 'error' (local)
*/

program define stacy_remove, rclass
    version 14.0
    syntax anything(name=packages)

    * Build command arguments
    local cmd "remove"

    * Validate required argument: packages
    if `"`packages'"' == "" {
        di as error "stacy_remove: packages is required"
        exit 198
    }

    if `"`packages'"' != "" {
        local cmd `"`cmd' "`packages'""'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_not_found
    if _rc == 0 {
        return scalar not_found = scalar(stacy_not_found)
    }

    capture confirm scalar stacy_removed
    if _rc == 0 {
        return scalar removed = scalar(stacy_removed)
    }

    capture confirm scalar stacy_total
    if _rc == 0 {
        return scalar total = scalar(stacy_total)
    }

    if `"${stacy_status}"' != "" {
        return local status `"${stacy_status}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
