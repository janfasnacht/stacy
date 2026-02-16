* Test: Invalid if condition
clear all
set obs 10
gen x = rnormal()
* Invalid if syntax
summarize x if invalid_var
