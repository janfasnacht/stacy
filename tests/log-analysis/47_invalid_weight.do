* Test: Invalid weight specification
clear all
set obs 10
gen x = rnormal()
gen y = rnormal()
gen wt = -1
* Negative weights
summarize x [pw=wt]
