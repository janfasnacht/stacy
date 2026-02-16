* Test: String where number expected
clear all
set obs 10
gen str_var = "not a number"
* Try to use string in numeric context
summarize str_var
