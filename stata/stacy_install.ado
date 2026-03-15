*! stacy_install.ado - Install packages from lockfile or SSC/GitHub
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Install packages from lockfile or SSC/GitHub

    Syntax:
        stacy_install [, options]

    Options:
        FROZEN               - Fail if lockfile doesn't match stacy.toml
        NOVerify             - Skip checksum verification
        With(string)         - Include dependency groups (comma-separated: dev, test)

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
    syntax [, FROZEN NOVerify With(string)]

    * Build command arguments
    local cmd "install"

    if "`frozen'" != "" {
        local cmd `"`cmd' --frozen"'
    }

    if "`noverify'" != "" {
        local cmd `"`cmd' --no-verify"'
    }

    if `"`with'"' != "" {
        local cmd `"`cmd' --with "`with'""'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_already_installed
    if _rc == 0 {
        return scalar already_installed = scalar(stacy_already_installed)
    }

    capture confirm scalar stacy_installed
    if _rc == 0 {
        return scalar installed = scalar(stacy_installed)
    }

    capture confirm scalar stacy_package_count
    if _rc == 0 {
        return scalar package_count = scalar(stacy_package_count)
    }

    capture confirm scalar stacy_skipped
    if _rc == 0 {
        return scalar skipped = scalar(stacy_skipped)
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
