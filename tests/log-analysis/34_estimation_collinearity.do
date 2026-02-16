* Test: Collinearity
clear all
set obs 100
gen x = rnormal()
gen x2 = x
gen y = rnormal()
* x and x2 are perfectly collinear
regress y x x2
