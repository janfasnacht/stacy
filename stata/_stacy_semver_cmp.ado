*! _stacy_semver_cmp.ado - Semver comparison helper
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.2.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

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
