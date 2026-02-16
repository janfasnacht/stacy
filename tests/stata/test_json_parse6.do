* test_json_parse6.do - Using = assignment for pattern
clear all
set more off

local project_root "`c(pwd)'"
adopath + "`project_root'/stata/"

di "=== JSON Parsing Debug 6 ===" _n

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

* Build pattern using = assignment (allows string expressions)
local dq = char(34)
local pattern = `dq' + "ready" + `dq' + ":"

di "Pattern via expression: |`pattern'|"

* Alternative: use double quote directly in strpos
* The key is finding "ready": in the JSON
* Let's try searching for just ready": which is unique enough
local pos = strpos(`"`content'"', `"ready":"')
di "Position of ready (with just ready\":\"):  `pos'"

if `pos' > 0 {
    di "Found it!"
    di "Context: " substr(`"`content'"', `pos', 30)

    * Value starts after the colon+space
    local start = `pos' + 8
    local after = substr(`"`content'"', `start', 20)
    local after = strtrim("`after'")
    di "Value area: `after'"
}

di _n "=== Done ==="
