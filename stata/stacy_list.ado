*! stacy_list.ado - List installed packages
*! Part of stacy: Modern Stata Workflow Tool
*! Version: 0.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    List installed packages

    Syntax:
        stacy_list [, options]

    Options:
        Tree                 - Group packages by dependency type

    Returns:
        r(package_count       ) - Number of packages (scalar)
        r(package_groups      ) - Comma-separated package groups (local)
        r(package_names       ) - Comma-separated package names (local)
        r(package_sources     ) - Comma-separated package sources (local)
        r(package_versions    ) - Comma-separated package versions (local)
        r(status              ) - 'success' or 'error' (local)
*/

program define stacy_list, rclass
    version 14.0
    syntax [, Tree]

    * Build command arguments
    local cmd "list"

    if "`tree'" != "" {
        local cmd `"`cmd' --tree"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar _stacy_json_package_count
    if _rc == 0 {
        return scalar package_count = scalar(_stacy_json_package_count)
    }

    if `"`_stacy_json_package_groups'"' != "" {
        return local package_groups `"`_stacy_json_package_groups'"'
    }

    if `"`_stacy_json_package_names'"' != "" {
        return local package_names `"`_stacy_json_package_names'"'
    }

    if `"`_stacy_json_package_sources'"' != "" {
        return local package_sources `"`_stacy_json_package_sources'"'
    }

    if `"`_stacy_json_package_versions'"' != "" {
        return local package_versions `"`_stacy_json_package_versions'"'
    }

    if `"`_stacy_json_status'"' != "" {
        return local status `"`_stacy_json_status'"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
