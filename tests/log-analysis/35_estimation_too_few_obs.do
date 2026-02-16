* Test: Too few observations
clear all
set obs 2
gen x = rnormal()
gen y = rnormal()
* Need more obs for regression
regress y x
