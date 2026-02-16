* Test: Assert failure
clear all
set obs 10
gen x = rnormal()
* This will fail
assert x > 0
