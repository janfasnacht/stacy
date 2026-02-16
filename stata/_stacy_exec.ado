*! _stacy_exec.ado - Execute stacy command and capture Stata-native output
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 0.1.0

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
        scalar _stacy_success = 1
        scalar _stacy_exit_code = 0
        local _stacy_log_file `"/path/to/log.log"'

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

    * Execute command and capture output
    if "`c(os)'" == "Windows" {
        quietly shell `full_cmd' > "`stata_out'" 2>&1
    }
    else {
        quietly shell `full_cmd' > "`stata_out'" 2>&1
    }

    * Check if output file was created
    capture confirm file `"`stata_out'"'
    if _rc != 0 {
        di as error "stacy command failed: no output"
        return scalar exit_code = 5
        exit 198
    }

    * Clear any existing _stacy_* scalars and locals
    _stacy_clear_vars

    * Execute the Stata output directly - no parsing needed!
    * The output file contains valid Stata commands like:
    *   scalar _stacy_success = 1
    *   local _stacy_log_file `"/path/to/file"'
    capture noisily do `"`stata_out'"'
    if _rc != 0 {
        di as error "Failed to execute stacy output"
        return scalar exit_code = 5
        exit _rc
    }

    * Return exit code from the executed output
    capture confirm scalar _stacy_exit_code
    if _rc == 0 {
        return scalar exit_code = scalar(_stacy_exit_code)
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

* Helper: Clear existing _stacy_* variables before executing new output
program define _stacy_clear_vars
    version 14.0

    * Clear scalars
    capture scalar drop _stacy_success
    capture scalar drop _stacy_exit_code
    capture scalar drop _stacy_duration_secs
    capture scalar drop _stacy_error_count
    capture scalar drop _stacy_ready
    capture scalar drop _stacy_passed
    capture scalar drop _stacy_warnings
    capture scalar drop _stacy_failed
    capture scalar drop _stacy_check_count
    capture scalar drop _stacy_has_config
    capture scalar drop _stacy_show_progress
    capture scalar drop _stacy_adopath_count
    capture scalar drop _stacy_installed
    capture scalar drop _stacy_already_installed
    capture scalar drop _stacy_skipped
    capture scalar drop _stacy_total
    capture scalar drop _stacy_package_count
    capture scalar drop _stacy_unique_count
    capture scalar drop _stacy_has_circular
    capture scalar drop _stacy_has_missing
    capture scalar drop _stacy_circular_count
    capture scalar drop _stacy_missing_count
    capture scalar drop _stacy_created_count
end
