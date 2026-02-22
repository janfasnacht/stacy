*! stacy_deps.ado - Show dependency tree for Stata scripts
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Show dependency tree for Stata scripts

    Syntax:
        stacy_deps <script> [, options]

    Options:
        Flat                 - Show flat list instead of tree

    Returns:
        r(circular_count      ) - Number of circular dependency paths (scalar)
        r(has_circular        ) - Circular deps found (1=yes, 0=no) (scalar)
        r(has_missing         ) - Missing files found (1=yes, 0=no) (scalar)
        r(missing_count       ) - Number of missing files (scalar)
        r(unique_count        ) - Number of unique dependencies (scalar)
        r(script              ) - Path to analyzed script (local)
*/

program define stacy_deps, rclass
    version 14.0
    syntax anything(name=script) [, Flat]

    * Build command arguments
    local cmd "deps"

    * Validate required argument: script
    if `"`script'"' == "" {
        di as error "stacy_deps: script is required"
        exit 198
    }

    if `"`script'"' != "" {
        local cmd `"`cmd' "`script'""'
    }

    if "`flat'" != "" {
        local cmd `"`cmd' --flat"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_circular_count
    if _rc == 0 {
        return scalar circular_count = scalar(stacy_circular_count)
    }

    capture confirm scalar stacy_has_circular
    if _rc == 0 {
        return scalar has_circular = scalar(stacy_has_circular)
    }

    capture confirm scalar stacy_has_missing
    if _rc == 0 {
        return scalar has_missing = scalar(stacy_has_missing)
    }

    capture confirm scalar stacy_missing_count
    if _rc == 0 {
        return scalar missing_count = scalar(stacy_missing_count)
    }

    capture confirm scalar stacy_unique_count
    if _rc == 0 {
        return scalar unique_count = scalar(stacy_unique_count)
    }

    if `"${stacy_script}"' != "" {
        return local script `"${stacy_script}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
