*! stacy_explain.ado - Look up Stata error code details
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.1
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
    capture confirm scalar stacy_code
    if _rc == 0 {
        return scalar code = scalar(stacy_code)
    }

    if `"${stacy_category}"' != "" {
        return local category `"${stacy_category}"'
    }

    if `"${stacy_description}"' != "" {
        return local description `"${stacy_description}"'
    }

    if `"${stacy_name}"' != "" {
        return local name `"${stacy_name}"'
    }

    if `"${stacy_url}"' != "" {
        return local url `"${stacy_url}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
