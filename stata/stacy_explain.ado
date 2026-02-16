*! stacy_explain.ado - Look up Stata error code details
*! Part of stacy: Modern Stata Workflow Tool
*! Version: 0.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Look up Stata error code details

    Syntax:
        stacy_explain <code> 

    Returns:
        r(code                ) - Error code number (scalar)
        r(category            ) - Error category (local)
        r(description         ) - Full error description (local)
        r(name                ) - Short error name (local)
        r(url                 ) - Link to Stata documentation (local)
*/

program define stacy_explain, rclass
    version 14.0
    syntax anything(name=code)

    * Build command arguments
    local cmd "explain"

    * Validate required argument: code
    if `"`code'"' == "" {
        di as error "stacy_explain: code is required"
        exit 198
    }

    if `"`code'"' != "" {
        local cmd `"`cmd' "`code'""'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar _stacy_json_code
    if _rc == 0 {
        return scalar code = scalar(_stacy_json_code)
    }

    if `"`_stacy_json_category'"' != "" {
        return local category `"`_stacy_json_category'"'
    }

    if `"`_stacy_json_description'"' != "" {
        return local description `"`_stacy_json_description'"'
    }

    if `"`_stacy_json_name'"' != "" {
        return local name `"`_stacy_json_name'"'
    }

    if `"`_stacy_json_url'"' != "" {
        return local url `"`_stacy_json_url'"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
