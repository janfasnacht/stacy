*! _stacy_check_version.ado - Wrapper/binary version compatibility check
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.3.1
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

program define _stacy_check_version, rclass
    version 14.0
    args binary

    * Cached for the session - skip if already verified.
    if "$stacy_version_checked" == "1" {
        exit 0
    }

    local expected "1.3.1"

    * Capture `<binary> --version` output.
    tempfile ver_out
    capture quietly shell "`binary'" --version > "`ver_out'" 2>&1
    local shell_rc = _rc

    tempname fh
    local raw ""
    capture file open `fh' using "`ver_out'", read text
    if _rc == 0 {
        file read `fh' raw
        file close `fh'
    }

    if `shell_rc' != 0 | `"`raw'"' == "" {
        di as error "stacy: could not determine binary version from `binary' --version"
        di as error "Run {bf:stacy_setup, force} to (re)install the binary."
        exit 198
    }

    * Output is typically `stacy X.Y.Z' - take the second whitespace token.
    local raw_trim = strtrim(`"`raw'"')
    tokenize `"`raw_trim'"'
    local actual `"`2'"'
    if `"`actual'"' == "" {
        local actual `"`1'"'
    }
    * Strip leading 'v' if present (defensive; clap default does not emit it).
    if substr(`"`actual'"', 1, 1) == "v" {
        local actual = substr(`"`actual'"', 2, .)
    }

    if `"`actual'"' == "" {
        di as error "stacy: could not parse binary version from output: `raw'"
        di as error "Run {bf:stacy_setup, force} to (re)install the binary."
        exit 198
    }

    _stacy_semver_cmp `"`actual'"' `"`expected'"'
    local cmp = r(cmp)
    local same_major = r(same_major)

    if `same_major' == 0 {
        di as error "stacy: binary version `actual' is from a different major version than the Stata wrappers (`expected')."
        di as error "Run {bf:stacy_setup, force} to install a matching binary."
        exit 198
    }
    if `cmp' < 0 {
        di as error "stacy: binary version `actual' is older than the Stata wrappers expect (>= `expected', same major)."
        di as error "Run {bf:stacy_setup, force} to install a matching binary."
        exit 198
    }

    * Cache success for the rest of the session.
    global stacy_version_checked "1"
    return local actual `"`actual'"'
    return local expected `"`expected'"'
end
