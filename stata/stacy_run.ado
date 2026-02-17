*! stacy_run.ado - Execute a Stata script with error detection
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.2
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Execute a Stata script with error detection

    Syntax:
        stacy_run [script] [, options]

    Options:
        Code(string)         - Inline Stata code
        Directory(string)    - Run Stata in this directory
        Profile              - Include execution metrics
        Quietly              - Suppress output
        Verbose              - Extra output

    Returns:
        r(duration_secs       ) - Execution time in seconds (scalar)
        r(error_count         ) - Number of errors detected (scalar)
        r(exit_code           ) - Exit code (0=success) (scalar)
        r(success             ) - Whether script succeeded (1=yes, 0=no) (scalar)
        r(log_file            ) - Path to log file (local)
        r(script              ) - Path to script (local)
        r(source              ) - 'file' or 'inline' (local)
*/

program define stacy_run, rclass
    version 14.0
    syntax [anything(name=script)] [, Code(string) Directory(string) Profile Quietly Verbose]

    * Build command arguments
    local cmd "run"

    if `"`script'"' != "" {
        local cmd `"`cmd' "`script'""'
    }

    if `"`code'"' != "" {
        local cmd `"`cmd' --code "`code'""'
    }

    if `"`directory'"' != "" {
        local cmd `"`cmd' --directory "`directory'""'
    }

    if "`profile'" != "" {
        local cmd `"`cmd' --profile"'
    }

    if "`quietly'" != "" {
        local cmd `"`cmd' --quiet"'
    }

    if "`verbose'" != "" {
        local cmd `"`cmd' --verbose"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_duration_secs
    if _rc == 0 {
        return scalar duration_secs = scalar(stacy_duration_secs)
    }

    capture confirm scalar stacy_error_count
    if _rc == 0 {
        return scalar error_count = scalar(stacy_error_count)
    }

    capture confirm scalar stacy_exit_code
    if _rc == 0 {
        return scalar exit_code = scalar(stacy_exit_code)
    }

    capture confirm scalar stacy_success
    if _rc == 0 {
        return scalar success = scalar(stacy_success)
    }

    if `"${stacy_log_file}"' != "" {
        return local log_file `"${stacy_log_file}"'
    }

    if `"${stacy_script}"' != "" {
        return local script `"${stacy_script}"'
    }

    if `"${stacy_source}"' != "" {
        return local source `"${stacy_source}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
