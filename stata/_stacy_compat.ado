*! _stacy_compat.ado - Wrapper/binary version compatibility check
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.2.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Provides:
      _stacy_compat_version  - returns r(version) with the wrapper version
      _stacy_check_version   - validates that the stacy binary version is
                               compatible with these wrappers (>= wrapper
                               version, same major). Caches success in
                               $stacy_version_checked.
      _stacy_semver_cmp      - helper: compares two semver strings, sets
                               r(cmp) in {-1, 0, 1} and r(same_major).

    The version constant is generated from Cargo.toml's package.version, so
    the wrapper expectation always matches the binary built from the same
    source tree. On mismatch, _stacy_check_version errors with a
    `stacy_setup, force` hint and exits 198.
*/

program define _stacy_compat_version, rclass
    version 14.0
    return local version "1.2.0"
end

program define _stacy_semver_cmp, rclass
    version 14.0
    args a b

    * Strip optional pre-release / build suffix after '-' or '+'.
    local a_clean = `"`a'"'
    local dash = strpos(`"`a_clean'"', "-")
    if `dash' > 0 local a_clean = substr(`"`a_clean'"', 1, `dash' - 1)
    local plus = strpos(`"`a_clean'"', "+")
    if `plus' > 0 local a_clean = substr(`"`a_clean'"', 1, `plus' - 1)

    local b_clean = `"`b'"'
    local dash = strpos(`"`b_clean'"', "-")
    if `dash' > 0 local b_clean = substr(`"`b_clean'"', 1, `dash' - 1)
    local plus = strpos(`"`b_clean'"', "+")
    if `plus' > 0 local b_clean = substr(`"`b_clean'"', 1, `plus' - 1)

    * Tokenize on '.': "1.2.3" -> tokens 1, ., 2, ., 3 at positions 1,2,3,4,5.
    tokenize "`a_clean'", parse(".")
    local a_major = real(`"`1'"')
    local a_minor = real(`"`3'"')
    local a_patch = real(`"`5'"')

    tokenize "`b_clean'", parse(".")
    local b_major = real(`"`1'"')
    local b_minor = real(`"`3'"')
    local b_patch = real(`"`5'"')

    * Coerce missing components (e.g. "1.2") to 0.
    if `a_major' >= . local a_major = 0
    if `a_minor' >= . local a_minor = 0
    if `a_patch' >= . local a_patch = 0
    if `b_major' >= . local b_major = 0
    if `b_minor' >= . local b_minor = 0
    if `b_patch' >= . local b_patch = 0

    local cmp = 0
    if `a_major' < `b_major' local cmp = -1
    else if `a_major' > `b_major' local cmp = 1
    else if `a_minor' < `b_minor' local cmp = -1
    else if `a_minor' > `b_minor' local cmp = 1
    else if `a_patch' < `b_patch' local cmp = -1
    else if `a_patch' > `b_patch' local cmp = 1

    return scalar cmp = `cmp'
    return scalar same_major = (`a_major' == `b_major')
end

program define _stacy_check_version, rclass
    version 14.0
    args binary

    * Cached for the session - skip if already verified.
    if "$stacy_version_checked" == "1" {
        exit 0
    }

    local expected "1.2.0"

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
