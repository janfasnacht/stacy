* Test: r(102) - Too few variables specified
* pwcorr with obs option requires at least 2 variables
clear all
set obs 100
gen x = rnormal()
* This should fail - pwcorr needs at least 2 variables
pwcorr x, obs
