* test_json_parse4.do - Using compound quotes throughout
clear all
set more off

local project_root "`c(pwd)'"
adopath + "`project_root'/stata/"

di "=== JSON Parsing Debug 4 ===" _n

* Get JSON from stacy
global stacy_binary "`project_root'/target/release/stacy"
tempfile jsonfile
shell "$stacy_binary" doctor --json > "`jsonfile'" 2>&1

* Read file content with COMPOUND quotes
tempname fh
local content ""
file open `fh' using `"`jsonfile'"', read text
file read `fh' line
while r(eof) == 0 {
    local content `"`content'`line'"'
    file read `fh' line
}
file close `fh'

* Clean up with compound quotes
local content = subinstr(`"`content'"', char(10), " ", .)
local content = subinstr(`"`content'"', char(13), " ", .)
local content = stritrim(`"`content'"')

* Escape braces
local content = subinstr(`"`content'"', "{", "[[", .)
local content = subinstr(`"`content'"', "}", "]]", .)

di "Content length: " strlen(`"`content'"')
di "First 200 chars:"
di substr(`"`content'"', 1, 200)

* Build pattern manually
local dq = char(34)
local pattern = `dq' + "ready" + `dq' + ":"

di _n "Pattern: `pattern'"

* Try strpos with compound quotes for content
local pos = strpos(`"`content'"', "`pattern'")
di "Position found: `pos'"

if `pos' > 0 {
    di "Context at pos:"
    di substr(`"`content'"', `pos', 30)

    * Extract after
    local start = `pos' + strlen("`pattern'")
    local after = substr(`"`content'"', `start', 20)
    local after = strtrim("`after'")
    di "After pattern: `after'"

    * Check for true
    if substr("`after'", 1, 4) == "true" {
        di "FOUND: true -> value should be 1"
    }
    else {
        di "NOT true, first 4 chars: '" substr("`after'", 1, 4) "'"
    }
}

di _n "=== Done ==="
