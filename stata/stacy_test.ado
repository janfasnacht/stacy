*! stacy_test.ado - Run tests
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.2
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Run tests

    Syntax:
        stacy_test [test] [, options]

    Options:
        Filter(string)       - Filter tests by pattern
        LIST                 - List tests without running
        PARALLEL             - Run tests in parallel
        Quiet                - Suppress progress output
        Verbose              - Show full log context for failures

    Returns:
        r(duration_secs       ) - Total execution time in seconds (scalar)
        r(failed              ) - Number of failed tests (scalar)
        r(passed              ) - Number of passed tests (scalar)
        r(skipped             ) - Number of skipped tests (scalar)
        r(success             ) - Whether all tests passed (1=yes, 0=no) (scalar)
        r(test_count          ) - Total number of tests (scalar)
        r(test_names          ) - Comma-separated test names (for --list) (local)
*/

program define stacy_test, rclass
    version 14.0
    syntax [anything(name=test)] [, Filter(string) LIST PARALLEL Quiet Verbose]

    * Build command arguments
    local cmd "test"

    if `"`test'"' != "" {
        local cmd `"`cmd' "`test'""'
    }

    if `"`filter'"' != "" {
        local cmd `"`cmd' --filter "`filter'""'
    }

    if "`list'" != "" {
        local cmd `"`cmd' --list"'
    }

    if "`parallel'" != "" {
        local cmd `"`cmd' --parallel"'
    }

    if "`quiet'" != "" {
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

    capture confirm scalar stacy_failed
    if _rc == 0 {
        return scalar failed = scalar(stacy_failed)
    }

    capture confirm scalar stacy_passed
    if _rc == 0 {
        return scalar passed = scalar(stacy_passed)
    }

    capture confirm scalar stacy_skipped
    if _rc == 0 {
        return scalar skipped = scalar(stacy_skipped)
    }

    capture confirm scalar stacy_success
    if _rc == 0 {
        return scalar success = scalar(stacy_success)
    }

    capture confirm scalar stacy_test_count
    if _rc == 0 {
        return scalar test_count = scalar(stacy_test_count)
    }

    if `"${stacy_test_names}"' != "" {
        return local test_names `"${stacy_test_names}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
