* test_json_parse5.do - Fixed string concatenation
clear all
set more off

local project_root "`c(pwd)'"
adopath + "`project_root'/stata/"

di "=== JSON Parsing Debug 5 ===" _n

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

* Build pattern correctly - Stata concatenates by juxtaposition, not +
local dq = char(34)
local pattern `"`dq'ready`dq':"'

di "Pattern: `pattern'"
di "Pattern length: " strlen("`pattern'")

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
        di "SUCCESS: Found true!"
    }
    else {
        di "Value: '" substr("`after'", 1, 10) "'"
    }
}
else {
    di "Pattern not found!"
    di "Looking for 'ready' manually..."
    local pos2 = strpos(`"`content'"', "ready")
    di "Position of 'ready': `pos2'"
    if `pos2' > 0 {
        di "Context: " substr(`"`content'"', `pos2' - 5, 30)
    }
}

di _n "=== Done ==="
