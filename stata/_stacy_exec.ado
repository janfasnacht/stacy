*! _stacy_exec.ado - Execute stacy command and capture Stata-native output
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.1

/*
    Execute a stacy CLI command and capture Stata-native output.

    Syntax: _stacy_exec command [arguments]

    This is the core execution wrapper. It:
    1. Finds the stacy binary
    2. Executes the command with --format stata flag
    3. Captures output to a temp file
    4. Executes the output file with `do` (no parsing needed!)
    5. Returns exit code in r(exit_code)

    The --format stata output produces Stata-native commands like:
        scalar stacy_success = 1
        scalar stacy_exit_code = 0
        global stacy_log_file "/path/to/log.log"

    These are directly executable - no JSON parsing required.
*/

program define _stacy_exec, rclass
    version 14.0

    * Parse syntax - everything is the command
    local subcmd `"`0'"'

    * Find stacy binary
    _stacy_find_binary
    if r(found) == 0 {
        di as error "stacy binary not found. Run 'stacy_setup' to install."
        exit 601
    }
    local binary `"`r(binary)'"'

    * Create temp file for Stata output
    tempfile stata_out

    * Build command with --format stata
    local full_cmd `""`binary'" `subcmd' --format stata"'

    * Execute command and capture output (stderr to separate file)
    tempfile stata_err
    if "`c(os)'" == "Windows" {
        quietly shell `full_cmd' > "`stata_out'" 2>"`stata_err'"
    }
    else {
        quietly shell `full_cmd' > "`stata_out'" 2>"`stata_err'"
    }

    * Check if output file has content (empty = CLI error, check stderr)
    capture confirm file `"`stata_out'"'
    if _rc != 0 {
        di as error "stacy command failed: no output"
        return scalar exit_code = 5
        exit 198
    }

    * Check if stdout is empty (CLI returned an error on stderr only)
    local filesize = 0
    capture {
        tempname fh_check
        file open `fh_check' using `"`stata_out'"', read text
        file read `fh_check' line
        file close `fh_check'
    }
    if `"`line'"' == "" {
        * stdout empty — show stderr as error message
        capture {
            tempname fh_err
            file open `fh_err' using `"`stata_err'"', read text
            file read `fh_err' errline
            file close `fh_err'
        }
        if `"`errline'"' != "" {
            di as error `"`errline'"'
        }
        else {
            di as error "stacy command failed (no output)"
        }
        return scalar exit_code = 5
        exit 198
    }

    * Clear any existing stacy_* scalars and globals
    _stacy_clear_vars

    * Execute the Stata output directly - no parsing needed!
    * The output file contains valid Stata commands like:
    *   scalar stacy_success = 1
    *   global stacy_log_file "/path/to/file"
    capture noisily do `"`stata_out'"'
    if _rc != 0 {
        di as error "Failed to execute stacy output"
        return scalar exit_code = 5
        exit _rc
    }

    * Return exit code from the executed output
    capture confirm scalar stacy_exit_code
    if _rc == 0 {
        return scalar exit_code = scalar(stacy_exit_code)
    }
    else {
        * Default to success for commands without exit_code (doctor, env, etc.)
        return scalar exit_code = 0
    }

    * Check for stacy update notification (silent on failure)
    capture noisily {
        * Build path to flag file (Stata does not expand ~)
        if "`c(os)'" == "Windows" {
            local cache_base : env LOCALAPPDATA
            if `"`cache_base'"' == "" {
                local cache_base : env USERPROFILE
                local cache_base `"`cache_base'/AppData/Local"'
            }
            local update_flag `"`cache_base'/stacy/cache/update-available"'
        }
        else {
            local cache_base : env XDG_CACHE_HOME
            if `"`cache_base'"' == "" {
                local cache_base : env HOME
                local cache_base `"`cache_base'/.cache"'
            }
            local update_flag `"`cache_base'/stacy/update-available"'
        }
        capture confirm file `"`update_flag'"'
        if _rc == 0 {
            tempname fh
            file open `fh' using `"`update_flag'"', read text
            file read `fh' current_ver
            file read `fh' latest_ver
            file read `fh' upgrade_cmd
            file close `fh'
            if `"`current_ver'"' != "" & `"`latest_ver'"' != "" {
                di as text ""
                di as text "{bf:Update available:} v`current_ver' -> v`latest_ver'"
                di as text "Run {bf:`upgrade_cmd'} to update"
            }
        }
    }
end

* Helper: Clear existing stacy_* variables before executing new output
program define _stacy_clear_vars
    version 14.0

    * Clear all stacy_* global macros (string values from CLI output)
    * Preserve stacy_binary (user-set binary path, not CLI output)
    local save_binary `"$stacy_binary"'
    capture macro drop stacy_*
    if `"`save_binary'"' != "" {
        global stacy_binary `"`save_binary'"'
    }

    * Clear scalars — common across commands
    capture scalar drop stacy_success
    capture scalar drop stacy_exit_code
    capture scalar drop stacy_duration_secs
    capture scalar drop stacy_error_count

    * doctor
    capture scalar drop stacy_ready
    capture scalar drop stacy_passed
    capture scalar drop stacy_warnings
    capture scalar drop stacy_failed
    capture scalar drop stacy_check_count

    * env
    capture scalar drop stacy_has_config
    capture scalar drop stacy_show_progress
    capture scalar drop stacy_adopath_count

    * install
    capture scalar drop stacy_installed
    capture scalar drop stacy_already_installed
    capture scalar drop stacy_skipped
    capture scalar drop stacy_total

    * list / lock / packages
    capture scalar drop stacy_package_count
    capture scalar drop stacy_unique_count
    capture scalar drop stacy_in_sync

    * deps
    capture scalar drop stacy_has_circular
    capture scalar drop stacy_has_missing
    capture scalar drop stacy_circular_count
    capture scalar drop stacy_missing_count
    capture scalar drop stacy_total_count

    * init
    capture scalar drop stacy_created_count

    * bench
    capture scalar drop stacy_measured_runs
    capture scalar drop stacy_warmup_runs
    capture scalar drop stacy_mean_secs
    capture scalar drop stacy_median_secs
    capture scalar drop stacy_min_secs
    capture scalar drop stacy_max_secs
    capture scalar drop stacy_stddev_secs

    * cache
    capture scalar drop stacy_entry_count
    capture scalar drop stacy_size_bytes
    capture scalar drop stacy_cache_exists
    capture scalar drop stacy_entries_removed
    capture scalar drop stacy_entries_remaining

    * task
    capture scalar drop stacy_task_count
    capture scalar drop stacy_script_count
    capture scalar drop stacy_success_count
    capture scalar drop stacy_failed_count

    * test
    capture scalar drop stacy_test_count

    * outdated
    capture scalar drop stacy_outdated_count
    capture scalar drop stacy_total_count
end
