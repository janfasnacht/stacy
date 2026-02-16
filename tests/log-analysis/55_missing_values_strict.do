* Test: r(416) - Missing values encountered
* Matrix operations do not allow missing values
clear all
set obs 10
gen x = rnormal()
gen y = rnormal()
replace x = . in 1
* mkmat should fail with missing values
mkmat x y, matrix(M)
