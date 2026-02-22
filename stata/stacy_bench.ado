*! stacy_bench.ado - Benchmark script execution
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.1.0
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Benchmark script execution

    Syntax:
        stacy_bench <script> [, options]

    Options:
        NOWarmup             - Skip warmup runs
        Quiet                - Suppress progress output
        Runs(integer)        - Number of measured runs
        Warmup(integer)      - Number of warmup runs

    Returns:
        r(max_secs            ) - Maximum execution time in seconds (scalar)
        r(mean_secs           ) - Mean execution time in seconds (scalar)
        r(measured_runs       ) - Number of measured runs (scalar)
        r(median_secs         ) - Median execution time in seconds (scalar)
        r(min_secs            ) - Minimum execution time in seconds (scalar)
        r(stddev_secs         ) - Standard deviation in seconds (scalar)
        r(success             ) - Whether all runs succeeded (1=yes, 0=no) (scalar)
        r(warmup_runs         ) - Number of warmup runs (scalar)
        r(script              ) - Path to benchmarked script (local)
*/

program define stacy_bench, rclass
    version 14.0
    syntax anything(name=script) [, NOWarmup Quiet Runs(string) Warmup(string)]

    * Build command arguments
    local cmd "bench"

    * Validate required argument: script
    if `"`script'"' == "" {
        di as error "stacy_bench: script is required"
        exit 198
    }

    if `"`script'"' != "" {
        local cmd `"`cmd' "`script'""'
    }

    if "`nowarmup'" != "" {
        local cmd `"`cmd' --no-warmup"'
    }

    if "`quiet'" != "" {
        local cmd `"`cmd' --quiet"'
    }

    if `"`runs'"' != "" {
        local cmd `"`cmd' --runs "`runs'""'
    }

    if `"`warmup'"' != "" {
        local cmd `"`cmd' --warmup "`warmup'""'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_max_secs
    if _rc == 0 {
        return scalar max_secs = scalar(stacy_max_secs)
    }

    capture confirm scalar stacy_mean_secs
    if _rc == 0 {
        return scalar mean_secs = scalar(stacy_mean_secs)
    }

    capture confirm scalar stacy_measured_runs
    if _rc == 0 {
        return scalar measured_runs = scalar(stacy_measured_runs)
    }

    capture confirm scalar stacy_median_secs
    if _rc == 0 {
        return scalar median_secs = scalar(stacy_median_secs)
    }

    capture confirm scalar stacy_min_secs
    if _rc == 0 {
        return scalar min_secs = scalar(stacy_min_secs)
    }

    capture confirm scalar stacy_stddev_secs
    if _rc == 0 {
        return scalar stddev_secs = scalar(stacy_stddev_secs)
    }

    capture confirm scalar stacy_success
    if _rc == 0 {
        return scalar success = scalar(stacy_success)
    }

    capture confirm scalar stacy_warmup_runs
    if _rc == 0 {
        return scalar warmup_runs = scalar(stacy_warmup_runs)
    }

    if `"${stacy_script}"' != "" {
        return local script `"${stacy_script}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
