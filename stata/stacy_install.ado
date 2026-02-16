*! stacy_install.ado - Install packages from lockfile or SSC/GitHub
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 0.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Install packages from lockfile or SSC/GitHub

    Syntax:
        stacy_install <package> [, options]

    Options:
        From(string)         - Source: ssc or github:user/repo

    Returns:
        r(already_installed   ) - Number already installed (scalar)
        r(installed           ) - Number of newly installed packages (scalar)
        r(package_count       ) - Same as total (scalar)
        r(skipped             ) - Number skipped (errors) (scalar)
        r(total               ) - Total packages processed (scalar)
        r(status              ) - 'success' or 'error' (local)
*/

program define stacy_install, rclass
    version 14.0
    syntax anything(name=package) [, From(string)]

    * Build command arguments
    local cmd "install"

    if `"`package'"' != "" {
        local cmd `"`cmd' "`package'""'
    }

    if `"`from'"' != "" {
        local cmd `"`cmd' --from "`from'""'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar _stacy_json_already_installed
    if _rc == 0 {
        return scalar already_installed = scalar(_stacy_json_already_installed)
    }

    capture confirm scalar _stacy_json_installed
    if _rc == 0 {
        return scalar installed = scalar(_stacy_json_installed)
    }

    capture confirm scalar _stacy_json_package_count
    if _rc == 0 {
        return scalar package_count = scalar(_stacy_json_package_count)
    }

    capture confirm scalar _stacy_json_skipped
    if _rc == 0 {
        return scalar skipped = scalar(_stacy_json_skipped)
    }

    capture confirm scalar _stacy_json_total
    if _rc == 0 {
        return scalar total = scalar(_stacy_json_total)
    }

    if `"`_stacy_json_status'"' != "" {
        return local status `"`_stacy_json_status'"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
