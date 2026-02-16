* Test: r(430) - Convergence not achieved
* Force probit to fail convergence with low iterations
clear all
set obs 100
gen y = (_n > 50)
gen x = rnormal()
* Force convergence failure with only 1 iteration
probit y x, iterate(1)
