* Test: Too few variables specified
clear all
set obs 100
gen x = rnormal()
* regress needs at least 2 variables
regress x
