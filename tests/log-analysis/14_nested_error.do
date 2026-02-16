* Test nested do-files (error in nested file)
clear all
set obs 100
gen x = rnormal()

display "=== Main script: About to call helper with error ==="

* Call nested do-file that will fail
do nested/helper_with_error.do

display "=== Main script: This should not be reached ==="
