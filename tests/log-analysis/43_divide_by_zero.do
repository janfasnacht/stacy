* Test: Division by zero (may produce missing, not error)
clear all
set obs 10
gen x = 1
gen y = 0
gen z = x / y
* Force error context
assert z != .
