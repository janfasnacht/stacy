*! stacy_doctor.ado - Run system diagnostics
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.1
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Run system diagnostics

    Syntax:
        stacy_doctor 

    Returns:
        r(check_count         ) - Total number of checks (scalar)
        r(failed              ) - Number of failed checks (scalar)
        r(passed              ) - Number of checks passed (scalar)
        r(ready               ) - System is ready to use (1=yes, 0=no) (scalar)
        r(warnings            ) - Number of warnings (scalar)
*/

program define stacy_doctor, rclass
    version 14.0
    syntax 

    * Build command arguments
    local cmd "doctor"

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_check_count
    if _rc == 0 {
        return scalar check_count = scalar(stacy_check_count)
    }

    capture confirm scalar stacy_failed
    if _rc == 0 {
        return scalar failed = scalar(stacy_failed)
    }

    capture confirm scalar stacy_passed
    if _rc == 0 {
        return scalar passed = scalar(stacy_passed)
    }

    capture confirm scalar stacy_ready
    if _rc == 0 {
        return scalar ready = scalar(stacy_ready)
    }

    capture confirm scalar stacy_warnings
    if _rc == 0 {
        return scalar warnings = scalar(stacy_warnings)
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
