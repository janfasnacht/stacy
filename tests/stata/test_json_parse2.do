* test_json_parse2.do - More detailed JSON parsing debug
clear all
set more off

local project_root "`c(pwd)'"
adopath + "`project_root'/stata/"

di "=== JSON Parsing Debug 2 ===" _n

* Get JSON from stacy
global stacy_binary "`project_root'/target/release/stacy"
tempfile jsonfile
shell "$stacy_binary" doctor --json > "`jsonfile'" 2>&1

* Read file content (same as _stacy_parse_json)
tempname fh
local content ""
file open `fh' using `"`jsonfile'"', read text
file read `fh' line
while r(eof) == 0 {
    local content `"`content'`line'"'
    file read `fh' line
}
file close `fh'

* Clean up content
local content = subinstr(`"`content'"', char(10), " ", .)
local content = subinstr(`"`content'"', char(13), " ", .)
local content = stritrim(`"`content'"')

* Escape braces
local content = subinstr(`"`content'"', "{", "[[", .)
local content = subinstr(`"`content'"', "}", "]]", .)

di "Content after escaping (first 300 chars):"
di substr(`"`content'"', 1, 300)

di _n "Looking for ready..."

* Try to find "ready" key
local pattern `""ready""'
di "Pattern: `pattern'"

local pos = strpos(`"`content'"', `"`pattern'"')
di "Position of 'ready': `pos'"

* If found, show context
if `pos' > 0 {
    di "Context around ready:"
    di substr(`"`content'"', `pos', 50)
}

* Now test actual extraction
di _n "Calling _stacy_extract_bool..."
_stacy_extract_bool `"`content'"' "ready" _test_ready
di "Result: " scalar(_test_ready)

di _n "=== Done ==="
