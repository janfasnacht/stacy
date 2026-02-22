*! stacy_update.ado - Update packages to latest versions
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Update packages to latest versions

    Syntax:
        stacy_update [packages] [, options]

    Options:
        DRYrun               - Show what would be updated without making changes

    Returns:
        r(dry_run             ) - Whether this was a dry run (1=yes, 0=no) (scalar)
        r(failed              ) - Number of packages that failed to update (scalar)
        r(total               ) - Total packages checked (scalar)
        r(updated             ) - Number of packages updated (scalar)
        r(updates_available   ) - Number of packages with updates available (scalar)
        r(status              ) - 'success', 'partial', or 'error' (local)
*/

program define stacy_update, rclass
    version 14.0
    syntax [anything(name=packages)] [, DRYrun]

    * Build command arguments
    local cmd "update"

    if `"`packages'"' != "" {
        local cmd `"`cmd' "`packages'""'
    }

    if "`dryrun'" != "" {
        local cmd `"`cmd' --dry-run"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_dry_run
    if _rc == 0 {
        return scalar dry_run = scalar(stacy_dry_run)
    }

    capture confirm scalar stacy_failed
    if _rc == 0 {
        return scalar failed = scalar(stacy_failed)
    }

    capture confirm scalar stacy_total
    if _rc == 0 {
        return scalar total = scalar(stacy_total)
    }

    capture confirm scalar stacy_updated
    if _rc == 0 {
        return scalar updated = scalar(stacy_updated)
    }

    capture confirm scalar stacy_updates_available
    if _rc == 0 {
        return scalar updates_available = scalar(stacy_updates_available)
    }

    if `"${stacy_status}"' != "" {
        return local status `"${stacy_status}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
