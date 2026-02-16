* Test: r(402) - Negative weights encountered
* fweights must be non-negative integers
clear all
set obs 10
gen x = rnormal()
gen wt = -1
* fweight with negative value should trigger r(402)
summarize x [fweight=wt]
