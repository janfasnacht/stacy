* Test: Ambiguous abbreviation
clear all
set obs 10
gen variable1 = 1
gen variable2 = 2
* 'var' is ambiguous
summarize var
