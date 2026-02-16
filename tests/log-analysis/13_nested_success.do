* Test nested do-files (success case)
clear all
set obs 100
gen x = rnormal()

display "=== Main script: About to call helper ==="

* Call nested do-file
do nested/helper.do

display "=== Main script: Back from helper ==="
summarize x y
