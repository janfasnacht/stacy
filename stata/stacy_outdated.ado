*! stacy_outdated.ado - Check for package updates
*! Part of stacy: Modern Stata Workflow Tool
*! Version: 0.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Check for package updates

    Syntax:
        stacy_outdated 

    Returns:
        r(outdated_count      ) - Number of outdated packages (scalar)
        r(total_count         ) - Total packages checked (scalar)
        r(outdated_currents   ) - Comma-separated current versions (local)
        r(outdated_latests    ) - Comma-separated latest versions (local)
        r(outdated_names      ) - Comma-separated outdated package names (local)
        r(status              ) - 'success' or 'error' (local)
*/

program define stacy_outdated, rclass
    version 14.0
    syntax 

    * Build command arguments
    local cmd "outdated"

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar _stacy_json_outdated_count
    if _rc == 0 {
        return scalar outdated_count = scalar(_stacy_json_outdated_count)
    }

    capture confirm scalar _stacy_json_total_count
    if _rc == 0 {
        return scalar total_count = scalar(_stacy_json_total_count)
    }

    if `"`_stacy_json_outdated_currents'"' != "" {
        return local outdated_currents `"`_stacy_json_outdated_currents'"'
    }

    if `"`_stacy_json_outdated_latests'"' != "" {
        return local outdated_latests `"`_stacy_json_outdated_latests'"'
    }

    if `"`_stacy_json_outdated_names'"' != "" {
        return local outdated_names `"`_stacy_json_outdated_names'"'
    }

    if `"`_stacy_json_status'"' != "" {
        return local status `"`_stacy_json_status'"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
