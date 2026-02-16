*! stacy_lock.ado - Generate or verify lockfile
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.1
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Generate or verify lockfile

    Syntax:
        stacy_lock [, options]

    Options:
        CHECK                - Verify lockfile matches stacy.toml without updating

    Returns:
        r(in_sync             ) - Whether lockfile is in sync (1=yes, 0=no) (scalar)
        r(package_count       ) - Number of packages in lockfile (scalar)
        r(updated             ) - Whether lockfile was updated (1=yes, 0=no) (scalar)
        r(status              ) - 'success', 'updated', or 'error' (local)
*/

program define stacy_lock, rclass
    version 14.0
    syntax [, CHECK]

    * Build command arguments
    local cmd "lock"

    if "`check'" != "" {
        local cmd `"`cmd' --check"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_in_sync
    if _rc == 0 {
        return scalar in_sync = scalar(stacy_in_sync)
    }

    capture confirm scalar stacy_package_count
    if _rc == 0 {
        return scalar package_count = scalar(stacy_package_count)
    }

    capture confirm scalar stacy_updated
    if _rc == 0 {
        return scalar updated = scalar(stacy_updated)
    }

    if `"${stacy_status}"' != "" {
        return local status `"${stacy_status}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
