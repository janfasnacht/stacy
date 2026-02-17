*! stacy_init.ado - Initialize new stacy project
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.2
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Initialize new stacy project

    Syntax:
        stacy_init [path] [, options]

    Options:
        Force                - Overwrite existing files

    Returns:
        r(created_count       ) - Number of files/directories created (scalar)
        r(package_count       ) - Number of packages specified (scalar)
        r(path                ) - Path where project was created (local)
        r(status              ) - 'success' or 'error' (local)
*/

program define stacy_init, rclass
    version 14.0
    syntax [anything(name=path)] [, Force]

    * Build command arguments
    local cmd "init"

    if `"`path'"' != "" {
        local cmd `"`cmd' "`path'""'
    }

    if "`force'" != "" {
        local cmd `"`cmd' --force"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_created_count
    if _rc == 0 {
        return scalar created_count = scalar(stacy_created_count)
    }

    capture confirm scalar stacy_package_count
    if _rc == 0 {
        return scalar package_count = scalar(stacy_package_count)
    }

    if `"${stacy_path}"' != "" {
        return local path `"${stacy_path}"'
    }

    if `"${stacy_status}"' != "" {
        return local status `"${stacy_status}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
