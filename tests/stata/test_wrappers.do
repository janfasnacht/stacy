* test_wrappers.do - Test stacy Stata wrappers
* Run this in Stata to verify wrappers work correctly
*
* This test can be run in two ways:
* 1. From the fixture harness (cargo test): Uses local ado/ directory
* 2. Manually from repo root: Uses stata/ directory
*
* The STACY_BINARY environment variable should be set to the stacy binary path.

clear all
set more off

* Determine adopath based on context
* If we're in a fixture project (ado/ exists), use that
* Otherwise, try the stata/ directory relative to repo root
capture confirm file "ado/stacy.ado"
if _rc == 0 {
    * Running from fixture - use local ado/ (highest priority)
    adopath ++ "`c(pwd)'/ado/"
    di as text "Using local ado/ directory"
}
else {
    * Try stata/ relative to current directory (running from repo root)
    capture confirm file "stata/stacy.ado"
    if _rc == 0 {
        adopath ++ "`c(pwd)'/stata/"
        di as text "Using stata/ directory"
    }
    else {
        * Last resort: check parent of tests/stata/ directory
        adopath ++ "`c(pwd)'/stata/"
        di as text "Using hardcoded stata/ path"
    }
}

* STACY_BINARY is set via environment variable by the test harness
* or defaults to checking common locations
* Note: _stacy_find_binary looks for $stacy_binary (lowercase)
local stacy_bin : env STACY_BINARY
if "`stacy_bin'" != "" {
    global stacy_binary "`stacy_bin'"
    di as text "stacy_binary from env: $stacy_binary"
}
else {
    * Try to find it
    capture confirm file "target/release/stacy"
    if _rc == 0 {
        global stacy_binary "`c(pwd)'/target/release/stacy"
    }
    else {
        capture confirm file "target/debug/stacy"
        if _rc == 0 {
            global stacy_binary "`c(pwd)'/target/debug/stacy"
        }
        else {
            * Fallback
            global stacy_binary "`c(pwd)'/target/release/stacy"
        }
    }
    di as text "stacy_binary auto-detected: $stacy_binary"
}

di _n "{hline 60}"
di "Testing stacy Stata Wrappers"
di "{hline 60}" _n

* =============================================================================
* Test 1: stacy doctor
* =============================================================================
di as text "TEST 1: stacy doctor"
di as text "{hline 40}"

capture noisily stacy doctor
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy doctor executed successfully"
    return list

    * Check expected returns exist
    capture confirm scalar r(ready)
    if _rc == 0 {
        di as result "  [PASS] r(ready) = " r(ready)
    }
    else {
        di as error "  [FAIL] r(ready) not found"
    }

    capture confirm scalar r(passed)
    if _rc == 0 {
        di as result "  [PASS] r(passed) = " r(passed)
    }
}
else {
    di as error "  [FAIL] stacy doctor failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 2: stacy env
* =============================================================================
di as text "TEST 2: stacy env"
di as text "{hline 40}"

capture noisily stacy env
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy env executed successfully"
    return list

    * Check expected returns
    if "`r(stata_binary)'" != "" {
        di as result "  [PASS] r(stata_binary) = `r(stata_binary)'"
    }
    if "`r(project_root)'" != "" {
        di as result "  [PASS] r(project_root) = `r(project_root)'"
    }
}
else {
    di as error "  [FAIL] stacy env failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 3: stacy run with inline code
* =============================================================================
di as text "TEST 3: stacy run --code"
di as text "{hline 40}"

capture noisily stacy run, code("display 42")
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy run --code executed successfully"
    return list

    capture confirm scalar r(success)
    if _rc == 0 {
        di as result "  [PASS] r(success) = " r(success)
    }
    capture confirm scalar r(exit_code)
    if _rc == 0 {
        di as result "  [PASS] r(exit_code) = " r(exit_code)
    }
    capture confirm scalar r(duration_secs)
    if _rc == 0 {
        di as result "  [PASS] r(duration_secs) = " r(duration_secs)
    }
    if "`r(source)'" != "" {
        di as result "  [PASS] r(source) = `r(source)'"
    }
}
else {
    di as error "  [FAIL] stacy run failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 4: Error handling - run nonexistent file
* =============================================================================
di as text "TEST 4: Error handling (nonexistent file)"
di as text "{hline 40}"

capture noisily stacy run "this_file_does_not_exist.do"
local rc = _rc

if `rc' != 0 {
    di as result "  [PASS] Correctly returned error for missing file (rc = `rc')"
}
else {
    di as error "  [FAIL] Should have returned error for missing file"
}

di _n

* =============================================================================
* Test 5: stacy list
* =============================================================================
di as text "TEST 5: stacy list"
di as text "{hline 40}"

capture noisily stacy list
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy list executed successfully"
    return list

    capture confirm scalar r(package_count)
    if _rc == 0 {
        di as result "  [PASS] r(package_count) = " r(package_count)
    }
}
else {
    di as error "  [FAIL] stacy list failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 6: stacy outdated
* =============================================================================
di as text "TEST 6: stacy outdated"
di as text "{hline 40}"

capture noisily stacy outdated
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy outdated executed successfully"
    return list

    capture confirm scalar r(outdated_count)
    if _rc == 0 {
        di as result "  [PASS] r(outdated_count) = " r(outdated_count)
    }
    capture confirm scalar r(total_count)
    if _rc == 0 {
        di as result "  [PASS] r(total_count) = " r(total_count)
    }
}
else {
    di as error "  [FAIL] stacy outdated failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 7: stacy lock
* =============================================================================
di as text "TEST 7: stacy lock"
di as text "{hline 40}"

capture noisily stacy lock
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy lock executed successfully"
    return list

    capture confirm scalar r(package_count)
    if _rc == 0 {
        di as result "  [PASS] r(package_count) = " r(package_count)
    }
    capture confirm scalar r(in_sync)
    if _rc == 0 {
        di as result "  [PASS] r(in_sync) = " r(in_sync)
    }
}
else {
    di as error "  [FAIL] stacy lock failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 8: stacy cache_info
* =============================================================================
di as text "TEST 8: stacy cache_info"
di as text "{hline 40}"

capture noisily stacy cache_info
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy cache info executed successfully"
    return list

    capture confirm scalar r(entry_count)
    if _rc == 0 {
        di as result "  [PASS] r(entry_count) = " r(entry_count)
    }
    capture confirm scalar r(cache_exists)
    if _rc == 0 {
        di as result "  [PASS] r(cache_exists) = " r(cache_exists)
    }
}
else {
    di as error "  [FAIL] stacy cache info failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 9: stacy task --list
* =============================================================================
di as text "TEST 9: stacy task --list"
di as text "{hline 40}"

capture noisily stacy task, list
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy task --list executed successfully"
    return list

    capture confirm scalar r(task_count)
    if _rc == 0 {
        di as result "  [PASS] r(task_count) = " r(task_count)
    }
}
else {
    di as error "  [FAIL] stacy task --list failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 10: stacy test --list
* =============================================================================
di as text "TEST 10: stacy test --list"
di as text "{hline 40}"

capture noisily stacy test, list
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy test --list executed successfully"
    return list

    capture confirm scalar r(test_count)
    if _rc == 0 {
        di as result "  [PASS] r(test_count) = " r(test_count)
    }
}
else {
    di as error "  [FAIL] stacy test --list failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 11: Dispatcher - unknown subcommand
* =============================================================================
di as text "TEST 11: Unknown subcommand"
di as text "{hline 40}"

capture noisily stacy foobar_nonexistent
local rc = _rc

if `rc' == 198 {
    di as result "  [PASS] Unknown command returns error 198"
}
else {
    di as error "  [FAIL] Expected rc=198, got rc=`rc'"
}

di _n

* =============================================================================
* Test 12: Dispatcher - version command
* =============================================================================
di as text "TEST 12: stacy version"
di as text "{hline 40}"

capture noisily stacy version
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy version executed successfully"
}
else {
    di as error "  [FAIL] stacy version failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 13: stacy run with script file
* =============================================================================
di as text "TEST 13: stacy run with script file"
di as text "{hline 40}"

capture noisily stacy run "scripts/hello.do"
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy run with file executed successfully"
    return list

    capture confirm scalar r(success)
    if _rc == 0 & r(success) == 1 {
        di as result "  [PASS] r(success) = 1"
    }

    if "`r(source)'" == "file" {
        di as result "  [PASS] r(source) = file"
    }
}
else {
    di as error "  [FAIL] stacy run with file failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 14: stacy deps
* =============================================================================
di as text "TEST 14: stacy deps"
di as text "{hline 40}"

capture noisily stacy deps "scripts/hello.do"
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy deps executed successfully"
    return list

    capture confirm scalar r(unique_count)
    if _rc == 0 {
        di as result "  [PASS] r(unique_count) = " r(unique_count)
    }
}
else {
    di as error "  [FAIL] stacy deps failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 15: stacy bench
* =============================================================================
di as text "TEST 15: stacy bench"
di as text "{hline 40}"

capture noisily stacy bench "scripts/hello.do", runs(2) nowarmup quiet
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy bench executed successfully"
    return list

    capture confirm scalar r(mean_secs)
    if _rc == 0 {
        di as result "  [PASS] r(mean_secs) = " r(mean_secs)
    }
    capture confirm scalar r(measured_runs)
    if _rc == 0 {
        di as result "  [PASS] r(measured_runs) = " r(measured_runs)
    }
}
else {
    di as error "  [FAIL] stacy bench failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 16: Dispatcher - no subcommand (should show help)
* =============================================================================
di as text "TEST 16: stacy (no subcommand)"
di as text "{hline 40}"

capture noisily stacy
local rc = _rc

if `rc' == 198 {
    di as result "  [PASS] No subcommand returns error 198"
}
else {
    di as error "  [FAIL] Expected rc=198, got rc=`rc'"
}

di _n

* =============================================================================
* Test 17: Dispatcher - help command
* =============================================================================
di as text "TEST 17: stacy help"
di as text "{hline 40}"

capture noisily stacy help
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy help executed successfully"
}
else {
    di as error "  [FAIL] stacy help failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 18: stacy init (in temp directory)
* =============================================================================
di as text "TEST 18: stacy init"
di as text "{hline 40}"

* Create a temp directory for init test
local tempdir "`c(pwd)'/temp_init_test"
capture mkdir "`tempdir'"

capture noisily stacy init "`tempdir'", force
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy init executed successfully"
    return list

    capture confirm scalar r(created_count)
    if _rc == 0 {
        di as result "  [PASS] r(created_count) = " r(created_count)
    }

    if "`r(status)'" == "success" {
        di as result "  [PASS] r(status) = success"
    }
}
else {
    di as error "  [FAIL] stacy init failed with rc = `rc'"
}

* Clean up temp directory
capture shell rm -rf "`tempdir'"

di _n

* =============================================================================
* Test 19: stacy init --force (overwrite existing)
* =============================================================================
di as text "TEST 19: stacy init --force"
di as text "{hline 40}"

* Create temp directory and init twice with force
local tempdir "`c(pwd)'/temp_init_force_test"
capture mkdir "`tempdir'"

* First init
capture noisily stacy init "`tempdir'"
local rc1 = _rc

* Second init with force (should succeed)
capture noisily stacy init "`tempdir'", force
local rc2 = _rc

if `rc2' == 0 {
    di as result "  [PASS] stacy init --force succeeded on existing project"
}
else {
    di as error "  [FAIL] stacy init --force failed with rc = `rc2'"
}

* Clean up temp directory
capture shell rm -rf "`tempdir'"

di _n

* =============================================================================
* Test 20: stacy task (run actual task)
* =============================================================================
di as text "TEST 20: stacy task test-task"
di as text "{hline 40}"

capture noisily stacy task "test-task"
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy task executed successfully"
    return list

    capture confirm scalar r(success)
    if _rc == 0 & r(success) == 1 {
        di as result "  [PASS] r(success) = 1"
    }

    capture confirm scalar r(script_count)
    if _rc == 0 {
        di as result "  [PASS] r(script_count) = " r(script_count)
    }

    if "`r(task_name)'" == "test-task" {
        di as result "  [PASS] r(task_name) = test-task"
    }
}
else {
    di as error "  [FAIL] stacy task failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 21: stacy cache_clean
* =============================================================================
di as text "TEST 21: stacy cache_clean"
di as text "{hline 40}"

capture noisily stacy cache_clean
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy cache clean executed successfully"
    return list

    capture confirm scalar r(entries_removed)
    if _rc == 0 {
        di as result "  [PASS] r(entries_removed) = " r(entries_removed)
    }

    capture confirm scalar r(entries_remaining)
    if _rc == 0 {
        di as result "  [PASS] r(entries_remaining) = " r(entries_remaining)
    }
}
else {
    di as error "  [FAIL] stacy cache clean failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 22: stacy run with failing script
* =============================================================================
di as text "TEST 22: stacy run (failing script)"
di as text "{hline 40}"

capture noisily stacy run "scripts/fail.do"
local rc = _rc

* The command should return an error (script fails with error 99)
if `rc' != 0 {
    di as result "  [PASS] stacy run correctly returned error for failing script (rc = `rc')"
    return list

    capture confirm scalar r(success)
    if _rc == 0 & r(success) == 0 {
        di as result "  [PASS] r(success) = 0 (correctly indicates failure)"
    }

    capture confirm scalar r(exit_code)
    if _rc == 0 & r(exit_code) != 0 {
        di as result "  [PASS] r(exit_code) = " r(exit_code) " (non-zero)"
    }
}
else {
    di as error "  [FAIL] stacy run should have returned error for failing script"
}

di _n

* =============================================================================
* Test 23: stacy deps with includes
* =============================================================================
di as text "TEST 23: stacy deps (script with includes)"
di as text "{hline 40}"

capture noisily stacy deps "scripts/with_deps.do"
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy deps executed successfully"
    return list

    capture confirm scalar r(unique_count)
    if _rc == 0 {
        di as result "  [PASS] r(unique_count) = " r(unique_count)
        * Should have at least 1 dependency (hello.do)
        if r(unique_count) >= 1 {
            di as result "  [PASS] Found expected dependencies"
        }
    }

    capture confirm scalar r(total_count)
    if _rc == 0 {
        di as result "  [PASS] r(total_count) = " r(total_count)
    }
}
else {
    di as error "  [FAIL] stacy deps failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 24: stacy lock --check
* =============================================================================
di as text "TEST 24: stacy lock --check"
di as text "{hline 40}"

capture noisily stacy lock, check
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy lock --check executed successfully"
    return list

    capture confirm scalar r(in_sync)
    if _rc == 0 {
        di as result "  [PASS] r(in_sync) = " r(in_sync)
    }

    capture confirm scalar r(package_count)
    if _rc == 0 {
        di as result "  [PASS] r(package_count) = " r(package_count)
    }
}
else {
    di as error "  [FAIL] stacy lock --check failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 25: Return value type validation
* =============================================================================
di as text "TEST 25: Return value type validation"
di as text "{hline 40}"

* Run doctor to get various return values
capture noisily stacy doctor
local rc = _rc
local pass_count = 0
local test_count = 0

if `rc' == 0 {
    * Test that scalars are numeric
    local test_count = `test_count' + 1
    capture confirm scalar r(ready)
    if _rc == 0 {
        * Check it's a valid number (0 or 1)
        if r(ready) == 0 | r(ready) == 1 {
            local pass_count = `pass_count' + 1
            di as result "  [PASS] r(ready) is boolean (0 or 1)"
        }
        else {
            di as error "  [FAIL] r(ready) should be 0 or 1, got " r(ready)
        }
    }

    * Test that passed is numeric
    local test_count = `test_count' + 1
    capture confirm scalar r(passed)
    if _rc == 0 {
        if r(passed) >= 0 {
            local pass_count = `pass_count' + 1
            di as result "  [PASS] r(passed) is non-negative integer"
        }
    }

    * Test that failed is numeric
    local test_count = `test_count' + 1
    capture confirm scalar r(failed)
    if _rc == 0 {
        if r(failed) >= 0 {
            local pass_count = `pass_count' + 1
            di as result "  [PASS] r(failed) is non-negative integer"
        }
    }

    * Test that locals are strings
    local test_count = `test_count' + 1
    if "`r(stacy_version)'" != "" {
        local pass_count = `pass_count' + 1
        di as result "  [PASS] r(stacy_version) is non-empty string: `r(stacy_version)'"
    }

    local test_count = `test_count' + 1
    if "`r(stata_binary)'" != "" {
        local pass_count = `pass_count' + 1
        di as result "  [PASS] r(stata_binary) is non-empty string"
    }

    di as result "  Passed `pass_count'/`test_count' type validation checks"
}
else {
    di as error "  [FAIL] stacy doctor failed with rc = `rc'"
}

di _n

* =============================================================================
* Test 26: Update notification flag file (read and display)
* =============================================================================
di as text "TEST 26: Update notification flag file"
di as text "{hline 40}"

* Build the flag file path the same way _stacy_exec does
local cache_base : env XDG_CACHE_HOME
if `"`cache_base'"' == "" {
    local cache_base : env HOME
    local cache_base `"`cache_base'/.cache"'
}
local flag_dir `"`cache_base'/stacy"'
local update_flag `"`flag_dir'/update-available"'

* Create a test flag file
capture mkdir `"`flag_dir'"'
tempname fh
file open `fh' using `"`update_flag'"', write text replace
file write `fh' "0.1.0" _n
file write `fh' "0.2.0" _n
file write `fh' "brew upgrade stacy" _n
file close `fh'

* Verify _stacy_exec reads and displays it by running a command
* The update notification appears after the command output
capture noisily stacy doctor
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy doctor succeeded with flag file present"
}
else {
    di as error "  [FAIL] stacy doctor failed with flag file present (rc = `rc')"
}

* Verify the flag file can be read back correctly (same logic as _stacy_exec)
capture {
    tempname fh2
    file open `fh2' using `"`update_flag'"', read text
    file read `fh2' current_ver
    file read `fh2' latest_ver
    file read `fh2' upgrade_cmd
    file close `fh2'
}

if `"`current_ver'"' == "0.1.0" {
    di as result "  [PASS] Flag file line 1 (current_ver) = `current_ver'"
}
else {
    di as error "  [FAIL] Flag file line 1 expected '0.1.0', got '`current_ver''"
}

if `"`latest_ver'"' == "0.2.0" {
    di as result "  [PASS] Flag file line 2 (latest_ver) = `latest_ver'"
}
else {
    di as error "  [FAIL] Flag file line 2 expected '0.2.0', got '`latest_ver''"
}

if `"`upgrade_cmd'"' == "brew upgrade stacy" {
    di as result "  [PASS] Flag file line 3 (upgrade_cmd) = `upgrade_cmd'"
}
else {
    di as error "  [FAIL] Flag file line 3 expected 'brew upgrade stacy', got '`upgrade_cmd''"
}

* Clean up - remove the test flag file
capture erase `"`update_flag'"'

di _n

* =============================================================================
* Test 27: Update notification absent when no flag file
* =============================================================================
di as text "TEST 27: No update notification without flag file"
di as text "{hline 40}"

* Make sure flag file is gone
capture erase `"`update_flag'"'

* Run a command - should work fine without the flag file
capture noisily stacy doctor
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy doctor succeeded without flag file"
}
else {
    di as error "  [FAIL] stacy doctor failed without flag file (rc = `rc')"
}

di _n

* =============================================================================
* Test 28: Update notification with empty flag file (graceful handling)
* =============================================================================
di as text "TEST 28: Empty flag file (graceful handling)"
di as text "{hline 40}"

* Create an empty flag file
tempname fh3
file open `fh3' using `"`update_flag'"', write text replace
file close `fh3'

* Should not crash - the capture noisily block handles it
capture noisily stacy doctor
local rc = _rc

if `rc' == 0 {
    di as result "  [PASS] stacy doctor succeeded with empty flag file"
}
else {
    di as error "  [FAIL] stacy doctor failed with empty flag file (rc = `rc')"
}

* Clean up
capture erase `"`update_flag'"'

di _n

* =============================================================================
* Summary
* =============================================================================
di "{hline 60}"
di "Test complete. Check results above."
di "{hline 60}"
