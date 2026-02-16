* Test: Invalid format specification
clear all
set obs 10
gen x = 1
format x %invalid
