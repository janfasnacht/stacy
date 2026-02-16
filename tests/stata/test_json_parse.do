* test_json_parse.do - Debug JSON parsing specifically
clear all
set more off

local project_root "`c(pwd)'"
adopath + "`project_root'/stata/"

di "=== JSON Parsing Debug ===" _n

* Test with hardcoded JSON
local json1 `"{"ready": true, "passed": 3}"'
di "Test 1: Simple JSON"
di "  Input: `json1'"

_stacy_extract_bool `"`json1'"' "ready" _test_ready
di "  Extracted ready: " scalar(_test_ready)

_stacy_extract_number `"`json1'"' "passed" _test_passed
di "  Extracted passed: " scalar(_test_passed)

* Test with nested JSON (like doctor output)
local json2 `"{"checks": [], "ready": true, "summary": {"failed": 0, "passed": 3, "warnings": 2}}"'
di _n "Test 2: Nested JSON"
di "  Input: `json2'"

_stacy_extract_bool `"`json2'"' "ready" _test_ready2
di "  Extracted ready: " scalar(_test_ready2)

_stacy_extract_number `"`json2'"' "passed" _test_passed2
di "  Extracted passed: " scalar(_test_passed2)

_stacy_extract_number `"`json2'"' "failed" _test_failed2
di "  Extracted failed: " scalar(_test_failed2)

* Test reading from actual file
di _n "Test 3: From actual stacy output file"

global stacy_binary "`project_root'/target/release/stacy"
tempfile jsonfile
shell "$stacy_binary" doctor --json > "`jsonfile'" 2>&1

* Read file content
tempname fh
local content ""
file open `fh' using `"`jsonfile'"', read text
file read `fh' line
while r(eof) == 0 {
    local content `"`content'`line'"'
    file read `fh' line
}
file close `fh'

* Show first 200 chars
di "  Content length: " strlen(`"`content'"')
di "  First 200 chars: " substr(`"`content'"', 1, 200)

* Try to find "ready"
local pos = strpos(`"`content'"', `""ready""')
di "  Position of 'ready': `pos'"

* Test extraction
_stacy_extract_bool `"`content'"' "ready" _test_ready3
di "  Extracted ready: " scalar(_test_ready3)

di _n "=== Done ==="
