* Test: Error propagation through multiple nested files
* Similar to 14 but different error
clear all
set obs 10
gen x = rnormal()
* Call nested file that will error
do tests/log-analysis/nested/helper_with_error.do
