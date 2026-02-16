* Test: No variance in variable
clear all
set obs 100
gen x = 1
gen y = rnormal()
* x has no variance
regress y x
