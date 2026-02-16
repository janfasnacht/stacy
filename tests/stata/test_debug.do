* test_debug.do - Debug stacy Stata wrappers
clear all
set more off

* Set the CORRECT global (lowercase)
local project_root "`c(pwd)'"
global stacy_binary "`project_root'/target/release/stacy"

* Add stata/ to adopath
adopath + "`project_root'/stata/"

di _n "=== Debugging stacy Wrapper ===" _n

* Test 1: Check if binary is found
di "Step 1: Finding binary..."
_stacy_find_binary
di "  Found: " r(found)
di "  Binary: `r(binary)'"

if r(found) == 0 {
    di as error "Binary not found! Check path."
    exit 601
}

* Test 2: Run doctor directly and check output
di _n "Step 2: Testing stacy doctor JSON output..."

* Create temp file for output
tempfile jsonout
di "  Temp file: `jsonout'"

* Run command
local binary = r(binary)
di "  Running: `binary' doctor --json"

shell "`binary'" doctor --json > "`jsonout'" 2>&1

* Check if file exists and show content
di "  Checking output file..."
type "`jsonout'"

* Test 3: Try parsing
di _n "Step 3: Testing JSON parsing..."
_stacy_parse_json "`jsonout'" doctor

* Show what was extracted
di "  Extracted values:"
capture confirm scalar _stacy_json_ready
if _rc == 0 {
    di "    _stacy_json_ready = " scalar(_stacy_json_ready)
}
else {
    di "    _stacy_json_ready not found"
}

capture confirm scalar _stacy_json_passed
if _rc == 0 {
    di "    _stacy_json_passed = " scalar(_stacy_json_passed)
}
else {
    di "    _stacy_json_passed not found"
}

di _n "=== Debug Complete ==="
