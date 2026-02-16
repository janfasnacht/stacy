* Test: r(103) - Too many variables specified
* tabulate has a limit on number of variables
clear all
set obs 10
gen x1 = 1
gen x2 = 2
gen x3 = 3
* tabulate only accepts 1 or 2 variables, not 3
tabulate x1 x2 x3
