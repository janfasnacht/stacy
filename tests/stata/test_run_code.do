* test_run_code.do - Debug stacy run --code
clear all
set more off

local project_root "`c(pwd)'"
adopath + "`project_root'/stata/"
global stacy_binary "`project_root'/target/release/stacy"

di "=== Testing stacy run --code ===" _n

* Step 1: Verify binary exists
_stacy_find_binary
di "Binary: `r(binary)'"
local binary = r(binary)

* Step 2: Try simple inline code
tempfile jsonout

di "Running: `binary' run --code 'display 1' --json"
shell "`binary'" run --code "display 1" --json > "`jsonout'" 2>&1

di _n "JSON output:"
type "`jsonout'"

* Step 3: Try via wrapper
di _n "Now trying via stacy_run wrapper..."
capture noisily stacy_run, code("display 1")
di "Return code: " _rc

if _rc == 0 {
    return list
}

di _n "=== Done ==="
